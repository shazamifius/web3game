//! SONDE DE LIEN — partie 1 : le TYPE de NAT (cône PERÇABLE vs symétrique/CGNAT).
//!
//! # Pourquoi (Phase 2 du PLAN_TEST_RESEAU, doute D36)
//! La Phase 1 a tranché : la redondance relais ne sert QUE sur de la perte *aléatoire avec
//! marge*, jamais sur un lien saturé. Pour décider ça machine par machine, chaque agent doit
//! d'abord CONNAÎTRE la nature de son lien. Le premier trait — le plus structurant — c'est :
//! mon NAT est-il **perçable** (cône) ou non (**symétrique / CGNAT**, relais obligatoire) ?
//!
//! # La méthode (RFC 5389, dep-free)
//! On envoie une « binding request » STUN à DEUX serveurs publics d'IP DIFFÉRENTE, depuis la
//! MÊME prise UDP, et on lit l'adresse PUBLIQUE qu'ils nous renvoient (« reflexive address ») :
//!   - même port public vu par les deux → le NAT garde un mapping STABLE quel que soit le
//!     destinataire (« endpoint-independent ») = **cône** → perçable par hole-punching ;
//!   - port public DIFFÉRENT → le NAT refait un mapping par destinataire (« endpoint-dependent »)
//!     = **symétrique** (typique du CGNAT 4G) → le perçage direct échoue → **relais obligatoire**.
//! Un NAT « ouvert » (aucune traduction) se comporte comme un cône (perçable trivialement) → on
//! le replie honnêtement dans `Cone`. Aucune réponse (UDP bloqué, hors-ligne) → `Unknown`.
//!
//! # Découpage testable
//! L'encodage de la requête, le décodage de l'adresse réfléchie et la CLASSIFICATION sont des
//! fonctions PURES (zéro réseau) → couvertes par des tests déterministes. Seul `probe_nat` /
//! `run_natcheck` touchent le réseau (un aller-retour au démarrage, hors boucle chaude).

use super::link::rendezvous_addr;
use super::wire::{KIND_ECHO, PROTO_VERSION};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

/// Le « magic cookie » STUN (RFC 5389) : constante fixe en tête de chaque message (octets 4..8).
/// Sert AUSSI de masque XOR pour l'adresse réfléchie (attribut XOR-MAPPED-ADDRESS).
const MAGIC_COOKIE: u32 = 0x2112_A442;

/// Type de message STUN : « binding request » — ce qu'on ÉMET vers le serveur.
const MSG_BINDING_REQUEST: u16 = 0x0001;
/// Type de message STUN : « binding success response » — ce qu'on ATTEND en retour.
const MSG_BINDING_SUCCESS: u16 = 0x0101;

/// Attribut moderne (RFC 5389) : adresse réfléchie MASQUÉE (XOR avec le magic cookie). Préféré.
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
/// Attribut historique (RFC 3489) : adresse réfléchie NUE. Repli si pas de XOR-MAPPED-ADDRESS.
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;

/// Famille d'adresse IPv4 dans un attribut d'adresse STUN.
const FAMILY_IPV4: u8 = 0x01;

/// Serveurs STUN publics et bien connus, de FOURNISSEURS DIFFÉRENTS (donc d'IP différentes) →
/// indispensable pour distinguer cône de symétrique (un symétrique remappe par IP de destination).
/// On les essaie dans l'ordre jusqu'à obtenir DEUX réponses depuis deux IP distinctes. Ce ne sont
/// pas des secrets, juste des adresses publiques — l'agent ne porte toujours QUE des adresses, jamais de clé.
const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "stun.nextcloud.com:443",
    "stun.stunprotocol.org:3478",
];

/// Le verdict de NAT. Volontairement à TROIS états honnêtes : on sait dire perçable, non-perçable,
/// ou « je n'ai pas pu trancher ». (« Open NAT » = perçable trivialement → replié dans `Cone`.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NatType {
    /// Mapping endpoint-indépendant (même port public vers 2 serveurs) → PERÇABLE par hole-punching.
    Cone,
    /// Mapping endpoint-dépendant (le port public change) → CGNAT/symétrique → RELAIS obligatoire.
    Symmetric,
    /// Pas de réponse STUN (UDP bloqué, hors-ligne, ou une seule observation) → indéterminé.
    Unknown,
}

