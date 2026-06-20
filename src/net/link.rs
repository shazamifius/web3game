//! LE LIEN : `NetLink`, la ressource Bevy qui tient le réseau d'un client.
//!
//! Présente uniquement en mode multijoueur. Elle contient la prise UDP, l'adresse
//! du rendez-vous, notre identifiant (attribué par le rendez-vous → `Option`
//! tant qu'on ne l'a pas), notre couleur, et l'ANNUAIRE des autres joueurs.

use super::accuse::encode_accuse;
use super::aoi::{cell_of, dist2, relevance_weight, FOCUS_SWAP_MARGIN, K_FOCUS};
use super::cell::{build_cell_summary, CellSummary};
use super::crypto::{Identity, PeerId, pow_bits};
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use bevy::prelude::Resource;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::time::Instant;

#[derive(Resource)]
pub struct NetLink {
    pub(crate) socket: Socket,
    pub(crate) rendezvous: SocketAddr,
    /// Notre identité : `None` tant que le rendez-vous ne nous a pas (encore) répondu
    /// (sert juste de garde « suis-je inscrit ? »). Dès le 1er WELCOME, on y met notre
    /// propre clé (`identity.id()`) : depuis le chap. 6.1, l'identité n'est plus un
    /// numéro attribué — c'est notre clé, qu'on connaît dès le départ.
    pub(crate) my_id: Option<PeerId>,
    pub(crate) my_color: (f32, f32, f32),
    /// Notre identité cryptographique (paire de clés). On SIGNE nos paquets avec ;
    /// notre clé publique EST notre identité, portée dans chaque paquet.
    pub(crate) identity: Identity,
    pub(crate) world_hue: Option<u16>, // couleur de salle donnée par le serveur (None = pas connecté)
    pub(crate) peers: HashMap<PeerId, SocketAddr>, // les autres joueurs : identité → adresse
    /// DERNIÈRE POSITION (x, z) INDICATIVE de chaque pair (chap. 8.1). Alimentée par les états
    /// reçus ET par les cartes de gossip — donc une partie est NON CORROBORÉE (un tiers a pu la
    /// revendiquer). C'est un INDICE de DÉCOUVERTE/AoI (pondérer la pertinence avant d'avoir
    /// entendu le pair, fabriquer nos cartes), **jamais une base de confiance**. Pour toute
    /// décision de confiance (co-localisation d'un témoin, 9.2), on utilise `confirmed_pos`.
    pub(crate) peer_pos: HashMap<PeerId, (f32, f32)>,
    /// POSITION CORROBORÉE (x, z) d'un pair (chap. 9.4) : écrite UNIQUEMENT par `note_pos`, c.-à-d.
    /// depuis un ÉTAT SIGNÉ du pair lui-même qu'on a accepté. Le gossip n'y touche JAMAIS. C'est la
    /// seule position digne de confiance : un attaquant ne peut pas faire croire qu'un pair est
    /// ailleurs (ex. coller un témoin sur une victime pour fabriquer une fausse co-localisation et
    /// framer, D9). Les jugements de crédibilité (9.2) la lisent ; `peer_pos` reste l'indice ouvert.
    pub(crate) confirmed_pos: HashMap<PeerId, (f32, f32)>,
    /// ANTI-REJEU (chap. 5.2, durci au 7.3) : une FENÊTRE GLISSANTE par pair (style
    /// IPsec/DTLS). On retient le plus grand `seq` vu + un masque des 64 derniers déjà
    /// acceptés → on tolère le ré-ordonnancement du réseau (un paquet en retard mais
    /// jamais vu, dans la fenêtre, passe) SANS rouvrir le rejeu (un seq déjà vu, ou trop
    /// vieux pour la fenêtre, est refusé).
    pub(crate) replay: HashMap<PeerId, ReplayWindow>,
    /// RÉPUTATION (chap. 5.4) : nombre de « fautes » constatées par pair (état
    /// impossible, orbe trichée…). Au-delà de `MAX_STRIKES`, le pair est mis en
    /// sourdine (« mute ») : on ignore tout ce qu'il envoie.
    pub(crate) strikes: HashMap<PeerId, Strike>,
    /// RÉPUTATION PARTAGÉE (chap. 6.7) : pour chaque tricheur, l'ensemble des nœuds
    /// DISTINCTS qui l'ont accusé. Au-delà de `ACCUSE_QUORUM`, on le bannit aussi.
    pub(crate) accusations: HashMap<PeerId, HashSet<PeerId>>,
    /// Tricheurs pour lesquels on a DÉJÀ diffusé NOTRE accusation (une seule fois).
    pub(crate) accused_broadcast: HashSet<PeerId>,
    /// SEAU À JETONS PAR SOURCE DE GOSSIP (chap. 8.1b, ferme D23). Combien de NOUVEAUX
    /// pairs cet expéditeur de cartes a-t-il encore le droit de nous faire apprendre ?
    /// Un attaquant qui déverse des milliers de cartes ne peut plus nous faire percer
    /// (réflexion) ni gonfler nos tables au-delà de ce débit borné — même protocole que
    /// le rate-limit de réception (5.5), mais appliqué à l'APPRENTISSAGE, pas au paquet.
    pub(crate) gossip_credit: HashMap<SocketAddr, f32>,
    /// ENSEMBLE FOCUS COLLANT (chap. 8.2a-bis) : les pairs à qui on tient un lien plein débit
    /// (20 Hz, prédiction, avatar détaillé). Maintenu avec HYSTÉRÉSIS par `refresh_focus` →
    /// on NE recompose PAS le top-K à chaque tick (ce qui causait le « churn » mesuré au 8.2b),
    /// on garde les membres tant qu'ils restent pertinents. Le reste de la table = la CONSCIENCE.
    pub(crate) focus: Vec<PeerId>,
    /// RÉSUMÉS DE CELLULE reçus (chap. 8.3) : le dernier résumé connu par cellule. C'est ce qui
    /// nous fait PERCEVOIR une foule lointaine via UN flux par cellule au lieu de N états — fin de
    /// l'effondrement de fraîcheur en 1/N de la conscience. Borné par `MAX_CELLS` (anti-inondation
    /// de fausses cellules). Consultatif (pas autoritaire) : un résumé en double ne corrompt rien.
    pub(crate) cell_summaries: HashMap<(i32, i32), CellSummary>,
    pub(crate) weak: bool, // faible upload : on émet notre état via un parent (relais) au lieu de tous
}

/// Nombre de fautes au-delà duquel on coupe le son d'un pair (réputation). Chaque
/// nœud est ainsi le « Shield » de ce qu'il observe : il détecte et bannit localement.
pub(crate) const MAX_STRIKES: u32 = 5;

/// RÉHABILITATION (chap. 9.3, D8) : temps (s) pour qu'UNE faute se dissipe. Le score de fautes
/// décroît linéairement → 5 fautes = sourdine, mais il suffit de ~`5 × STRIKE_DECAY_SECS` sans
/// récidive pour redevenir audible. Un glitch transitoire ne bannit donc plus À VIE ; un
/// récidiviste, lui, ré-accumule plus vite qu'il ne décroît et reste muet. (Réglage assumé.)
const STRIKE_DECAY_SECS: f32 = 60.0;

/// Une marque de réputation à DÉCROISSANCE (chap. 9.3) : un score de fautes + l'instant de la
/// dernière mise à jour, pour décroître paresseusement à la lecture (pas de balayage périodique).
pub(crate) struct Strike {
    score: f32,
    last: Instant,
}

