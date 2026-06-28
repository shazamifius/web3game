//! LE RENDEZ-VOUS : le seul morceau qui n'est PAS du pair-à-pair.
//!
//! Ce petit serveur ne fait qu'une chose : présenter les joueurs entre eux. Quand
//! un client dit « HELLO », le serveur retient son adresse (qu'il LIT dans le
//! paquet reçu — pas besoin que le client la connaisse), lui attribue un
//! identifiant, et lui renvoie la liste de tous les autres. Ensuite, les clients
//! s'envoient leur état DIRECTEMENT, sans repasser par lui.
//!
//! Lancement :  cargo run -- rendezvous

use super::aoi::{dist2, keep_nearest, within_radius, MAX_NEIGHBORS};
use super::control::{decode_hello, encode_welcome};
use super::crypto::{PeerId, pow_bits};
use super::message::decode_relay_fwd;
use super::skin::random_hue;
use super::transport::Socket;
use super::wire::{kind, KIND_RELAY_FWD, RENDEZVOUS_PORT};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Plafond du nombre de clients que le rendez-vous garde en mémoire (chap. 9.5a, D21). Sa table
/// est indexée par ADRESSE SOURCE (lue dans le paquet), or une source UDP est usurpable → sans
/// borne, un flood de HELLO depuis des adresses bidon ferait croître la table jusqu'à épuiser la
/// RAM (le rendez-vous est notre SEUL point central → le protéger compte, D21). Large devant une
/// vraie instance ; l'éviction des silencieux (5 s) libère en continu.
/// *Résidu ASSUMÉ (D21, après T1.2) : le rate-limit débit ci-dessous coupe l'amplification d'UNE
/// source, mais un flood depuis BEAUCOUP de sources usurpées distinctes peut encore saturer la table
/// (chaque adresse a son propre seau) — borné par ce plafond + l'éviction 5 s, pas supprimé. La vraie
/// parade restante = ROUTABILITÉ (handshake prouvant que la source peut RECEVOIR à son adresse), plus
/// lourde car elle change le flux HELLO côté client → laissée à une étape supervisée (anti-spoofing complet).*
const MAX_CLIENTS: usize = 8192;

/// RATE-LIMIT DÉBIT par source (chap. 12.3 / D21) : un HELLO nous coûte un WELCOME en retour
/// (amplification + CPU). Sans borne, une source peut nous faire répondre à volonté. On met donc un
/// seau à jetons PAR adresse source : `HELLO_RATE` HELLO/s tolérés en régime, `HELLO_BURST` en pointe.
/// Un client HONNÊTE émet 1 HELLO/s (`HELLO_PERIOD`) → jamais throttlé ; un spammeur, lui, se voit
/// ignoré dès qu'il dépasse (on ne lui répond plus → fin de l'amplification depuis cette source).
const HELLO_RATE: f32 = 4.0;
const HELLO_BURST: f32 = 8.0;

/// Admet-on ce HELLO dans la table du rendez-vous (chap. 9.5a) ? Un client DÉJÀ connu est toujours
/// admis (on rafraîchit). Un NOUVEAU n'est admis que s'il reste de la place sous le plafond. Pur →
/// testable sans lancer la boucle réseau.
fn should_admit(is_known: bool, current_len: usize, cap: usize) -> bool {
    is_known || current_len < cap
}

/// Politique de rate-limit débit (D21), PURE/testable : depuis le crédit courant d'une source, rend
/// `(répond-on ?, crédit restant)`. On répond (et on dépense 1 jeton) s'il reste au moins 1 jeton ;
/// sinon on ignore SANS dépenser (la source a épuisé son budget de réponses pour l'instant).
fn rate_limit_hello(credit: f32) -> (bool, f32) {
    if credit >= 1.0 {
        (true, credit - 1.0)
    } else {
        (false, credit)
    }
}

/// RELAIS NAT (chap. 12.3 / D17), repli derrière DRAPEAU. Budget par source du relais : un repli
/// honnête émet ~20 paquets/s (cadence de jeu) → on tolère `RELAY_RATE` en régime, `RELAY_CAP` en
/// pointe. Au-delà, on cesse de relayer cette source (anti-amplification : le rendez-vous ne se
/// laisse pas transformer en réflecteur illimité). Valeurs jumelles de celles du relais pair (6.5).
const RELAY_RATE: f32 = 30.0;
const RELAY_CAP: f32 = 60.0;