/// Étiquette courte et stable pour le heartbeat / l'affichage (`nat:"cone"` etc.).
pub(crate) fn nat_str(t: NatType) -> &'static str {
    match t {
        NatType::Cone => "cone",
        NatType::Symmetric => "sym",
        NatType::Unknown => "?",
    }
}

/// Fabrique une « binding request » STUN : en-tête de 20 octets, aucun attribut. PUR (testable).
/// `txid` = identifiant de transaction (12 octets) qu'on retrouvera dans la réponse pour l'apparier.
pub(crate) fn encode_binding_request(txid: [u8; 12]) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..2].copy_from_slice(&MSG_BINDING_REQUEST.to_be_bytes());
    // octets 2..4 = longueur des attributs = 0 (déjà à zéro).
    b[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    b[8..20].copy_from_slice(&txid);
    b
}

/// Décode l'adresse PUBLIQUE réfléchie d'une réponse STUN. PUR (testable sans réseau). Rejette tout
/// paquet qui n'est pas une réponse de succès, dont le magic cookie ou l'identifiant de transaction
/// ne correspondent pas (anti-paquet-étranger). Cherche XOR-MAPPED-ADDRESS, sinon MAPPED-ADDRESS.
/// IPv4 uniquement (notre cas P2P) → une réponse purement IPv6 donne `None`.
pub(crate) fn decode_mapped_address(resp: &[u8], txid: [u8; 12]) -> Option<SocketAddr> {
    if resp.len() < 20 {
        return None;
    }
    if u16::from_be_bytes([resp[0], resp[1]]) != MSG_BINDING_SUCCESS {
        return None;
    }
    if u32::from_be_bytes([resp[4], resp[5], resp[6], resp[7]]) != MAGIC_COOKIE {
        return None;
    }
    if resp[8..20] != txid {
        return None;
    }
    let attr_len = u16::from_be_bytes([resp[2], resp[3]]) as usize;
    let end = (20 + attr_len).min(resp.len());
    let mut i = 20;
    // On peut rencontrer les deux attributs : on PRÉFÈRE la version XOR (renvoyée immédiatement),
    // en gardant la version nue comme repli si seule elle est présente.
    let mut fallback: Option<SocketAddr> = None;
    while i + 4 <= end {
        let atype = u16::from_be_bytes([resp[i], resp[i + 1]]);
        let alen = u16::from_be_bytes([resp[i + 2], resp[i + 3]]) as usize;
        let vstart = i + 4;
        let vend = vstart + alen;
        if vend > resp.len() {
            break;
        }
        let val = &resp[vstart..vend];
        match atype {
            ATTR_XOR_MAPPED_ADDRESS => {
                if let Some(a) = parse_addr(val, true) {
                    return Some(a);
                }
            }
            ATTR_MAPPED_ADDRESS => {
                if fallback.is_none() {
                    fallback = parse_addr(val, false);
                }
            }
            _ => {}
        }
        // Chaque attribut est aligné sur 4 octets (padding).
        i = vstart + ((alen + 3) & !3);
    }
    fallback
}

/// Lit un attribut d'adresse STUN IPv4 : `[réservé(1)][famille(1)][port(2)][adresse(4)]`. Si `xor`,
/// dé-masque port et adresse avec le magic cookie (XOR-MAPPED-ADDRESS). PUR.
fn parse_addr(val: &[u8], xor: bool) -> Option<SocketAddr> {
    if val.len() < 8 || val[1] != FAMILY_IPV4 {
        return None; // tronqué ou IPv6 → on ne gère pas
    }
    let mut port = u16::from_be_bytes([val[2], val[3]]);
    let mut octets = [val[4], val[5], val[6], val[7]];
    if xor {
        port ^= (MAGIC_COOKIE >> 16) as u16; // les 16 bits de poids fort du cookie
        let cookie = MAGIC_COOKIE.to_be_bytes();
        for k in 0..4 {
            octets[k] ^= cookie[k];
        }
    }
    Some(SocketAddr::from((Ipv4Addr::from(octets), port)))
}