/// Score de fautes APRÈS décroissance, étant donné le temps écoulé depuis la dernière faute
/// (chap. 9.3). Pur → testable sans horloge. Décroissance linéaire, plancher à 0.
fn decayed_score(score: f32, elapsed_secs: f32) -> f32 {
    (score - elapsed_secs / STRIKE_DECAY_SECS).max(0.0)
}
/// Nombre d'accusateurs DISTINCTS requis avant de bannir un tricheur qu'on n'a PAS
/// vu soi-même (chap. 6.7). Anti-framing : un seul (ou quelques) menteur(s) ne suffit
/// pas. ⚠ Depuis 9.2, ce n'est plus un simple COMPTE : c'est le seuil de POIDS cumulé
/// (`ACCUSE_WEIGHT_QUORUM`) — l'attaque `sybil-frame` a prouvé qu'avec une PoW jouet, 3
/// identités conjurées suffisaient (D6/D7/D20). On PONDÈRE désormais chaque accusateur.
pub(crate) const ACCUSE_QUORUM: usize = 3;

/// Seuil de POIDS cumulé d'accusations pour bannir par quorum (chap. 9.2). On ne compte
/// plus des têtes (frameable par des Sybils bon marché) : on SOMME le poids de crédibilité
/// de chaque accusateur (`accusation_weight`) et on ne bannit qu'au-delà de ce seuil. Égal au
/// vieux compte → il faut ~3 TÉMOINS CRÉDIBLES co-localisés (et 0 Sybil conjuré ne contribue).
pub(crate) const ACCUSE_WEIGHT_QUORUM: f32 = ACCUSE_QUORUM as f32;

/// Rayon (m) sous lequel un accusateur a pu ÊTRE TÉMOIN de la triche de l'accusé (chap. 9.2) :
/// au-delà, il n'était pas à portée pour « voir » → plausibilité réduite. Large (englobe l'AoI
/// proche) car la triche se constate dans le voisinage.
pub(crate) const WITNESS_RADIUS: f32 = 50.0;

/// Poids PLANCHER d'un accusateur ÉTABLI (il a du standing) mais dont je ne peux pas confirmer
/// la co-localisation avec l'accusé (chap. 9.2). Il compte un peu (réputation de voisin réel),
/// mais il faut alors BEAUCOUP plus d'accusateurs qu'avec des témoins co-localisés → dégradation
/// gracieuse, sans jamais ouvrir la porte au framing bon marché (un Sybil SANS standing pèse 0).
pub(crate) const WITNESS_FLOOR: f32 = 0.34;

/// Plafond MÉMOIRE de la table de pairs connus (chap. 8.1, D22). Le gossip lève le
/// plafond de VISION (32) — on peut apprendre toute la foule — mais pas celui de la
/// MÉMOIRE : au-delà de `MAX_KNOWN` cartes, on cesse d'en ajouter (anti-inondation de
/// fausses cartes). Très large devant une foule réaliste ; l'éviction fine (TTL) est D16.
pub(crate) const MAX_KNOWN: usize = 4096;

/// Plafond du nombre de RÉSUMÉS de cellule retenus (chap. 8.3). Comme `MAX_KNOWN` pour les pairs :
/// borne la mémoire contre un attaquant qui inventerait des milliers de fausses cellules. Très
/// large devant le nombre de cellules réellement à portée d'un joueur.
pub(crate) const MAX_CELLS: usize = 4096;

/// NOUVEAUX pairs/s qu'UNE source de gossip peut nous faire apprendre (chap. 8.1b, D23).
/// Au-delà, ses cartes inconnues sont ignorées : un attaquant ne peut plus nous faire
/// percer 1000 victimes d'un coup (réflexion) ni saturer la table par rafale. Généreux
/// pour la découverte honnête (un voisin présente ≤16 cartes/paquet à ~2 Hz = ~32/s en
/// pointe, mais on n'apprend chaque pair qu'UNE fois → en régime, presque rien à dépenser).
pub(crate) const GOSSIP_LEARN_RATE: f32 = 16.0;
/// Réserve max du seau d'apprentissage (tolère une courte rafale de découverte au démarrage).
pub(crate) const GOSSIP_LEARN_CAP: f32 = 64.0;
/// Plafond du nombre de sources de gossip suivies (anti-saturation mémoire, comme
/// `MAX_BUCKETS` au 6.5) : au-delà, on jette les seaux PLEINS (sources inactives).
pub(crate) const MAX_GOSSIP_SOURCES: usize = 4096;

/// Largeur de la fenêtre d'anti-rejeu, en nombre de `seq` (chap. 7.3). 64 = un masque
/// `u64`, zéro allocation. À 20 paquets/s ça couvre ~3,2 s de ré-ordonnancement possible
/// — bien plus que ce qu'un vrai réseau (ou `tc netem`) produit. Choix standard
/// IPsec/DTLS/WireGuard.
const REPLAY_WINDOW: u64 = 64;

/// FENÊTRE GLISSANTE D'ANTI-REJEU pour un pair (chap. 7.3). `top` = le plus grand `seq`
/// accepté ; `mask` = un masque où le bit `i` signale que le `seq` (`top` − `i`) a déjà
/// été vu (bit 0 = `top` lui-même). On accepte : tout `seq` > `top` (on fait glisser la
/// fenêtre), et tout `seq` DANS la fenêtre `[top−63, top]` pas encore vu (← c'est ce qui
/// répare le ré-ordonnancement). On refuse : un `seq` déjà vu (vrai rejeu) et un `seq`
/// trop vieux (< `top` − 63). Remplace l'ancien « `seq` ≤ dernier → rejet » strict, qui
/// jetait les paquets honnêtes ré-ordonnés par le réseau (bug mesuré au 7.2 : −70 % de
/// débit honnête sous `mauvais`).
pub(crate) struct ReplayWindow {
    top: u64,
    mask: u64,
}

impl ReplayWindow {
    /// Crée la fenêtre sur le premier `seq` vu d'un pair (marqué comme déjà reçu).
    fn new(seq: u64) -> ReplayWindow {
        ReplayWindow { top: seq, mask: 1 }
    }

    /// Tente d'accepter `seq`. `true` s'il est neuf (et on le mémorise), `false` si
    /// c'est un rejeu ou s'il est trop vieux pour la fenêtre.
    fn accept(&mut self, seq: u64) -> bool {
        if seq > self.top {
            // Plus récent que tout : on fait glisser la fenêtre vers l'avant. Les seq
            // déjà vus reculent de `diff` bits ; au-delà de 64 ils sortent de la fenêtre.
            let diff = seq - self.top;
            self.mask = if diff >= REPLAY_WINDOW { 1 } else { (self.mask << diff) | 1 };
            self.top = seq;
            true
        } else {
            // Égal ou plus ancien : neuf seulement si jamais vu ET encore dans la fenêtre.
            let diff = self.top - seq;
            if diff >= REPLAY_WINDOW {
                return false; // trop vieux : hors de la fenêtre
            }
            let bit = 1u64 << diff;
            if self.mask & bit != 0 {
                false // déjà vu : vrai rejeu
            } else {
                self.mask |= bit;
                true // en retard mais légitime → accepté (le fix du 7.3)
            }
        }
    }
}

impl NetLink {
    /// Prépare le réseau d'un client : prise sur un port éphémère (choisi par
    /// l'OS), et adresse du rendez-vous local. `weak` = mode « faible upload » :
    /// on n'émet plus son état à tous les pairs, mais une seule fois à un parent
    /// (relais) qui le recopie à notre place.
    pub fn new(color: (f32, f32, f32), weak: bool) -> std::io::Result<NetLink> {
        let socket = Socket::bind(0)?; // 0 = l'OS choisit un port libre
        let rendezvous = rendezvous_addr();
        // On MINE notre identité (preuve de travail anti-Sybil, chap. 6.2/9.1) une fois,
        // au lancement. La privée reste ici. Ce coût (réglable, `pow_bits()`) rend une
        // identité « chère » → ni reconnexion gratuite après bannissement, ni Sybil de masse.
        // En TESTS : identité NON minée (rapide) — sinon `cargo test` minerait à pleine
        // difficulté à chaque `NetLink::new` ; la PoW elle-même est testée dans `crypto`.
        #[cfg(test)]
        let identity = Identity::generate();
        #[cfg(not(test))]
        let identity = Identity::generate_pow(pow_bits());
        println!(
            "Client réseau : port local {}, rendez-vous {}{}.",
            socket.local_addr()?,
            rendezvous,
            if weak { " (faible upload : via un parent)" } else { "" }
        );
        Ok(NetLink {
            socket,
            rendezvous,
            my_id: None,
            my_color: color,
            identity,
            world_hue: None,
            peers: HashMap::new(),
            peer_pos: HashMap::new(),
            confirmed_pos: HashMap::new(),
            replay: HashMap::new(),
            strikes: HashMap::new(),
            accusations: HashMap::new(),
            accused_broadcast: HashSet::new(),
            gossip_credit: HashMap::new(),
            focus: Vec::new(),
            cell_summaries: HashMap::new(),
            weak,
        })
    }