/// BANC UNIQUEMENT — perte injectée sur la recopie relais (`RELAY_DROP_PCT`, 0..100, défaut **0** =
/// chemin intact). Modélise un lien lossy (4G/CGNAT : ~88 % mesuré) pour PROUVER en headless que la
/// redondance d'émission ([bot::relay_redundancy_from_env]) écrase la traîne p95 — sans dépendre d'un
/// vrai mobile. En prod le drapeau est absent → 0 → aucune perte ajoutée. Pseudo-aléatoire mais
/// DÉTERMINISTE (xorshift, graine fixe) → le chiffre est reproductible (règle du projet).
fn relay_drop_pct() -> f64 {
    relay_drop_pct_of(std::env::var("RELAY_DROP_PCT").ok().as_deref())
}

/// Politique PURE (testable) : parse + borne à [0, 100] ; absent ou invalide → 0 (chemin intact).
fn relay_drop_pct_of(v: Option<&str>) -> f64 {
    v.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0).clamp(0.0, 100.0)
}

/// xorshift64 — PRNG déterministe sans dépendance (le projet est dep-free). Avance la graine et rend
/// le nouvel état. Sert UNIQUEMENT au tirage de perte du banc (jamais sur le chemin de prod).
fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Le rendez-vous est-il en MODE RELAIS ? Éteint par DÉFAUT (`RENDEZVOUS_RELAY` non défini) → il
/// reste un présentateur PUR, « tuable », qui ne route AUCUN état (comportement historique, chemin
/// par défaut byte-pour-byte intact). Allumé (`RENDEZVOUS_RELAY=1`) → il relaie les paires qui ne
/// peuvent pas se percer. *Casse la propriété « rendez-vous jetable » → assumé en v1 car c'est un
/// REPLI (cf. PAPIER 12.3) ; à décentraliser en v2.*
fn relay_enabled() -> bool {
    relay_flag_on(std::env::var("RENDEZVOUS_RELAY").ok().as_deref())
}

/// Politique du drapeau (PURE, testable sans toucher l'environnement) : seul `1`/`true` allume le
/// relais ; absent ou toute autre valeur = ÉTEINT (présentateur pur). Défaut sûr = OFF.
fn relay_flag_on(v: Option<&str>) -> bool {
    matches!(v, Some("1") | Some("true"))
}

/// DÉCISION DE RELAIS (pure, testable) : à partir d'une enveloppe `KIND_RELAY_FWD` reçue et de la
/// table des clients, rend `(adresse de B, payload scellé à recopier)` — ou `None` si l'enveloppe est
/// malformée, ou si le destinataire n'est pas un client connu. 12.3-G : on NE vérifie PLUS le sceau
/// ici (le payload peut être un état OU une orbe, sceaux différents) → c'est le DESTINATAIRE qui
/// vérifie (états et orbes s'auto-vérifient à la réception). L'anti-amplification NE repose PAS sur ce
/// sceau mais sur fanout 1 + rate-limit (appelant) + dest inscrit (ci-dessous) → réflecteur 1:1 borné
/// entre deux clients connus, jamais un service ouvert. Le rendez-vous ne fait que PORTER des octets.
fn relay_decision<'a>(
    buf: &'a [u8],
    clients: &HashMap<SocketAddr, ClientInfo>,
) -> Option<(SocketAddr, &'a [u8])> {
    let (dest, payload) = decode_relay_fwd(buf)?;
    // RECONNEXION (bug trouvé le 22 juin) : à la reconnexion, un même id a 2 entrées (ancienne adresse
    // pas encore évincée + nouvelle) → on doit router vers la PLUS RÉCENTE (`seen` max), sinon on relaie
    // vers l'ancienne adresse MORTE et le pair ne reçoit rien pendant ~5 s.
    let addr = clients
        .iter()
        .filter(|(_, info)| info.id == dest)
        .max_by_key(|(_, info)| info.seen)
        .map(|(a, _)| *a)?;
    Some((addr, payload))
}

/// Ce que le rendez-vous retient d'un client : son id, sa dernière activité, sa
/// position (pour l'AoI), et son dernier nombre de voisins (pour ne logger qu'au
/// changement).
struct ClientInfo {
    id: PeerId, // identité (clé publique) du client : redistribuée aux autres
    seen: Instant,
    pos: (f32, f32),
    last_count: usize,
}