/// Classe le NAT à partir des DEUX adresses réfléchies (une par serveur). PUR (testable). Même
/// adresse publique (IP + port) vue par les deux → cône (perçable) ; port différent → symétrique ;
/// moins de deux observations → indéterminé.
pub(crate) fn classify_nat(a: Option<SocketAddr>, b: Option<SocketAddr>) -> NatType {
    match (a, b) {
        (Some(x), Some(y)) => {
            if x.ip() == y.ip() && x.port() == y.port() {
                NatType::Cone
            } else {
                NatType::Symmetric
            }
        }
        _ => NatType::Unknown,
    }
}

/// Le résultat COMPLET de la sonde de lien : type de NAT + latence/gigue vers les serveurs STUN.
/// Le RTT/jitter sont mesurés « gratuitement » sur les mêmes aller-retours STUN (aucun serveur en
/// plus) : c'est la latence INTERNET générale de la machine, un bon indicateur de qualité de lien
/// (et la gigue trahit un lien mobile/saturé). La perte/le débit soutenable viendront d'une sonde
/// d'écho dédiée (étape suivante). `None` quand aucune réponse STUN exploitable.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct LinkProbe {
    pub nat: Option<NatType>,
    pub rtt_ms: Option<u32>,
    pub jitter_ms: Option<u32>,
    pub public_addr: Option<SocketAddr>,
}

/// La MÉDIANE d'une série de RTT (ms). PUR (testable). Trie une copie, prend l'élément central
/// (moyenne entière des deux centraux si la taille est paire). `None` si vide.
fn median_ms(samples: &[u32]) -> Option<u32> {
    if samples.is_empty() {
        return None;
    }
    let mut v = samples.to_vec();
    v.sort_unstable();
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        ((v[n / 2 - 1] as u64 + v[n / 2] as u64) / 2) as u32
    })
}

/// La GIGUE (jitter) au sens RFC 3550 : la moyenne des écarts ABSOLUS entre RTT successifs. PUR
/// (testable). C'est la variation du délai d'un paquet à l'autre — l'indicateur d'un lien instable
/// (mobile, congestionné). `None` s'il y a moins de 2 échantillons (pas de variation mesurable).
fn jitter_ms(samples: &[u32]) -> Option<u32> {
    if samples.len() < 2 {
        return None;
    }
    let mut total = 0u64;
    for w in samples.windows(2) {
        total += (w[0] as i64 - w[1] as i64).unsigned_abs();
    }
    Some((total / (samples.len() as u64 - 1)) as u32)
}

/// Un identifiant de transaction « assez unique » pour apparier requête/réponse. Pas besoin de
/// crypto-aléa ici (c'est juste un tag d'appariement) : horloge nanoseconde + adresse pile (ASLR).
fn random_txid() -> [u8; 12] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut t = [0u8; 12];
    t[0..8].copy_from_slice(&nanos.to_be_bytes());
    let salt = (&nanos as *const u64 as u64).to_be_bytes();
    t[8..12].copy_from_slice(&salt[0..4]);
    t
}

/// Interroge UN serveur STUN et renvoie (adresse publique réfléchie, RTT en ms de l'aller-retour
/// réussi). Best-effort : jusqu'à 3 tentatives (UDP peut perdre), on ignore les paquets venus
/// d'ailleurs. `None` = pas de réponse exploitable. Le chrono est remis à chaque tentative → on
/// mesure le RTT du SEUL aller-retour qui a abouti (pas le cumul des timeouts).
fn stun_query(socket: &UdpSocket, server: SocketAddr) -> Option<(SocketAddr, u32)> {
    let mut buf = [0u8; 512];
    for _ in 0..3 {
        let txid = random_txid();
        let request = encode_binding_request(txid);
        let t0 = std::time::Instant::now();
        if socket.send_to(&request, server).is_err() {
            return None;
        }
        match socket.recv_from(&mut buf) {
            Ok((n, from)) if from.ip() == server.ip() => {
                if let Some(addr) = decode_mapped_address(&buf[..n], txid) {
                    let rtt = t0.elapsed().as_millis().min(u32::MAX as u128) as u32;
                    return Some((addr, rtt));
                }
            }
            Ok(_) => continue,  // paquet d'un autre expéditeur → on retente
            Err(_) => continue, // timeout → on retente
        }
    }
    None
}