    /// APPREND un pair depuis une source CORROBORÉE (chap. 8.1) : le WELCOME (amorçage par
    /// le rendez-vous) ou un signal direct. L'ajoute s'il est nouveau, en RAFRAÎCHISSANT son
    /// adresse et sa position si fournie. Renvoie `true` SEULEMENT si c'était un INCONNU (→
    /// l'appelant ouvrira un trou NAT). On ne s'apprend jamais soi-même, ni l'identité nulle.
    ///
    /// **PoW exigée (chap. 8.1b, D23) :** un id sans preuve de travail (`has_pow`) est ignoré —
    /// même venant du rendez-vous (défense contre un rendez-vous menteur, D10). Une fausse
    /// identité coûte donc ~2¹⁶ comme une vraie.
    ///
    /// Borne MÉMOIRE (`MAX_KNOWN`) : la VISION n'est plus plafonnée (l'intérêt du gossip contre
    /// D22), mais la mémoire SI — sinon un menteur nous noierait. Table pleine → on refuse le
    /// nouveau (l'éviction fine par TTL est le chap. 12 / D16 ; ici, borne dure).
    pub(crate) fn learn_peer(&mut self, id: PeerId, addr: SocketAddr, pos: Option<(f32, f32)>) -> bool {
        if id.is_none() || Some(id) == self.my_id || !id.has_pow(pow_bits()) {
            return false;
        }
        if let Some(xz) = pos {
            self.peer_pos.insert(id, xz);
        }
        if self.peers.contains_key(&id) {
            self.peers.insert(id, addr); // adresse rafraîchie (source corroborée), pas un nouveau
            return false;
        }
        if self.peers.len() >= MAX_KNOWN {
            return false; // table pleine : on borne la mémoire (D16)
        }
        self.peers.insert(id, addr);
        true
    }

    /// RECHARGE les seaux d'apprentissage de gossip (chap. 8.1b) : à appeler une fois par
    /// image, comme la recharge des seaux de réception (5.5). Évince les sources en trop
    /// (seaux pleins = sources inactives) pour borner la mémoire.
    pub(crate) fn recharge_gossip_credit(&mut self, dt: f32) {
        for credit in self.gossip_credit.values_mut() {
            *credit = (*credit + dt * GOSSIP_LEARN_RATE).min(GOSSIP_LEARN_CAP);
        }
        if self.gossip_credit.len() > MAX_GOSSIP_SOURCES {
            self.gossip_credit.retain(|_, c| *c < GOSSIP_LEARN_CAP);
        }
    }

    /// APPREND un pair depuis une CARTE DE GOSSIP — source NON corroborée, donc durcie
    /// (chap. 8.1b, ferme D23). Trois gardes par rapport à `learn_peer` :
    ///   - **(a) PoW** sur l'id (hérité, via la garde ci-dessous) → pas de pollution gratuite ;
    ///   - **(b) jamais d'écrasement d'adresse** : pour un pair DÉJÀ connu, on ne touche PAS à
    ///     son adresse (le ouï-dire ne peut pas rediriger notre trafic vers une victime) — on
    ///     met juste à jour sa position (indice d'AoI, inoffensif) ;
    ///   - **(d) rate-limit par source** : `from` (l'expéditeur du paquet de gossip) ne peut
    ///     nous faire apprendre qu'`GOSSIP_LEARN_RATE` NOUVEAUX pairs/s.
    /// Renvoie `true` seulement si un INCONNU a été ajouté (→ l'appelant ouvrira un trou).
    pub(crate) fn learn_from_gossip(
        &mut self,
        from: SocketAddr,
        id: PeerId,
        addr: SocketAddr,
        pos: (f32, f32),
    ) -> bool {
        if id.is_none() || Some(id) == self.my_id || !id.has_pow(pow_bits()) {
            return false; // (a) identité nulle / soi-même / sans preuve de travail
        }
        if self.peers.contains_key(&id) {
            // (b) pair déjà connu : on n'écrase PAS son adresse (anti-redirection). La
            //     position n'est qu'un indice d'AoI → on peut la rafraîchir sans risque.
            self.peer_pos.insert(id, pos);
            return false;
        }
        if self.peers.len() >= MAX_KNOWN {
            return false; // table pleine (D16)
        }
        // (d) seau par source : un seul expéditeur ne peut pas nous faire apprendre une foule
        //     d'un coup (borne la réflexion + la pollution à la source).
        let credit = self.gossip_credit.entry(from).or_insert(GOSSIP_LEARN_CAP);
        if *credit < 1.0 {
            return false; // cet expéditeur a épuisé son budget d'apprentissage
        }
        *credit -= 1.0;
        self.peer_pos.insert(id, pos);
        self.peers.insert(id, addr);
        true
    }

    /// Note la dernière position (x, z) d'un pair (chap. 8.1), à chaque état accepté. **N'est
    /// appelé QUE sur un état SIGNÉ du pair lui-même** (cf. receive/bot, après `accept_seq`).
    /// Écrit donc à la fois `peer_pos` (indice AoI/gossip) ET `confirmed_pos` (position CORROBORÉE,
    /// chap. 9.4 : la seule base de confiance — le gossip ne l'atteint jamais).
    pub(crate) fn note_pos(&mut self, id: PeerId, xz: (f32, f32)) {
        self.peer_pos.insert(id, xz);
        self.confirmed_pos.insert(id, xz);
    }