pub fn run_rendezvous() {
    let socket = match Socket::bind(RENDEZVOUS_PORT) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le rendez-vous sur {RENDEZVOUS_PORT} : {e}");
            return;
        }
    };
    // La couleur de salle de CETTE session de serveur : tous les joueurs connectés
    // l'adopteront. Deux fenêtres de couleur différente = pas le même serveur.
    let world_hue = random_hue();
    println!(
        "Rendez-vous : écoute sur 127.0.0.1:{RENDEZVOUS_PORT} (couleur de salle : teinte {world_hue}°). En attente de joueurs…"
    );
    // 12.3 : mode relais (repli NAT), lu UNE fois. Éteint par défaut → présentateur pur (chemin intact).
    let relay = relay_enabled();
    if relay {
        println!("⚠ Mode RELAIS ACTIVÉ (RENDEZVOUS_RELAY) : je route les états des paires qui ne percent pas. Plus « jetable » → repli v1 (cf. PAPIER 12.3).");
    }
    // BANC : perte injectée sur la recopie relais (0 = chemin intact). Déterministe (graine fixe).
    let drop_pct = relay_drop_pct();
    let mut drop_seed: u64 = 0x9E3779B97F4A7C15;
    if drop_pct > 0.0 {
        println!("🧪 BANC : perte injectée {drop_pct:.0}% sur la recopie relais (déterministe). Hors banc, ce drapeau est absent.");
    }

    let mut clients: HashMap<SocketAddr, ClientInfo> = HashMap::new();
    // D21 : seaux de rate-limit débit par source (adresse). Rechargés au temps écoulé à chaque tour.
    let mut hello_credit: HashMap<SocketAddr, f32> = HashMap::new();
    // 12.3 : budget de relais par source (séparé du HELLO car cadence de jeu ~20 Hz, pas 1 Hz).
    let mut relay_credit: HashMap<SocketAddr, f32> = HashMap::new();
    // 12.3 (diagnostic) : paires (src→dest) déjà relayées au moins une fois → on logue la 1re recopie
    // de chaque sens (preuve OBSERVABLE du relais, neutre, sans dépendre des fenêtres 3D).
    let mut relayed_pairs: HashSet<(PeerId, PeerId)> = HashSet::new();
    let mut last_tick = Instant::now();

    loop {
        // Recharge des seaux débit (D21) par le temps réellement écoulé depuis le tour précédent ;
        // on draine les sources inactives (seau plein) si la table enfle (anti-saturation mémoire).
        let dt = last_tick.elapsed().as_secs_f32();
        last_tick = Instant::now();
        for credit in hello_credit.values_mut() {
            *credit = (*credit + dt * HELLO_RATE).min(HELLO_BURST);
        }
        if hello_credit.len() > MAX_CLIENTS {
            hello_credit.retain(|_, c| *c < HELLO_BURST);
        }
        // 12.3 : même recharge/drainage pour le budget de relais (anti-saturation mémoire).
        if relay {
            for credit in relay_credit.values_mut() {
                *credit = (*credit + dt * RELAY_RATE).min(RELAY_CAP);
            }
            if relay_credit.len() > MAX_CLIENTS {
                relay_credit.retain(|_, c| *c < RELAY_CAP);
            }
        }

        for (from, bytes) in socket.poll() {
            // 12.3 — REPLI RELAIS (derrière le drapeau). Une enveloppe KIND_RELAY_FWD est routée vers
            // son destinataire (un client connu), à débit borné, UNIQUEMENT si l'émetteur est lui aussi
            // un client inscrit (réflecteur 1:1 entre deux pairs connus, pas un service ouvert). 12.3-G :
            // le payload peut être un état OU une orbe → le sceau est vérifié par le DESTINATAIRE, plus
            // ici (anti-amplification = fanout 1 + rate-limit + dest inscrit). Hors mode relais : ignoré.
            if relay && kind(&bytes) == Some(KIND_RELAY_FWD) {
                if clients.contains_key(&from) {
                    let c = relay_credit.entry(from).or_insert(RELAY_CAP);
                    if *c >= 1.0 {
                        if let Some((dest_addr, payload)) = relay_decision(&bytes, &clients) {
                            *c -= 1.0; // le relais a FAIT son travail : le crédit est consommé même si
                                       // le réseau (banc) perd ensuite le paquet — la perte est en AVAL.
                            let dropped = drop_pct > 0.0
                                && (xorshift64(&mut drop_seed) % 10_000) as f64 / 100.0 < drop_pct;
                            if !dropped {
                                let _ = socket.send_to(dest_addr, payload);
                                // Diagnostic : 1re recopie EFFECTIVE de CE sens → on l'annonce une fois.
                                if let (Some(src), Some(dst)) =
                                    (clients.get(&from).map(|i| i.id), clients.get(&dest_addr).map(|i| i.id))
                                {
                                    if relayed_pairs.insert((src, dst)) {
                                        println!("🔀 RELAIS établi : {} → {} (première recopie)", src.short(), dst.short());
                                    }
                                }
                            }
                        }
                    }
                }
                continue; // une enveloppe de relais n'est jamais un HELLO
            }
            // HELLO porte la position du joueur (pour l'AoI) ET son identité (clé
            // publique). Depuis le chap. 6.1, le rendez-vous n'ATTRIBUE plus de numéro :
            // l'identité, c'est la clé. Il ne fait que présenter les joueurs entre eux.
            let Some((px, pz, id)) = decode_hello(&bytes) else {
                continue; // le rendez-vous ne comprend que HELLO
            };
            // 6.2 : une identité sans preuve de travail n'est même pas listée.
            if !id.has_pow(pow_bits()) {
                continue;
            }
            // D21 : rate-limit débit par source. Une source ne peut pas nous faire répondre à
            // volonté (anti-amplification + anti-CPU). Honnête (1 HELLO/s) → jamais throttlé.
            let credit = hello_credit.entry(from).or_insert(HELLO_BURST);
            let (repond, reste) = rate_limit_hello(*credit);
            *credit = reste;
            if !repond {
                continue; // budget de réponses épuisé pour cette source → on l'ignore ce tour
            }
            // 9.5a (D21) : borne MÉMOIRE. Un client déjà connu est rafraîchi ; un nouveau n'entre
            // que s'il reste de la place sous le plafond → un flood de sources usurpées ne peut
            // plus faire enfler la table sans fin (au pire il sature, l'éviction 5 s la draine).
            if !should_admit(clients.contains_key(&from), clients.len(), MAX_CLIENTS) {
                continue;
            }
            let pos = (px, pz);
            let now = Instant::now();
            // Nouveau venu (adresse jamais vue) ? On le signale une fois.
            let last_count = match clients.get(&from) {
                Some(info) => info.last_count,
                None => {
                    println!("Joueur {} rejoint ({from}).", id.short());
                    usize::MAX // force le log au premier roster
                }
            };

            // VOISINAGE BORNÉ (chap. 6.6) : pré-filtre grossier par rayon, puis on ne
            // garde que les MAX_NEIGHBORS pairs les PLUS PROCHES. C'est la borne
            // d'échelle : le WELCOME ne déborde plus (trou n°2) et chacun ne parle qu'à
            // ~K voisins → O(N·K) au lieu d'O(N²) (trou n°3). Le water-filling côté
            // client répartit ensuite le débit entre ces voisins.
            let cands: Vec<((PeerId, SocketAddr), f32)> = clients
                .iter()
                .filter(|(addr, info)| **addr != from && within_radius(info.pos, pos))
                .map(|(addr, info)| ((info.id, *addr), dist2(info.pos, pos)))
                .collect();
            let roster: Vec<(PeerId, SocketAddr)> = keep_nearest(cands, MAX_NEIGHBORS);

            if roster.len() != last_count {
                println!("Joueur {} : {} a portee.", id.short(), roster.len());
            }
            clients.insert(from, ClientInfo { id, seen: now, pos, last_count: roster.len() });
            let _ = socket.send_to(from, &encode_welcome(world_hue, &roster));
        }

        // On oublie les clients silencieux depuis plus de CLIENT_TTL (déconnectés). 28 juin (D17) :
        // passé de 5 s à 20 s. Sur un lien LOINTAIN/lossy (ami CGNAT réel), perdre 5 s de HELLO
        // d'affilée est facile → le client était évincé puis ré-admis en boucle (« churn » observé :
        // même id rejoint 16-25× depuis la MÊME adresse) → le relais s'effondrait sans arrêt, donc
        // aucun flux descendant stable, donc rien à mesurer. 20 s tolère la perte transitoire sans
        // garder éternellement un vrai disparu (borné par MAX_CLIENTS de toute façon).
        let now = Instant::now();
        clients.retain(|addr, info| {
            let keep = now.duration_since(info.seen) < Duration::from_secs(20);
            if !keep {
                println!("Joueur {} parti ({addr}).", info.id.short());
                // 12.3 (diag) : on oublie ses paires de relais → s'il se reconnecte, le ré-établissement
                // sera de nouveau LOGGÉ (« 🔀 RELAIS établi ») → on VOIT les reconnexions, plus de devinette.
                let gone = info.id;
                relayed_pairs.retain(|(s, d)| *s != gone && *d != gone);
            }
            keep
        });

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9.5a — la politique d'admission borne la table : un NOUVEAU venu est refusé quand c'est
    /// plein (anti-DoS mémoire), mais un client DÉJÀ connu est toujours rafraîchi (il ne perd pas
    /// sa place parce que la table est pleine).
    #[test]
    fn admission_borne_la_table_mais_garde_les_connus() {
        // De la place : tout le monde entre.
        assert!(should_admit(false, 10, 8192)); // nouveau, place libre
        assert!(should_admit(true, 10, 8192)); // connu
        // Pleine : le nouveau est refusé, le connu reste admis.
        assert!(!should_admit(false, 8192, 8192)); // nouveau + plein → refusé
        assert!(should_admit(true, 8192, 8192)); // connu + plein → rafraîchi quand même
        assert!(!should_admit(false, 9000, 8192)); // au-delà du plafond aussi
    }

    /// D21 — le rate-limit débit : on répond tant qu'il reste un jeton (et on le dépense) ; à sec,
    /// on ignore SANS dépenser. Une rafale d'une même source finit donc ignorée (anti-amplification),
    /// pendant qu'un client honnête (1 HELLO/s, seau qui se recharge à 4/s) garde toujours du crédit.
    #[test]
    fn rate_limit_hello_coupe_la_rafale_pas_l_honnete() {
        // Seau plein : on répond et on décompte.
        let (ok, reste) = rate_limit_hello(HELLO_BURST);
        assert!(ok);
        assert_eq!(reste, HELLO_BURST - 1.0);
        // Pile un jeton : dernier répondu, tombe à zéro.
        assert_eq!(rate_limit_hello(1.0), (true, 0.0));
        // À sec : on ignore, et on ne descend pas sous zéro (rien dépensé).
        assert_eq!(rate_limit_hello(0.5), (false, 0.5));
        assert_eq!(rate_limit_hello(0.0), (false, 0.0));
        // Une rafale qui épuise le seau finit ignorée (8 réponses puis plus rien jusqu'à recharge).
        let mut c = HELLO_BURST;
        let mut repondus = 0;
        for _ in 0..100 {
            let (r, reste) = rate_limit_hello(c);
            c = reste;
            if r {
                repondus += 1;
            }
        }
        assert_eq!(repondus, HELLO_BURST as i32); // exactement la pointe, pas 100
    }

    // --- 12.3 : REPLI RELAIS (routage côté rendez-vous) -------------------------

    use super::super::crypto::Identity;
    use super::super::message::{encode_relay_fwd, encode_signed, sig_ok, PlayerState, SIGNED_STATE_SIZE};

    fn client(id: PeerId, addr: &str) -> (SocketAddr, ClientInfo) {
        let info = ClientInfo { id, seen: Instant::now(), pos: (0.0, 0.0), last_count: 0 };
        (addr.parse().unwrap(), info)
    }

    /// Fabrique l'état SCELLÉ d'un joueur (forme KIND_STATE de 182 o) sous son identité.
    fn sealed_state(idy: &Identity) -> [u8; SIGNED_STATE_SIZE] {
        let p = PlayerState {
            id: idy.id(), x: 1.0, y: 0.0, z: -2.0, vx: 0.0, vy: 0.0, vz: 0.0,
            yaw: 0.0, pitch: 0.0, r: 0.5, g: 0.5, b: 0.5, parent: None, seq: 1,
        };
        encode_signed(&p, idy)
    }

    /// Le mode relais est ÉTEINT par défaut (drapeau absent) et ne s'allume QUE sur `1`/`true` → le
    /// chemin par défaut reste celui d'avant (aucun routage d'état). Testé purement, sans toucher l'env.
    #[test]
    fn relais_eteint_par_defaut() {
        assert!(!relay_flag_on(None)); // drapeau absent → OFF (présentateur pur)
        assert!(!relay_flag_on(Some("0")));
        assert!(!relay_flag_on(Some("yes"))); // toute valeur inattendue → OFF (défaut sûr)
        assert!(relay_flag_on(Some("1")));
        assert!(relay_flag_on(Some("true")));
    }

    /// BANC : la perte injectée est ÉTEINTE par défaut (0 = chemin intact) et BORNÉE à [0, 100] ;
    /// une valeur absente/invalide retombe sur 0 (jamais de perte fantôme en prod).
    #[test]
    fn perte_injectee_bornee_et_eteinte_par_defaut() {
        assert_eq!(relay_drop_pct_of(None), 0.0); // absent → chemin intact
        assert_eq!(relay_drop_pct_of(Some("paf")), 0.0); // invalide → 0
        assert_eq!(relay_drop_pct_of(Some("88")), 88.0);
        assert_eq!(relay_drop_pct_of(Some("150")), 100.0); // borné haut
        assert_eq!(relay_drop_pct_of(Some("-5")), 0.0); // borné bas
    }

    /// LE cas nominal : A (qui ne perce pas B) envoie une enveloppe RELAY_FWD(dest=B) ; le rendez-vous
    /// la route vers l'ADRESSE de B, en recopiant l'état scellé de A VERBATIM (le sceau tient).
    #[test]
    fn relais_route_vers_le_bon_destinataire_verbatim() {
        let a = Identity::generate();
        let b = Identity::generate();
        let mut clients = HashMap::new();
        let (b_addr, b_info) = client(b.id(), "203.0.113.7:5000");
        clients.insert(b_addr, b_info);
        let (other_addr, other_info) = client(Identity::generate().id(), "198.51.100.2:6000");
        clients.insert(other_addr, other_info);

        let sealed = sealed_state(&a);
        let env = encode_relay_fwd(b.id(), &sealed);
        let (addr, payload) = relay_decision(&env, &clients).expect("doit router");
        assert_eq!(addr, b_addr); // vers B, pas l'autre
        assert_eq!(payload, &sealed[..]); // l'état de A est recopié à l'octet près
        assert!(sig_ok(payload)); // et son sceau tient → le rendez-vous ne forge rien
    }

    /// Destinataire INCONNU (pas un client) → on ne route pas (pas de réflecteur vers l'extérieur).
    #[test]
    fn relais_refuse_destinataire_inconnu() {
        let a = Identity::generate();
        let inconnu = Identity::generate();
        let clients: HashMap<SocketAddr, ClientInfo> = HashMap::new(); // personne
        let env = encode_relay_fwd(inconnu.id(), &sealed_state(&a));
        assert!(relay_decision(&env, &clients).is_none());
    }

    /// 12.3-G — DÉPLACEMENT DE LA VÉRIF DE SCEAU. Le rendez-vous ne vérifie PLUS le sceau (le payload
    /// peut être un état OU une orbe) : il ROUTE même un payload au sceau cassé vers un dest inscrit,
    /// et c'est le DESTINATAIRE qui le jette (états et orbes s'auto-vérifient à la réception). L'anti-
    /// amplification tient toujours : fanout 1 + rate-limit (appelant) + dest inscrit (cf. test voisin
    /// `relais_refuse_destinataire_inconnu` qui, lui, est la vraie barrière anti-réflecteur).
    #[test]
    fn relais_route_meme_payload_non_verifie_le_dest_tranche() {
        let a = Identity::generate();
        let b = Identity::generate();
        let mut clients = HashMap::new();
        let (b_addr, b_info) = client(b.id(), "203.0.113.7:5000");
        clients.insert(b_addr, b_info);

        let mut sealed = sealed_state(&a);
        sealed[40] ^= 0xFF; // on casse le corps signé
        assert!(!sig_ok(&sealed)); // le sceau est bien invalide…
        let env = encode_relay_fwd(b.id(), &sealed);
        // …mais le rendez-vous route quand même vers B (dest inscrit) : la vérif a migré chez le dest.
        let (addr, payload) = relay_decision(&env, &clients).expect("12.3-G : route sans vérifier");
        assert_eq!(addr, b_addr);
        assert_eq!(payload, &sealed[..]); // porté verbatim ; B le rejettera (sig_ok faux à la réception)
    }
}