/// LA SONDE DE LIEN : interroge des serveurs STUN jusqu'à 2 réponses d'IP distinctes (pour classer
/// le NAT), puis fait une courte RAFALE vers le premier serveur répondant pour mesurer RTT médian
/// et gigue. Touche le réseau (au démarrage), borné par des timeouts courts → ne bloque jamais
/// durablement. Renvoie aussi les observations brutes (serveur → réflexive) pour le diagnostic.
pub(crate) fn probe_link() -> (LinkProbe, Vec<(SocketAddr, SocketAddr)>) {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return (LinkProbe::default(), Vec::new());
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(700)));

    // 1) Deux observations depuis deux serveurs d'IP DISTINCTE → classification du NAT.
    let mut obs: Vec<(SocketAddr, SocketAddr)> = Vec::new(); // (serveur, réflexive)
    let mut rtts: Vec<u32> = Vec::new();
    for host in STUN_SERVERS {
        let Ok(addrs) = host.to_socket_addrs() else { continue };
        let Some(server) = addrs.into_iter().find(|a| a.is_ipv4()) else { continue };
        if obs.iter().any(|(s, _)| s.ip() == server.ip()) {
            continue; // déjà une observation depuis cette IP → on en veut une AUTRE
        }
        if let Some((reflexive, rtt)) = stun_query(&socket, server) {
            obs.push((server, reflexive));
            rtts.push(rtt);
            if obs.len() >= 2 {
                break;
            }
        }
    }

    let nat = if obs.len() >= 2 {
        Some(classify_nat(Some(obs[0].1), Some(obs[1].1)))
    } else if obs.len() == 1 {
        Some(NatType::Unknown) // une seule observation → on ne peut pas trancher cône/symétrique
    } else {
        None
    };

    // 2) Rafale vers le premier serveur répondant → RTT successifs pour la gigue.
    let mut burst: Vec<u32> = Vec::new();
    if let Some((server, _)) = obs.first() {
        for _ in 0..6 {
            if let Some((_, rtt)) = stun_query(&socket, *server) {
                burst.push(rtt);
            }
        }
    }

    let mut all_rtts = rtts.clone();
    all_rtts.extend_from_slice(&burst);
    let probe = LinkProbe {
        nat,
        rtt_ms: median_ms(&all_rtts),
        jitter_ms: jitter_ms(&burst),
        public_addr: obs.first().map(|(_, r)| *r),
    };
    (probe, obs)
}

/// Le fragment JSON de SONDE à coller dans un battement de cœur (toujours préfixé par `,`), ex.
/// `,"nat":"sym","rtt":120,"jitter":35`. Champs absents si indisponibles (rétro-compatible : un
/// vieux lecteur ignore ce qu'il ne connaît pas, le serveur recopie tout verbatim). Fait l'aller-
/// retour réseau → à appeler UNE fois (en arrière-plan), pas dans une boucle chaude.
pub(crate) fn link_diag() -> String {
    let (p, _) = probe_link();
    let mut d = String::new();
    if let Some(nat) = p.nat {
        d.push_str(&format!(",\"nat\":\"{}\"", nat_str(nat)));
    }
    if let Some(rtt) = p.rtt_ms {
        d.push_str(&format!(",\"rtt\":{rtt}"));
    }
    if let Some(jitter) = p.jitter_ms {
        d.push_str(&format!(",\"jitter\":{jitter}"));
    }
    d
}