    /// Met à jour l'ensemble FOCUS COLLANT (chap. 8.2a-bis) depuis notre position `my`.
    /// Le focus = les pairs à qui on tient un lien plein débit ; on le STABILISE :
    ///   1. on retire les membres qui ont quitté la table ;
    ///   2. on remplit les places libres avec les pairs connus les PLUS pertinents ;
    ///   3. on ne REMPLACE un membre que si un autre est `FOCUS_SWAP_MARGIN`× plus pertinent
    ///      (un seul échange par appel) → marge anti-oscillation, fin du churn du 8.2b.
    /// La pertinence vient de la dernière position CONNUE (`peer_pos`) ; un pair SANS position
    /// connue a pertinence 0 → il n'accapare PAS de slot de focus. C'est le DÉCOUPLAGE
    /// découverte/focus : un inconnu se fait entendre par la CONSCIENCE, pas en volant le plein débit.
    pub(crate) fn refresh_focus(&mut self, my: (f32, f32)) {
        // Pertinence par pair connu (snapshot local → pas de double emprunt de self).
        let rel: HashMap<PeerId, f32> = self
            .peers
            .keys()
            .map(|id| {
                let r = self.peer_pos.get(id).map(|&p| relevance_weight(dist2(my, p))).unwrap_or(0.0);
                (*id, r)
            })
            .collect();
        let rel_of = |id: &PeerId| rel.get(id).copied().unwrap_or(0.0);

        // 1) on retire les membres partis (et l'identité nulle par précaution).
        self.focus.retain(|id| self.peers.contains_key(id));

        // 2) on remplit les places libres avec les plus pertinents hors focus (position connue).
        if self.focus.len() < K_FOCUS {
            let mut cands: Vec<PeerId> = self
                .peers
                .keys()
                .filter(|id| !self.focus.contains(id) && rel_of(id) > 0.0)
                .copied()
                .collect();
            cands.sort_by(|a, b| rel_of(b).partial_cmp(&rel_of(a)).unwrap_or(std::cmp::Ordering::Equal));
            for id in cands {
                if self.focus.len() >= K_FOCUS {
                    break;
                }
                self.focus.push(id);
            }
        }

        // 3) UN échange hystérétique : le meilleur hors focus déloge le pire du focus seulement
        //    s'il est NETTEMENT plus pertinent (× FOCUS_SWAP_MARGIN) → pas de va-et-vient.
        if self.focus.len() == K_FOCUS {
            let worst = self
                .focus
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| rel_of(a).partial_cmp(&rel_of(b)).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, id)| (i, *id));
            let best = self
                .peers
                .keys()
                .filter(|id| !self.focus.contains(id))
                .max_by(|a, b| rel_of(a).partial_cmp(&rel_of(b)).unwrap_or(std::cmp::Ordering::Equal))
                .copied();
            if let (Some((wi, worst_id)), Some(best_id)) = (worst, best) {
                if rel_of(&best_id) > rel_of(&worst_id) * FOCUS_SWAP_MARGIN {
                    self.focus[wi] = best_id;
                }
            }
        }
    }

    /// Ce pair est-il actuellement au FOCUS (lien plein débit) ? (chap. 8.2a-bis)
    pub(crate) fn is_focus(&self, id: &PeerId) -> bool {
        self.focus.contains(id)
    }

    /// HÔTE d'une cellule (chap. 8.3) : élection DÉTERMINISTE = le plus petit id parmi les
    /// occupants connus de cette cellule (moi inclus si j'y suis). Même règle que la migration
    /// de l'orbe → tout le monde tombe sur le même hôte SANS vote. `me` = ma position, `my_id` =
    /// mon identité. Renvoie `None` si la cellule est vide (personne de connu, ni moi). On ne
    /// place un pair que si on connaît sa position (`peer_pos`) ET qu'il est encore dans la table.
    ///
    /// Contrairement à l'orbe, l'hôte n'est pas une AUTORITÉ : il ne fait que RÉSUMER sa région
    /// (8.3c). Donc un désaccord transitoire (deux hôtes) ne corrompt rien — juste un résumé
    /// redondant. C'est pourquoi cette élection simple suffit ici (pas besoin du quorum de D11).
    pub(crate) fn cell_host(&self, cell: (i32, i32), me: (f32, f32), my_id: PeerId) -> Option<PeerId> {
        let mut host: Option<PeerId> = if cell_of(me.0, me.1) == cell { Some(my_id) } else { None };
        for (id, pos) in &self.peer_pos {
            if self.peers.contains_key(id) && cell_of(pos.0, pos.1) == cell {
                host = Some(match host {
                    Some(h) => h.min(*id),
                    None => *id,
                });
            }
        }
        host
    }

    /// Suis-je l'hôte de MA propre cellule (chap. 8.3) ? `me` = ma position.
    pub(crate) fn am_i_cell_host(&self, me: (f32, f32), my_id: PeerId) -> bool {
        self.cell_host(cell_of(me.0, me.1), me, my_id) == Some(my_id)
    }

    /// Construit le RÉSUMÉ de MA cellule (chap. 8.3c) — SEULEMENT si j'en suis l'hôte. Occupants =
    /// les pairs CONNUS (position dans `peer_pos`) qui tombent dans ma cellule, plus MOI. Le `count`
    /// reflète toute la foule de la cellule ; l'échantillon en donne quelques positions. `None` si
    /// je ne suis pas l'hôte (un seul nœud résume → pas de cacophonie). *(Trust : le count s'appuie
    /// sur `peer_pos`, qui inclut du gossip non corroboré → un hôte peut sur/sous-estimer ; résumé
    /// CONSULTATIF, corroboration = 8.8.)*
    pub(crate) fn build_my_cell_summary(&self, me: (f32, f32), my_id: PeerId, ts: u64) -> Option<CellSummary> {
        if !self.am_i_cell_host(me, my_id) {
            return None;
        }
        let my_cell = cell_of(me.0, me.1);
        let mut occupants: Vec<(f32, f32)> = vec![me]; // moi, l'hôte, je suis dans ma cellule
        for (id, pos) in &self.peer_pos {
            if self.peers.contains_key(id) && cell_of(pos.0, pos.1) == my_cell {
                occupants.push(*pos);
            }
        }
        Some(build_cell_summary(my_cell, &occupants, ts))
    }

    /// INGÈRE un résumé de cellule reçu (chap. 8.3c, durci 8.3d) : on ne retient que le PLUS FRAIS
    /// par cellule (`ts` strictement plus grand). C'est l'anti-rejeu des résumés, jumeau de
    /// `accept_seq` pour les états : une VIEILLE copie partielle qui circule encore via les relais
    /// (count faible, d'avant que l'hôte connaisse toute la foule) ne peut PLUS écraser la fraîche et
    /// complète — le bug de 8.3c (la perception EMPIRAIT à fenêtre longue). Borné par `MAX_CELLS`
    /// (anti-inondation). Renvoie `true` si on a ACCEPTÉ (cellule nouvelle OU plus fraîche) → vaut la
    /// peine d'être relayé ; `false` si rejeté (périmé, ou table pleine).
    pub(crate) fn ingest_summary(&mut self, s: CellSummary) -> bool {
        match self.cell_summaries.get(&s.cell) {
            Some(existing) => {
                if s.ts <= existing.ts {
                    return false; // pas plus frais → la vieille copie ne peut plus écraser
                }
                self.cell_summaries.insert(s.cell, s);
                true
            }
            None => {
                if self.cell_summaries.len() >= MAX_CELLS {
                    return false; // table pleine : on borne la mémoire
                }
                self.cell_summaries.insert(s.cell, s);
                true
            }
        }
    }

    /// Métrique (chap. 8.3) : la foule TOTALE qu'on perçoit via les résumés de cellule détenus =
    /// SOMME des occupants sur toutes les cellules résumées qu'on suit. Une foule dense étalée sur
    /// plusieurs cellules (ex. à cheval sur un coin de grille) est perçue par la somme de SES
    /// cellules. Si ≈ la taille de la foule à portée, l'invariant tient : on voit toute la foule via
    /// QUELQUES flux résumés (O(cellules)), pas N états au compte-gouttes 1/N de la conscience.
    pub(crate) fn summary_perceived(&self) -> u32 {
        self.cell_summaries.values().map(|s| s.count).sum()
    }

    /// Score de fautes COURANT d'un pair (après décroissance) à l'instant `now` (chap. 9.3).
    fn strike_score_at(&self, id: PeerId, now: Instant) -> f32 {
        match self.strikes.get(&id) {
            Some(s) => decayed_score(s.score, now.duration_since(s.last).as_secs_f32()),
            None => 0.0,
        }
    }

    /// Ce pair est-il en sourdine (trop de fautes RÉCENTES) ? Si oui, on ignore ses paquets.
    /// Le score décroît avec le temps (9.3) → une faute transitoire finit par se dissiper.
    pub(crate) fn is_muted(&self, id: PeerId) -> bool {
        self.is_muted_at(id, Instant::now())
    }

    /// Variante testable de `is_muted` (chap. 9.3) : on injecte `now` pour vérifier la
    /// réhabilitation sans dormir, et SANS arithmétique `Instant` risquée (on avance `now`).
    fn is_muted_at(&self, id: PeerId, now: Instant) -> bool {
        self.strike_score_at(id, now) >= MAX_STRIKES as f32
    }

    /// Compte les pairs ACTUELLEMENT en sourdine (score décru ≥ seuil) — pour la simu (chap. 9.3).
    pub(crate) fn muted_count(&self) -> usize {
        let now = Instant::now();
        self.strikes
            .values()
            .filter(|s| decayed_score(s.score, now.duration_since(s.last).as_secs_f32()) >= MAX_STRIKES as f32)
            .count()
    }

    /// Inscrit une faute (chap. 5.4, à décroissance 9.3) : on décroît d'abord le score existant,
    /// puis on ajoute 1, et on date la faute. Quand le seuil est franchi (et qu'on ne l'était pas
    /// déjà), on l'annonce : le pair est ignoré (banni localement, mais RÉVERSIBLE dans le temps).
    pub(crate) fn add_strike(&mut self, id: PeerId, reason: &str) {
        let now = Instant::now();
        let was_muted = self.is_muted_at(id, now);
        let score = self.strike_score_at(id, now) + 1.0; // décroît avant d'incrémenter
        self.strikes.insert(id, Strike { score, last: now });
        if score >= MAX_STRIKES as f32 {
            if !was_muted {
                eprintln!("🛡 Pair {} mis en SOURDINE (dernière : {reason}).", id.short());
            }
        } else {
            eprintln!("🛡 Faute de {} ({score:.0}/{MAX_STRIKES}) : {reason}.", id.short());
        }
    }

    /// Inflige une faute ET, si elle vient de faire passer le tricheur en sourdine,
    /// DIFFUSE une accusation signée à nos voisins (réputation partagée, chap. 6.7).
    /// À utiliser partout où l'on sanctionne une triche ATTRIBUABLE.
    pub(crate) fn punish(&mut self, offender: PeerId, reason: &str) {
        self.add_strike(offender, reason);
        // `insert` renvoie true la PREMIÈRE fois : on n'accuse qu'une fois par tricheur.
        if self.is_muted(offender) && self.accused_broadcast.insert(offender) {
            let bytes = encode_accuse(offender, &self.identity);
            for addr in self.peers.values() {
                let _ = self.socket.send_to(*addr, &bytes);
            }
        }
    }

    /// POIDS DE CRÉDIBILITÉ d'un accusateur (chap. 9.2) — le cœur du quorum pondéré. Répond à :
    /// « cette accusation mérite-t-elle de compter, et combien ? ». Deux facteurs :
    ///   - **STANDING** : l'accusateur m'a-t-il déjà envoyé un VRAI état signé que j'ai accepté
    ///     (entrée dans `replay`) ? Sinon c'est une identité qui SURGIT juste pour accuser (un Sybil
    ///     conjuré, cf. `attack sybil-frame`) → **poids 0**. C'est LE verrou qui ferme le framing
    ///     bon marché : peser exige d'avoir réellement participé au monde, pas juste miné une clé.
    ///   - **CO-LOCALISATION CORROBORÉE (chap. 9.4)** : si je connais les positions CONFIRMÉES
    ///     (`confirmed_pos`, issues des états SIGNÉS) de l'accusateur ET de l'accusé et qu'elles
    ///     sont à portée (`WITNESS_RADIUS`), il a pu VOIR la triche → poids plein (1.0) ; sinon
    ///     poids plancher (`WITNESS_FLOOR`). **On n'utilise PAS `peer_pos`** : sa part « gossip » est
    ///     revendiquée par un tiers → un attaquant pourrait coller un témoin sur la victime pour
    ///     fabriquer une fausse co-localisation. La position de confiance vient du pair LUI-MÊME.
    /// (Résidu honnête : un attaquant patient qui fait VIVRE ses Sybils gagne du standing ET peut
    /// les déplacer RÉELLEMENT près de la victime → durci par 9.2c (standing par durée) + diversité
    /// de voisinage 9.4b.)
    fn accusation_weight(&self, accuser: PeerId, offender_pos: Option<(f32, f32)>) -> f32 {
        if !self.replay.contains_key(&accuser) {
            return 0.0; // jamais entendu un état de lui → témoin non crédible (Sybil conjuré)
        }
        match (self.confirmed_pos.get(&accuser).copied(), offender_pos) {
            (Some(a), Some(o)) if dist2(a, o) <= WITNESS_RADIUS * WITNESS_RADIUS => 1.0,
            _ => WITNESS_FLOOR, // établi mais co-localisation non corroborée/lointaine → poids réduit
        }
    }

    /// Enregistre une accusation REÇUE (`accuser` accuse `offender`). Renvoie `true`
    /// si le POIDS CUMULÉ des accusateurs crédibles vient de franchir `ACCUSE_WEIGHT_QUORUM`
    /// → on bannit `offender` à notre tour, même sans l'avoir vu tricher. On ne RE-diffuse PAS
    /// (pas de cascade) : la réputation se propage à un saut des témoins directs.
    ///
    /// **Anti-framing pondéré (chap. 9.2) :** on ne compte plus des têtes (frameable par des Sybils
    /// bon marché, cf. `attack sybil-frame`) — on somme `accusation_weight` sur les accusateurs
    /// DISTINCTS. Un Sybil conjuré (sans standing) pèse 0 → un essaim d'identités fraîches ne fait
    /// plus taire un innocent. La position de l'accusé sert à juger la co-localisation des témoins.
    pub(crate) fn record_accusation(&mut self, offender: PeerId, accuser: PeerId) -> bool {
        self.accusations.entry(offender).or_default().insert(accuser);
        if self.is_muted(offender) {
            return false;
        }
        let off_pos = self.confirmed_pos.get(&offender).copied(); // position de confiance (9.4a)
        // Snapshot des accusateurs (évite le double emprunt de self pendant le calcul de poids).
        let accusers: Vec<PeerId> = self.accusations[&offender].iter().copied().collect();
        // 9.4b (ANTI-ÉCLIPSE) : on CAPE la contribution par SOUS-RÉSEAU. Un attaquant peut miner
        // mille Sybils, il n'a qu'une poignée d'IP → tous ses témoins co-localisés derrière un même
        // /24 comptent pour UN seul (≤ 1.0). Le quorum (3.0) exige donc des témoins de ≥3 RÉSEAUX
        // distincts. (Diversité d'id « façon Kademlia » serait inutile ici : les ids PoW aléatoires
        // se répartissent comme les honnêtes — c'est l'IP, rare, qui distingue l'attaquant.)
        let mut by_subnet: HashMap<(u8, u8, u8, u16), f32> = HashMap::new();
        for a in &accusers {
            let w = self.accusation_weight(*a, off_pos);
            if w <= 0.0 {
                continue;
            }
            // Adresse connue → clé /24 (ou loopback distinct par port) ; inconnue → clé propre à
            // l'id (on ne fusionne pas un témoin dont on ignore le réseau).
            let key = match self.peers.get(a) {
                Some(addr) => subnet_key(*addr),
                None => {
                    let b = a.bytes();
                    (b[0], b[5], b[31], 0xFFFF)
                }
            };
            let e = by_subnet.entry(key).or_insert(0.0);
            *e = (*e + w).min(1.0); // un sous-réseau = au plus 1 témoin effectif
        }
        let weight: f32 = by_subnet.values().sum();
        if weight >= ACCUSE_WEIGHT_QUORUM {
            self.strikes.insert(offender, Strike { score: MAX_STRIKES as f32, last: Instant::now() }); // force la sourdine (décroît ensuite comme les autres, 9.3)
            eprintln!("🛡 Pair {} mis en SOURDINE par QUORUM pondéré (poids {weight:.1} ≥ {ACCUSE_WEIGHT_QUORUM:.0}).", offender.short());
            return true;
        }
        false
    }

    /// ANTI-REJEU (durci au 7.3) : accepte un `seq` s'il est NEUF dans la fenêtre
    /// glissante de ce pair (et le mémorise). Tolère le ré-ordonnancement réseau ;
    /// refuse les vrais rejeus (seq déjà vu) et les paquets trop vieux (hors fenêtre).
    /// Le premier paquet d'un pair (fenêtre vierge) initialise la fenêtre et est accepté.
    pub(crate) fn accept_seq(&mut self, id: PeerId, seq: u64) -> bool {
        match self.replay.get_mut(&id) {
            Some(w) => w.accept(seq),
            None => {
                self.replay.insert(id, ReplayWindow::new(seq));
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link_de_test() -> NetLink {
        NetLink::new((1.0, 1.0, 1.0), false).expect("socket de test")
    }

    fn pid(seed: u8) -> PeerId {
        PeerId::from_bytes([seed; 32])
    }

    /// Un PeerId qui SATISFAIT la preuve de travail (2 octets de tête à zéro → ≥16 bits),
    /// distinct par `tag`. Évite de miner une vraie clé dans les tests (coûteux).
    fn pid_pow(tag: u16) -> PeerId {
        let mut b = [0u8; 32];
        b[2] = 1; // octets 0 et 1 à zéro → has_pow(16) garanti ; b[2]≠0 → jamais l'id nul
        b[4] = (tag >> 8) as u8;
        b[5] = tag as u8;
        PeerId::from_bytes(b)
    }

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /// 8.3a — l'élection d'hôte de cellule : plus petit id parmi les occupants connus de la
    /// cellule (moi inclus), DÉTERMINISTE, et qui ignore les pairs d'AUTRES cellules. Cellule
    /// vide → None.
    #[test]
    fn cell_host_est_le_plus_petit_id_de_la_cellule() {
        let mut link = link_de_test();
        let me_pos = (1.0, 1.0); // cellule (0,0)
        // pid(5) et pid(2) dans MA cellule ; pid(1) (plus petit) AILLEURS (cellule lointaine).
        link.peers.insert(pid(5), addr(7005));
        link.peer_pos.insert(pid(5), (2.0, 2.0)); // (0,0)
        link.peers.insert(pid(2), addr(7002));
        link.peer_pos.insert(pid(2), (3.0, 1.0)); // (0,0)
        link.peers.insert(pid(1), addr(7001));
        link.peer_pos.insert(pid(1), (200.0, 200.0)); // cellule lointaine → ne compte pas ici
        let me = pid(9);
        // Dans (0,0) : occupants {pid9(moi), pid5, pid2} → plus petit = pid2.
        assert_eq!(link.cell_host(cell_of(me_pos.0, me_pos.1), me_pos, me), Some(pid(2)));
        assert!(!link.am_i_cell_host(me_pos, me)); // pid2 < pid9 → ce n'est pas moi
        // La cellule lointaine de pid1 : son seul occupant connu = pid1 → c'est lui l'hôte.
        let loin = cell_of(200.0, 200.0);
        assert_eq!(link.cell_host(loin, me_pos, me), Some(pid(1)));
        // Une cellule vide n'a pas d'hôte.
        assert_eq!(link.cell_host((50, 50), me_pos, me), None);
    }

    /// 8.3d — l'ingestion ne garde que le PLUS FRAIS par cellule : une vieille copie partielle
    /// (count faible, ts ancien) qui circule encore via les relais ne peut PLUS écraser la fraîche
    /// et complète (count élevé, ts récent). C'est le bug de 8.3c (la perception EMPIRAIT à fenêtre
    /// longue) refermé. Jumeau de l'anti-rejeu `accept_seq`.
    #[test]
    fn ingest_garde_le_plus_frais_pas_le_dernier_arrive() {
        let mut link = link_de_test();
        let fraiche = CellSummary { cell: (0, 0), count: 190, ts: 1000, samples: vec![] };
        let vieille = CellSummary { cell: (0, 0), count: 50, ts: 200, samples: vec![] };
        // J'ingère d'abord la FRAÎCHE (complète).
        assert!(link.ingest_summary(fraiche.clone())); // nouvelle cellule → acceptée
        assert_eq!(link.summary_perceived(), 190);
        // Une VIEILLE copie arrive ENSUITE (relais en retard) : refusée, ne corrompt pas.
        assert!(!link.ingest_summary(vieille)); // périmée → rejetée
        assert_eq!(link.summary_perceived(), 190); // la fraîche tient
        // Une copie ENCORE plus fraîche (l'hôte a appris plus de monde) : acceptée, remplace.
        let plus_fraiche = CellSummary { cell: (0, 0), count: 198, ts: 1500, samples: vec![] };
        assert!(link.ingest_summary(plus_fraiche));
        assert_eq!(link.summary_perceived(), 198);
    }

    /// 8.2a-bis — le focus est COLLANT : il prend les plus proches, NE bouge PAS sous un petit
    /// bruit de position (fin du churn du 8.2b), mais accepte un pair NETTEMENT plus pertinent.
    #[test]
    fn focus_est_collant_pas_de_churn() {
        let mut link = link_de_test();
        let me = (0.0, 0.0);
        // 20 pairs à distances croissantes : pid(1) le plus proche … pid(20) le plus loin.
        for i in 1..=20u8 {
            link.peers.insert(pid(i), addr(7000 + i as u16));
            link.peer_pos.insert(pid(i), (i as f32, 0.0));
        }
        link.refresh_focus(me);
        let f0 = link.focus.clone();
        assert_eq!(f0.len(), K_FOCUS);
        for i in 1..=K_FOCUS as u8 {
            assert!(link.is_focus(&pid(i))); // les K_FOCUS plus proches
        }

        // Petit bruit de position (< marge) → focus STRICTEMENT inchangé (pas de churn).
        for i in 1..=20u8 {
            link.peer_pos.insert(pid(i), (i as f32 + 0.1, 0.0));
        }
        link.refresh_focus(me);
        assert_eq!(link.focus, f0);

        // Un lointain devient TRÈS proche (au-delà de la marge) → il entre au focus.
        link.peer_pos.insert(pid(20), (0.01, 0.0));
        link.refresh_focus(me);
        assert!(link.is_focus(&pid(20)));
    }

    /// 8.1b (a) — une carte de gossip SANS preuve de travail est IGNORÉE (pas de
    /// pollution gratuite de table). Idem pour `learn_peer` (rendez-vous menteur, D10).
    #[test]
    fn gossip_rejette_id_sans_pow() {
        let mut link = link_de_test();
        let sans_pow = pid(7); // 0x07… → 5 bits de tête à zéro < 16 → has_pow(16) faux
        assert!(!sans_pow.has_pow(pow_bits()));
        assert!(!link.learn_from_gossip(addr(9000), sans_pow, addr(5001), (0.0, 0.0)));
        assert!(!link.learn_peer(sans_pow, addr(5001), None));
        assert!(link.peers.is_empty()); // rien appris
    }

    /// 8.1b (b) — le gossip n'ÉCRASE JAMAIS l'adresse d'un pair déjà connu (anti-redirection
    /// vers une victime). On apprend un pair par voie corroborée, puis une carte de gossip
    /// prétend une AUTRE adresse pour lui : l'adresse en table ne bouge pas.
    #[test]
    fn gossip_n_ecrase_pas_l_adresse_d_un_pair_connu() {
        let mut link = link_de_test();
        let p = pid_pow(1);
        assert!(link.learn_peer(p, addr(5001), None)); // corroboré (WELCOME) → adresse de confiance
        // Carte de gossip menteuse : « p est à 6666 » (= adresse d'une victime).
        assert!(!link.learn_from_gossip(addr(9000), p, addr(6666), (1.0, 2.0)));
        assert_eq!(link.peers.get(&p), Some(&addr(5001))); // adresse INCHANGÉE
    }

    /// 8.1b (d) — une seule source de gossip ne peut nous faire apprendre qu'un nombre BORNÉ
    /// de nouveaux pairs (seau par source) : un attaquant ne peut pas nous faire percer une
    /// foule de victimes d'un coup. Sans recharge, le seau démarre plein (`GOSSIP_LEARN_CAP`).
    #[test]
    fn gossip_rate_limite_l_apprentissage_par_source() {
        let mut link = link_de_test();
        let source = addr(9000);
        let mut appris = 0usize;
        for i in 0..(GOSSIP_LEARN_CAP as u16 + 50) {
            if link.learn_from_gossip(source, pid_pow(i + 1), addr(7000 + i), (0.0, 0.0)) {
                appris += 1;
            }
        }
        assert_eq!(appris, GOSSIP_LEARN_CAP as usize); // plafonné, pas tout appris
        // Après recharge, la source récupère un peu de budget (apprentissage honnête possible).
        link.recharge_gossip_credit(1.0);
        assert!(link.learn_from_gossip(source, pid_pow(9999), addr(8999), (0.0, 0.0)));
    }

    /// ANTI-REJEU : on accepte des seq croissants, on refuse tout rejeu / paquet périmé.
    #[test]
    fn accept_seq_refuse_les_rejeus() {
        let mut link = link_de_test();
        let a = pid(3);
        assert!(link.accept_seq(a, 1)); // premier paquet de a : accepté
        assert!(link.accept_seq(a, 2)); // strictement croissant : accepté
        assert!(!link.accept_seq(a, 2)); // même seq rejoué : refusé
        assert!(!link.accept_seq(a, 1)); // seq plus ancien : refusé
        assert!(link.accept_seq(a, 5)); // on saute en avant : accepté
        // Un AUTRE pair a son propre compteur, indépendant.
        assert!(link.accept_seq(pid(4), 1));
    }

    /// ANTI-REJEU À FENÊTRE GLISSANTE (7.3) : un paquet honnête arrivé EN RETARD (mais
    /// jamais vu, dans la fenêtre) est accepté — c'est ce que l'ancien anti-rejeu strict
    /// jetait à tort (bug du 7.2). Le rejeu et le trop-vieux restent refusés.
    #[test]
    fn accept_seq_tolere_le_reordonnancement() {
        let mut link = link_de_test();
        let a = pid(7);
        assert!(link.accept_seq(a, 10)); // premier paquet : fenêtre sur top=10
        assert!(link.accept_seq(a, 12)); // 12 arrive avant 11 (ré-ordo) : accepté, top=12
        assert!(link.accept_seq(a, 11)); // 11 arrive EN RETARD, dans la fenêtre : ACCEPTÉ
                                         //   (avant le 7.3, c'était refusé → perte honnête)
        assert!(!link.accept_seq(a, 11)); // rejouer 11 : refusé (déjà vu)
        assert!(!link.accept_seq(a, 12)); // rejouer 12 : refusé (déjà vu)
        assert!(!link.accept_seq(a, 10)); // rejouer 10 : refusé (déjà vu)
        // Un seq jamais vu mais TROP VIEUX (hors fenêtre de 64) reste refusé : on saute
        // loin en avant, puis on tente un seq distant de plus de 64.
        assert!(link.accept_seq(a, 200)); // top = 200
        assert!(!link.accept_seq(a, 100)); // 200 − 100 = 100 ≥ 64 → trop vieux → refusé
        assert!(link.accept_seq(a, 180)); // 200 − 180 = 20 < 64, jamais vu → accepté
    }

    /// RÉPUTATION : au bout de MAX_STRIKES fautes, le pair passe en sourdine.
    #[test]
    fn mute_apres_max_strikes() {
        let mut link = link_de_test();
        let tricheur = pid(9);
        assert!(!link.is_muted(tricheur));
        for _ in 0..MAX_STRIKES {
            link.add_strike(tricheur, "test");
        }
        assert!(link.is_muted(tricheur)); // banni localement
        // Un pair sans faute reste audible.
        assert!(!link.is_muted(pid(10)));
    }

    /// 9.3 — la décroissance du score : pleine au temps 0, diminue avec le temps, plancher à 0.
    #[test]
    fn score_de_fautes_decroit_avec_le_temps() {
        assert_eq!(decayed_score(5.0, 0.0), 5.0); // tout de suite : intact
        assert!(decayed_score(5.0, STRIKE_DECAY_SECS) < 5.0); // une faute dissipée
        assert!((decayed_score(5.0, STRIKE_DECAY_SECS) - 4.0).abs() < 1e-3);
        assert_eq!(decayed_score(1.0, 10_000.0), 0.0); // jamais négatif
    }

    /// 9.3 (RÉHABILITATION, D8) : un pair muté redevient audible après assez de temps SANS
    /// récidive ; mais un RÉCIDIVISTE (qui re-fauve avant l'expiration) reste muet. On injecte
    /// `now` (via `is_muted_at`) pour vérifier sans dormir.
    #[test]
    fn rehabilitation_apres_decroissance_mais_pas_le_recidiviste() {
        let mut link = link_de_test();
        let id = pid(42);
        let t0 = Instant::now();
        // 5 fautes « maintenant » → muté.
        link.strikes.insert(id, Strike { score: 5.0, last: t0 });
        assert!(link.is_muted_at(id, t0));
        // Après ~5 × le délai sans récidive : score décru à 0 → RÉHABILITÉ.
        let plus_tard = t0 + std::time::Duration::from_secs_f32(STRIKE_DECAY_SECS * 5.0 + 1.0);
        assert!(!link.is_muted_at(id, plus_tard));
        // Un récidiviste : à mi-chemin il reste 2,5 de score ; s'il re-fauve, il repasse muet.
        let mi = t0 + std::time::Duration::from_secs_f32(STRIKE_DECAY_SECS * 2.5);
        assert!(!link.is_muted_at(id, mi)); // score ~2.5 < 5 → audible…
        // …mais on ne le réhabilite PAS « gratuitement » : il a fallu attendre. (Garde-fou.)
        assert!(link.strike_score_at(id, mi) > 2.0);
    }

    /// Adresse de test sur un sous-réseau /24 DISTINCT par id (chap. 9.4b) : `10.b0.b5.1`, donc
    /// `subnet_key` = `(10, b0, b5, 0)`, unique pour des ids distincts. Évite que le cap par
    /// sous-réseau fusionne par erreur des témoins indépendants dans les tests.
    fn addr_for(id: PeerId) -> SocketAddr {
        let b = id.bytes();
        SocketAddr::from(([10, b[0], b[5], 1], 9000 + b[31] as u16))
    }

    /// Donne du STANDING à un pair pour les tests (chap. 9.2/9.4) : simule qu'on a déjà accepté un
    /// état SIGNÉ de lui → entrée `replay` (standing), position CORROBORÉE (`note_pos`), et une
    /// adresse sur un /24 distinct (`addr_for`, pour la diversité réseau 9.4b). Sans standing un
    /// accusateur pèse 0.
    fn donne_standing(link: &mut NetLink, id: PeerId, pos: (f32, f32)) {
        link.accept_seq(id, 1); // un état accepté → entrée dans `replay` = standing
        link.note_pos(id, pos); // état signé → position CORROBORÉE (9.4a)
        link.peers.insert(id, addr_for(id)); // adresse → sous-réseau distinct (9.4b)
    }

    /// RÉPUTATION PARTAGÉE PONDÉRÉE (6.7 + 9.2) : un quorum de TÉMOINS CRÉDIBLES (standing +
    /// co-localisés avec l'accusé) met en sourdine ; un accusateur répété ne compte qu'une fois.
    #[test]
    fn quorum_de_temoins_credibles_met_en_sourdine() {
        let mut link = link_de_test();
        let tricheur = pid(50);
        link.note_pos(tricheur, (0.0, 0.0)); // position CORROBORÉE de l'accusé (état signé, 9.4)
        for s in 1..=3u8 {
            donne_standing(&mut link, pid(s), (1.0, 1.0)); // 3 témoins établis, à portée
        }
        assert!(!link.record_accusation(tricheur, pid(1)));
        assert!(!link.record_accusation(tricheur, pid(2)));
        assert!(!link.record_accusation(tricheur, pid(2))); // doublon : ne compte pas
        assert!(!link.is_muted(tricheur)); // 2 témoins (poids 2.0) < seuil (3.0)
        assert!(link.record_accusation(tricheur, pid(3))); // 3e témoin crédible → poids 3.0 ≥ seuil
        assert!(link.is_muted(tricheur));
    }

    /// 9.2 (LE CORRECTIF, preuve unitaire de `sybil-frame`) : un quorum de Sybils CONJURÉS — des
    /// identités qui n'ont JAMAIS envoyé d'état (aucun standing) — ne fait PAS taire un innocent,
    /// même à 3, 10 ou 100. Avant 9.2, 3 suffisaient (compte de têtes). Maintenant ils pèsent 0.
    #[test]
    fn framing_par_sybils_sans_standing_echoue() {
        let mut link = link_de_test();
        let innocent = pid(50);
        link.note_pos(innocent, (0.0, 0.0)); // position corroborée de l'innocent
        // 100 accusateurs conjurés : connus de personne, jamais entendus (pas de `replay`).
        for s in 1..=100u8 {
            assert!(!link.record_accusation(innocent, pid(s)));
        }
        assert!(!link.is_muted(innocent)); // poids cumulé = 0 → l'innocent reste audible
        // En revanche, dès que le quorum d'accusateurs gagne standing + co-localisation, ils
        // PÈSENT : ils sont déjà dans l'ensemble, donc un seul ré-enregistrement recalcule le
        // poids cumulé (3 × 1.0 = 3.0 ≥ seuil) → sourdine. (Ce sont alors de VRAIS témoins.)
        for s in 1..=3u8 {
            donne_standing(&mut link, pid(s), (1.0, 1.0));
        }
        assert!(link.record_accusation(innocent, pid(1))); // le recalcul franchit le seuil
        assert!(link.is_muted(innocent));
    }

    /// 9.4a (corroboration des positions) : un attaquant PATIENT donne du standing à ses témoins
    /// (ils ont vraiment participé), mais leurs vraies positions CORROBORÉES sont LOIN de la
    /// victime. Il GOSSIPE alors une fausse position « collée » sur la victime (`learn_from_gossip`)
    /// pour fabriquer une co-localisation. Avant 9.4, la crédibilité lisait `peer_pos` (que le
    /// gossip pollue) → poids plein → framing. Depuis 9.4, elle lit `confirmed_pos` (états signés
    /// seulement) → co-localisation NON corroborée → poids plancher → l'innocent N'est PAS banni.
    #[test]
    fn gossip_ne_peut_pas_falsifier_la_co_localisation_pour_framer() {
        let mut link = link_de_test();
        let innocent = pid(50);
        link.note_pos(innocent, (0.0, 0.0)); // l'innocent est à l'origine (position corroborée)
        // 3 témoins avec PoW (sinon le gossip les rejetterait), ÉTABLIS mais réellement LOIN de
        // l'innocent (position CORROBORÉE (500,500)).
        let temoins = [pid_pow(1), pid_pow(2), pid_pow(3)];
        for t in temoins {
            donne_standing(&mut link, t, (500.0, 500.0));
            // L'attaquant GOSSIPE « ce témoin est à (0,0) » (collé sur la victime) → pollue VRAIMENT
            // peer_pos (le témoin n'est pas encore dans peers). Si la crédibilité lisait peer_pos,
            // le framing marcherait — ce test échouerait. Elle lit confirmed_pos → il échoue.
            link.learn_from_gossip(addr(9000), t, addr(7001), (0.0, 0.0));
            assert_eq!(link.peer_pos.get(&t), Some(&(0.0, 0.0))); // peer_pos EST bien pollué…
            assert_eq!(link.confirmed_pos.get(&t), Some(&(500.0, 500.0))); // …mais pas confirmed_pos
        }
        // confirmed_pos (500,500) ≠ (0,0) → chaque témoin pèse le PLANCHER (0.34), pas 1.0.
        // 3 × 0.34 = 1.02 < 3.0 → pas de sourdine : la fausse co-localisation par gossip est INOPÉRANTE.
        for t in temoins {
            link.record_accusation(innocent, t);
        }
        assert!(!link.is_muted(innocent));
    }

    /// 9.4b (anti-éclipse) : même avec standing ET co-localisation RÉELLE (le résidu coûteux de
    /// 9.4a), des Sybils derrière UNE SEULE IP /24 sont capés à 1 témoin effectif → pas de quorum.
    /// Le contraste (mêmes témoins sur des /24 DISTINCTS) bannit bien → on ne casse pas l'honnête.
    #[test]
    fn sybils_d_un_meme_sous_reseau_ne_font_pas_quorum() {
        let innocent = pid(50);
        // 5 Sybils crédibles (standing + co-localisés à l'origine, comme l'innocent).
        let sybils = [pid_pow(10), pid_pow(11), pid_pow(12), pid_pow(13), pid_pow(14)];

        // (a) TOUS derrière le même /24 (203.0.113.x) → capés à 1.0 < 3.0 → l'innocent tient.
        let mut link = link_de_test();
        link.note_pos(innocent, (0.0, 0.0));
        for (k, s) in sybils.iter().enumerate() {
            link.accept_seq(*s, 1);
            link.note_pos(*s, (0.0, 0.0)); // RÉELLEMENT co-localisé (position corroborée)
            link.peers.insert(*s, SocketAddr::from(([203, 0, 113, 1], 6000 + k as u16))); // même /24
            link.record_accusation(innocent, *s);
        }
        assert!(!link.is_muted(innocent)); // 5 Sybils, 1 seule IP → 1 témoin effectif → pas de quorum

        // (b) Les MÊMES 5, mais sur 5 /24 distincts → 5 × 1.0 ≥ 3.0 → sourdine (témoins légitimes).
        let mut link2 = link_de_test();
        link2.note_pos(innocent, (0.0, 0.0));
        for (k, s) in sybils.iter().enumerate() {
            link2.accept_seq(*s, 1);
            link2.note_pos(*s, (0.0, 0.0));
            link2.peers.insert(*s, SocketAddr::from(([10, 20, k as u8, 1], 6000))); // /24 distincts
            link2.record_accusation(innocent, *s);
        }
        assert!(link2.is_muted(innocent)); // diversité réseau réelle → le quorum fonctionne
    }
}

/// Clé de DIVERSITÉ RÉSEAU d'une adresse (chap. 9.4b, anti-éclipse). Deux pairs du MÊME
/// sous-réseau /24 partagent une clé → ils comptent comme UNE source de confiance (un attaquant
/// derrière une poignée d'IP ne fabrique pas un quorum de Sybils, même en les co-localisant).
/// **Loopback (simu/dev) :** tous les nœuds ont 127.x mais un PORT distinct (vrais process
/// séparés) → on inclut le port pour NE PAS les fusionner, sinon la réputation légitime casserait
/// en simu localhost. Le /24 ne s'applique donc qu'aux vraies IP routables. *(Limite assumée : la
/// simu localhost ne peut pas exercer la diversité /24 ; le harnais NAT en namespaces, lui, le
/// pourrait — vraies IP distinctes. IPv6 non traité finement : repli sur adresse ~complète.)*
fn subnet_key(addr: SocketAddr) -> (u8, u8, u8, u16) {
    match addr {
        SocketAddr::V4(a) => {
            let o = a.ip().octets();
            if a.ip().is_loopback() {
                (o[0], o[1], o[2], addr.port()) // loopback : distinct par port (nœuds séparés)
            } else {
                (o[0], o[1], o[2], 0) // /24 : un attaquant derrière une IP partage cette clé
            }
        }
        SocketAddr::V6(a) => {
            let s = a.ip().segments();
            ((s[0] >> 8) as u8, s[0] as u8, (s[1] >> 8) as u8, addr.port())
        }
    }
}

/// Adresse du rendez-vous. Par défaut `127.0.0.1:4000` (tout sur le même PC) ;
/// surchargée par la variable d'environnement `RENDEZVOUS_ADDR` (ex.
/// `10.0.0.1:4000`) pour le test NAT en namespaces ou un vrai serveur distant.
pub(crate) fn rendezvous_addr() -> SocketAddr {
    if let Ok(s) = std::env::var("RENDEZVOUS_ADDR") {
        if let Ok(addr) = s.parse::<SocketAddr>() {
            return addr;
        }
        eprintln!("RENDEZVOUS_ADDR='{s}' illisible ; on retombe sur 127.0.0.1.");
    }
    SocketAddr::from(([127, 0, 0, 1], RENDEZVOUS_PORT))
}