/// `jeu natcheck` : sonde le lien de CETTE machine et imprime un verdict clair (perçable ou non,
/// latence, gigue). Outil de diagnostic à la main, jumeau de `jeu phase1` : zéro popup.
pub fn run_natcheck() {
    println!("[natcheck] Sonde du lien (NAT via STUN + latence/gigue)…");
    let (p, obs) = probe_link();
    if obs.is_empty() {
        println!("[natcheck] Aucune réponse STUN — UDP bloqué, hors-ligne, ou serveurs injoignables.");
    } else {
        for (i, (server, reflexive)) in obs.iter().enumerate() {
            println!("[natcheck]   serveur {} ({server}) → adresse publique vue : {reflexive}", i + 1);
        }
    }
    let verdict = match p.nat {
        Some(NatType::Cone) => "CÔNE → perçable par hole-punching (connexion directe possible)",
        Some(NatType::Symmetric) => "SYMÉTRIQUE/CGNAT → perçage direct impossible → RELAIS obligatoire",
        _ => "INDÉTERMINÉ → besoin d'au moins 2 réponses STUN d'IP distinctes",
    };
    let nat_label = p.nat.map(nat_str).unwrap_or("?");
    println!("[natcheck] Verdict : nat={nat_label} — {verdict}");
    match (p.rtt_ms, p.jitter_ms) {
        (Some(r), Some(j)) => println!("[natcheck] Latence vers STUN : rtt médian {r} ms, gigue {j} ms."),
        (Some(r), None) => println!("[natcheck] Latence vers STUN : rtt médian {r} ms (gigue indisponible)."),
        _ => println!("[natcheck] Latence : indisponible (pas assez de réponses)."),
    }
}

// ───────────────────────── Sonde de PERTE / CONGESTION (Phase 2b) ─────────────────────────

/// Taille (octets) d'un paquet d'écho de sonde. Fixe → on fait varier le DÉBIT par le nombre de
/// paquets/s, pas par la taille (plus simple à raisonner). Bornée < `MAX_ECHO_SIZE` côté serveur.
const LOSS_PACKET_SIZE: usize = 200;
/// Les PALIERS de débit (paquets/s) testés, croissants. À 200 o/paquet : ~0,32 / 0,8 / 1,6 / 3,2 Mbit/s.
/// On ne cherche PAS à saturer un gros lien (intrusif) — on cherche la PENTE : la perte/le RTT
/// montent-ils quand on pousse le débit ? Volume aller total ≈ 444 Ko (respect d'un lien mobile compté).
const LOSS_RATES_PPS: &[u32] = &[200, 500, 1000, 2000];
/// Durée (s) de chaque palier — assez pour un échantillon stable, assez court pour rester léger.
const LOSS_STEP_SECS: f32 = 0.6;

/// Fabrique un paquet d'écho de sonde de taille `LOSS_PACKET_SIZE` portant `seq` (pour apparier la
/// réponse). PUR (testable). Le rendez-vous le renvoie tel quel → on retrouve `seq` au retour.
fn encode_echo(seq: u64) -> Vec<u8> {
    let mut p = vec![0u8; LOSS_PACKET_SIZE];
    p[0] = KIND_ECHO;
    p[1] = PROTO_VERSION;
    p[2..10].copy_from_slice(&seq.to_be_bytes());
    p
}

/// Lit le `seq` d'un paquet d'écho renvoyé. PUR (testable). `None` si ce n'est pas un écho valide.
fn decode_echo_seq(buf: &[u8]) -> Option<u64> {
    if buf.len() < 10 || buf[0] != KIND_ECHO || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut s = [0u8; 8];
    s.copy_from_slice(&buf[2..10]);
    Some(u64::from_be_bytes(s))
}

/// Une mesure par palier : débit EFFECTIF (Mbit/s), perte (%), RTT médian (ms).
type LossPoint = (f64, f64, u32);

/// CLASSE la tendance perte/RTT vs débit. PUR (testable). Compare le palier bas au palier haut :
///  • la perte OU le RTT grimpent nettement avec le débit → **congestion** (le lien sature) ;
///  • perte présente mais ~PLATE selon le débit → **aléatoire** (bruit, pas saturation) ;
///  • perte faible + RTT stable → **sain** (le lien a de la marge sur la plage testée).
fn classify_loss_trend(points: &[LossPoint]) -> &'static str {
    if points.len() < 2 {
        return "indéterminé (pas assez de paliers)";
    }
    let (_, loss_lo, rtt_lo) = points[0];
    let (_, loss_hi, rtt_hi) = points[points.len() - 1];
    let rtt_grimpe = rtt_hi as f64 >= (rtt_lo as f64) * 2.0 + 30.0; // bufferbloat net
    let perte_grimpe = loss_hi >= loss_lo + 10.0; // la perte monte avec le débit
    let perte_haute_plate = loss_lo > 5.0 && (loss_hi - loss_lo).abs() < 10.0;
    if perte_grimpe || rtt_grimpe {
        "CONGESTION (perte/latence montent avec le débit → le lien sature)"
    } else if perte_haute_plate {
        "ALÉATOIRE (perte présente mais ~constante selon le débit → bruit, pas saturation)"
    } else {
        "SAIN (perte faible + latence stable → le lien a de la marge sur la plage testée)"
    }
}

/// Draine les échos déjà arrivés (non-bloquant) : pour chaque réponse appariée à un envoi en attente,
/// compte une réception et enregistre son RTT. Mutualisé entre la phase d'envoi et la grâce finale.
fn drain_echos(
    socket: &UdpSocket,
    rdv: SocketAddr,
    pending: &mut HashMap<u64, Instant>,
    recv: &mut u64,
    rtts: &mut Vec<u32>,
) {
    let mut buf = [0u8; 512];
    loop {
        match socket.recv_from(&mut buf) {
            Ok((n, from)) if from == rdv => {
                if let Some(seq) = decode_echo_seq(&buf[..n]) {
                    if let Some(t) = pending.remove(&seq) {
                        *recv += 1;
                        rtts.push(t.elapsed().as_millis().min(u32::MAX as u128) as u32);
                    }
                }
            }
            Ok(_) => continue,                                                  // pas du rendez-vous
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,      // plus rien pour l'instant
            Err(_) => break,
        }
    }
}

/// `jeu losscheck` : mesure la PERTE et le RTT par palier de débit croissant, vers le rendez-vous
/// (qui fait écho 1:1). Dit si le lien est SAIN, ALÉATOIRE ou en CONGESTION — le chaînon manquant
/// pour la redondance ADAPTATIVE (Phase 3 : ne dédoubler que sur perte aléatoire, jamais si ça sature).
pub fn run_losscheck() {
    let rdv = rendezvous_addr();
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        println!("[losscheck] Impossible d'ouvrir une prise UDP.");
        return;
    };
    let _ = socket.set_nonblocking(true);
    println!("[losscheck] Sonde de congestion vers le rendez-vous {rdv} (écho 1:1, paliers de débit)…");
    println!("[losscheck] {:>9} | {:>7} | {:>9} | {:>9}", "débit", "perte", "rtt méd.", "paquets");

    let mut seq: u64 = 0;
    let mut total_bytes: u64 = 0;
    let mut points: Vec<LossPoint> = Vec::new();

    for &pps in LOSS_RATES_PPS {
        let interval = Duration::from_secs_f64(1.0 / pps as f64);
        let step = Duration::from_secs_f32(LOSS_STEP_SECS);
        let mut pending: HashMap<u64, Instant> = HashMap::new();
        let (mut sent, mut recv) = (0u64, 0u64);
        let mut rtts: Vec<u32> = Vec::new();

        let start = Instant::now();
        let mut next_send = start;
        while start.elapsed() < step {
            let now = Instant::now();
            // Envoi cadencé. Si on a pris du retard de plus d'un intervalle, on se resynchronise sur
            // « maintenant » (pas de rafale de rattrapage) → on mesure le débit RÉELLEMENT atteint.
            while next_send <= now {
                let pkt = encode_echo(seq);
                if socket.send_to(&pkt, rdv).is_ok() {
                    sent += 1;
                    total_bytes += pkt.len() as u64;
                    pending.insert(seq, now);
                }
                seq += 1;
                next_send += interval;
                if next_send + interval < now {
                    next_send = now; // anti-dérive : on ne rattrape pas un gros retard en rafale
                }
            }
            drain_echos(&socket, rdv, &mut pending, &mut recv, &mut rtts);
            std::thread::sleep(Duration::from_micros(200)); // évite le spin CPU à 100 %
        }
        // Fenêtre de grâce : laisser revenir les échos retardataires avant de compter la perte.
        let grace_end = Instant::now() + Duration::from_millis(250);
        while Instant::now() < grace_end {
            drain_echos(&socket, rdv, &mut pending, &mut recv, &mut rtts);
            std::thread::sleep(Duration::from_micros(200));
        }

        let loss_pct = if sent > 0 { 100.0 * (sent - recv) as f64 / sent as f64 } else { 0.0 };
        let eff_pps = sent as f64 / LOSS_STEP_SECS as f64;
        let mbps = eff_pps * LOSS_PACKET_SIZE as f64 * 8.0 / 1.0e6;
        let rtt_med = median_ms(&rtts).unwrap_or(0);
        println!(
            "[losscheck] {:>6.2} Mb | {:>5.1} % | {:>6} ms | {:>4}/{:<4}",
            mbps, loss_pct, rtt_med, recv, sent
        );
        points.push((mbps, loss_pct, rtt_med));
    }

    println!("[losscheck] Volume envoyé : {:.0} Ko (+ autant en retour). Verdict : {}",
        total_bytes as f64 / 1024.0, classify_loss_trend(&points));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une réponse STUN forgée à la main, avec un attribut au choix, pour tester le décodage SANS réseau.
    /// `attr_type` + valeur d'adresse IPv4 (`[réservé, famille, port_hi, port_lo, a, b, c, d]`).
    fn forge_response(txid: [u8; 12], attr_type: u16, addr_val: &[u8]) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(&MSG_BINDING_SUCCESS.to_be_bytes());
        let attr_total = 4 + addr_val.len(); // en-tête d'attribut (4) + valeur
        r.extend_from_slice(&(attr_total as u16).to_be_bytes());
        r.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        r.extend_from_slice(&txid);
        r.extend_from_slice(&attr_type.to_be_bytes());
        r.extend_from_slice(&(addr_val.len() as u16).to_be_bytes());
        r.extend_from_slice(addr_val);
        r
    }

    /// La requête : 20 octets, type binding-request, longueur 0, magic cookie, et notre txid intact.
    #[test]
    fn requete_binding_bien_formee() {
        let txid = [7u8; 12];
        let req = encode_binding_request(txid);
        assert_eq!(req.len(), 20);
        assert_eq!(u16::from_be_bytes([req[0], req[1]]), MSG_BINDING_REQUEST);
        assert_eq!(u16::from_be_bytes([req[2], req[3]]), 0); // aucun attribut
        assert_eq!(u32::from_be_bytes([req[4], req[5], req[6], req[7]]), MAGIC_COOKIE);
        assert_eq!(&req[8..20], &txid);
    }

    /// XOR-MAPPED-ADDRESS : on masque une adresse connue, on vérifie que le décodage la retrouve.
    #[test]
    fn decode_xor_mapped_address() {
        let txid = [1u8; 12];
        // On veut décoder 203.0.113.5:50000 → on FABRIQUE la valeur masquée (XOR cookie).
        let ip = [203u8, 0, 113, 5];
        let port: u16 = 50000;
        let cookie = MAGIC_COOKIE.to_be_bytes();
        let xport = port ^ (MAGIC_COOKIE >> 16) as u16;
        let xip = [ip[0] ^ cookie[0], ip[1] ^ cookie[1], ip[2] ^ cookie[2], ip[3] ^ cookie[3]];
        let val = [0x00, FAMILY_IPV4, (xport >> 8) as u8, xport as u8, xip[0], xip[1], xip[2], xip[3]];
        let resp = forge_response(txid, ATTR_XOR_MAPPED_ADDRESS, &val);
        assert_eq!(
            decode_mapped_address(&resp, txid),
            Some(SocketAddr::from((Ipv4Addr::new(203, 0, 113, 5), 50000)))
        );
    }

    /// MAPPED-ADDRESS (nue, sans XOR) : repli quand le serveur n'envoie que la vieille version.
    #[test]
    fn decode_mapped_address_nue() {
        let txid = [2u8; 12];
        let port: u16 = 4000;
        let val = [0x00, FAMILY_IPV4, (port >> 8) as u8, port as u8, 88, 167, 242, 251];
        let resp = forge_response(txid, ATTR_MAPPED_ADDRESS, &val);
        assert_eq!(
            decode_mapped_address(&resp, txid),
            Some(SocketAddr::from((Ipv4Addr::new(88, 167, 242, 251), 4000)))
        );
    }

    /// Un paquet dont le txid ne correspond PAS est rejeté (anti-paquet-étranger), idem mauvais cookie.
    #[test]
    fn decode_rejette_les_imposteurs() {
        let txid = [3u8; 12];
        let val = [0x00, FAMILY_IPV4, 0x0f, 0xa0, 1, 2, 3, 4];
        let bon = forge_response(txid, ATTR_MAPPED_ADDRESS, &val);
        // Mauvais txid attendu → None.
        assert_eq!(decode_mapped_address(&bon, [9u8; 12]), None);
        // Mauvais cookie → None.
        let mut cookie_casse = bon.clone();
        cookie_casse[4] ^= 0xff;
        assert_eq!(decode_mapped_address(&cookie_casse, txid), None);
        // Trop court → None.
        assert_eq!(decode_mapped_address(&bon[..10], txid), None);
    }

    /// Médiane : impair → l'élément central ; pair → moyenne entière des deux centraux ; vide → None.
    #[test]
    fn mediane_des_rtt() {
        assert_eq!(median_ms(&[]), None);
        assert_eq!(median_ms(&[42]), Some(42));
        assert_eq!(median_ms(&[30, 10, 20]), Some(20)); // trié : 10,20,30 → 20
        assert_eq!(median_ms(&[10, 20, 30, 40]), Some(25)); // (20+40)/2 ... trié 10,20,30,40 → (20+30)/2=25
    }

    /// Gigue = moyenne des écarts absolus entre RTT successifs ; < 2 échantillons → None.
    #[test]
    fn gigue_ecarts_successifs() {
        assert_eq!(jitter_ms(&[]), None);
        assert_eq!(jitter_ms(&[50]), None);
        // |12-10| + |11-12| + |50-11| = 2+1+39 = 42 ; /3 = 14
        assert_eq!(jitter_ms(&[10, 12, 11, 50]), Some(14));
        // lien stable : écarts nuls → gigue 0
        assert_eq!(jitter_ms(&[20, 20, 20]), Some(0));
    }

    /// La classification : même adresse publique vue par 2 serveurs = cône ; port différent =
    /// symétrique ; moins de 2 observations = indéterminé.
    #[test]
    fn classification_cone_sym_unknown() {
        let pub1 = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 40000));
        let pub_meme = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 40000));
        let pub_autre_port = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 51234));
        assert_eq!(classify_nat(Some(pub1), Some(pub_meme)), NatType::Cone);
        assert_eq!(classify_nat(Some(pub1), Some(pub_autre_port)), NatType::Symmetric);
        assert_eq!(classify_nat(Some(pub1), None), NatType::Unknown);
        assert_eq!(classify_nat(None, None), NatType::Unknown);
        assert_eq!(nat_str(NatType::Cone), "cone");
        assert_eq!(nat_str(NatType::Symmetric), "sym");
        assert_eq!(nat_str(NatType::Unknown), "?");
    }

    /// Le paquet d'écho : bonne taille, bon type, et le `seq` survit à l'aller-retour ; rejet des malformés.
    #[test]
    fn echo_seq_aller_retour() {
        let p = encode_echo(0xDEAD_BEEF_01);
        assert_eq!(p.len(), LOSS_PACKET_SIZE);
        assert_eq!(p[0], KIND_ECHO);
        assert_eq!(decode_echo_seq(&p), Some(0xDEAD_BEEF_01));
        assert_eq!(decode_echo_seq(&p[..5]), None); // trop court
        let mut faux = p.clone();
        faux[0] = 99;
        assert_eq!(decode_echo_seq(&faux), None); // mauvais type
    }

    /// La classification de tendance : sain / congestion (par perte OU par RTT) / aléatoire / indéterminé.
    #[test]
    fn tendance_perte_congestion_alea_sain() {
        assert!(classify_loss_trend(&[(0.3, 0.0, 20), (3.2, 0.5, 22)]).starts_with("SAIN"));
        assert!(classify_loss_trend(&[(0.3, 1.0, 20), (3.2, 30.0, 25)]).starts_with("CONGESTION"));
        // congestion par bufferbloat (RTT qui grimpe) même sans perte
        assert!(classify_loss_trend(&[(0.3, 0.0, 20), (3.2, 0.0, 120)]).starts_with("CONGESTION"));
        assert!(classify_loss_trend(&[(0.3, 12.0, 20), (3.2, 14.0, 22)]).starts_with("ALÉATOIRE"));
        assert!(classify_loss_trend(&[(0.3, 0.0, 20)]).starts_with("indéterminé"));
    }
}
