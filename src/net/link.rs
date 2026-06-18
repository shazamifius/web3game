//! LE LIEN : `NetLink`, la ressource Bevy qui tient le réseau d'un client.
//!
//! Présente uniquement en mode multijoueur. Elle contient la prise UDP, l'adresse
//! du rendez-vous, notre identifiant (attribué par le rendez-vous → `Option`
//! tant qu'on ne l'a pas), notre couleur, et l'ANNUAIRE des autres joueurs.

use super::accuse::encode_accuse;
use super::crypto::{Identity, PeerId, POW_BITS};
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use bevy::prelude::Resource;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;

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
    /// DERNIÈRE POSITION (x, z) connue de chaque pair (chap. 8.1). Alimentée par les
    /// états reçus ET par les cartes de gossip. Sert (a) à pondérer l'AoI avant même
    /// d'avoir reçu un état du pair, et (b) à fabriquer nos propres « cartes de visite »
    /// quand on présente ce pair à d'autres (gossip).
    pub(crate) peer_pos: HashMap<PeerId, (f32, f32)>,
    /// ANTI-REJEU (chap. 5.2, durci au 7.3) : une FENÊTRE GLISSANTE par pair (style
    /// IPsec/DTLS). On retient le plus grand `seq` vu + un masque des 64 derniers déjà
    /// acceptés → on tolère le ré-ordonnancement du réseau (un paquet en retard mais
    /// jamais vu, dans la fenêtre, passe) SANS rouvrir le rejeu (un seq déjà vu, ou trop
    /// vieux pour la fenêtre, est refusé).
    pub(crate) replay: HashMap<PeerId, ReplayWindow>,
    /// RÉPUTATION (chap. 5.4) : nombre de « fautes » constatées par pair (état
    /// impossible, orbe trichée…). Au-delà de `MAX_STRIKES`, le pair est mis en
    /// sourdine (« mute ») : on ignore tout ce qu'il envoie.
    pub(crate) strikes: HashMap<PeerId, u32>,
    /// RÉPUTATION PARTAGÉE (chap. 6.7) : pour chaque tricheur, l'ensemble des nœuds
    /// DISTINCTS qui l'ont accusé. Au-delà de `ACCUSE_QUORUM`, on le bannit aussi.
    pub(crate) accusations: HashMap<PeerId, HashSet<PeerId>>,
    /// Tricheurs pour lesquels on a DÉJÀ diffusé NOTRE accusation (une seule fois).
    pub(crate) accused_broadcast: HashSet<PeerId>,
    pub(crate) weak: bool, // faible upload : on émet notre état via un parent (relais) au lieu de tous
}

/// Nombre de fautes au-delà duquel on coupe le son d'un pair (réputation). Chaque
/// nœud est ainsi le « Shield » de ce qu'il observe : il détecte et bannit localement.
pub(crate) const MAX_STRIKES: u32 = 5;
/// Nombre d'accusateurs DISTINCTS requis avant de bannir un tricheur qu'on n'a PAS
/// vu soi-même (chap. 6.7). Anti-framing : un seul (ou quelques) menteur(s) ne suffit
/// pas ; et chaque identité coûte une preuve de travail (6.2), donc fabriquer un
/// quorum de fausses identités est coûteux.
pub(crate) const ACCUSE_QUORUM: usize = 3;

/// Plafond MÉMOIRE de la table de pairs connus (chap. 8.1, D22). Le gossip lève le
/// plafond de VISION (32) — on peut apprendre toute la foule — mais pas celui de la
/// MÉMOIRE : au-delà de `MAX_KNOWN` cartes, on cesse d'en ajouter (anti-inondation de
/// fausses cartes). Très large devant une foule réaliste ; l'éviction fine (TTL) est D16.
pub(crate) const MAX_KNOWN: usize = 4096;

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
        // On MINE notre identité (preuve de travail anti-Sybil, chap. 6.2) une fois,
        // au lancement. La privée reste ici. Ce petit coût rend une identité « chère »
        // → on ne peut plus se reconnecter gratuitement après un bannissement.
        let identity = Identity::generate_pow(POW_BITS);
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
            replay: HashMap::new(),
            strikes: HashMap::new(),
            accusations: HashMap::new(),
            accused_broadcast: HashSet::new(),
            weak,
        })
    }

    /// APPREND un pair (chap. 8.1) : depuis le WELCOME (amorçage) OU une carte de gossip.
    /// L'ajoute à la table s'il est nouveau, en RAFRAÎCHISSANT son adresse et sa position
    /// si fournie. Renvoie `true` SEULEMENT si c'était un INCONNU (→ l'appelant ouvrira un
    /// trou NAT vers lui). On ne s'apprend jamais soi-même, ni l'identité nulle.
    ///
    /// Borne MÉMOIRE (`MAX_KNOWN`) : la VISION n'est plus plafonnée (c'est tout l'intérêt
    /// du gossip contre D22), mais la mémoire SI — sinon un menteur nous noierait sous de
    /// fausses cartes. Table pleine → on refuse le nouveau (l'éviction fine par TTL est
    /// le chap. 12 / D16 ; ici on se contente de la borne dure).
    pub(crate) fn learn_peer(&mut self, id: PeerId, addr: SocketAddr, pos: Option<(f32, f32)>) -> bool {
        if id.is_none() || Some(id) == self.my_id {
            return false;
        }
        if let Some(xz) = pos {
            self.peer_pos.insert(id, xz);
        }
        if self.peers.contains_key(&id) {
            self.peers.insert(id, addr); // adresse rafraîchie, pas un nouveau
            return false;
        }
        if self.peers.len() >= MAX_KNOWN {
            return false; // table pleine : on borne la mémoire (D16)
        }
        self.peers.insert(id, addr);
        true
    }

    /// Note la dernière position (x, z) d'un pair (chap. 8.1), à chaque état accepté.
    /// Sert à l'AoI et à fabriquer ses cartes de visite quand on le présente aux autres.
    pub(crate) fn note_pos(&mut self, id: PeerId, xz: (f32, f32)) {
        self.peer_pos.insert(id, xz);
    }

    /// Ce pair est-il en sourdine (trop de fautes) ? Si oui, on ignore ses paquets.
    pub(crate) fn is_muted(&self, id: PeerId) -> bool {
        self.strikes.get(&id).copied().unwrap_or(0) >= MAX_STRIKES
    }

    /// Inscrit une faute au compteur d'un pair et la journalise. Quand le seuil est
    /// franchi, on l'annonce : le pair est désormais ignoré (banni localement).
    pub(crate) fn add_strike(&mut self, id: PeerId, reason: &str) {
        let n = self.strikes.entry(id).or_insert(0);
        *n += 1;
        if *n == MAX_STRIKES {
            eprintln!("🛡 Pair {} mis en SOURDINE après {n} fautes (dernière : {reason}).", id.short());
        } else if *n < MAX_STRIKES {
            eprintln!("🛡 Faute de {} ({n}/{MAX_STRIKES}) : {reason}.", id.short());
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

    /// Enregistre une accusation REÇUE (`accuser` accuse `offender`). Renvoie `true`
    /// si elle vient d'atteindre le QUORUM → on bannit `offender` à notre tour, même
    /// sans l'avoir vu tricher. On ne RE-diffuse PAS (pas de cascade) : la réputation
    /// se propage à un saut des témoins directs. Anti-framing : accusateurs DISTINCTS.
    pub(crate) fn record_accusation(&mut self, offender: PeerId, accuser: PeerId) -> bool {
        let set = self.accusations.entry(offender).or_default();
        set.insert(accuser);
        if set.len() >= ACCUSE_QUORUM && !self.is_muted(offender) {
            self.strikes.insert(offender, MAX_STRIKES); // force la sourdine
            eprintln!("🛡 Pair {} mis en SOURDINE par QUORUM ({} accusateurs).", offender.short(), ACCUSE_QUORUM);
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

    /// RÉPUTATION PARTAGÉE (6.7) : un quorum d'accusateurs DISTINCTS met en sourdine,
    /// même sans avoir vu le tricheur soi-même. Un accusateur qui se répète ne compte
    /// qu'une fois (anti-framing à un seul menteur).
    #[test]
    fn quorum_d_accusations_met_en_sourdine() {
        let mut link = link_de_test();
        let tricheur = pid(50);
        assert!(!link.record_accusation(tricheur, pid(1)));
        assert!(!link.record_accusation(tricheur, pid(2)));
        assert!(!link.record_accusation(tricheur, pid(2))); // doublon : ne compte pas
        assert!(!link.is_muted(tricheur)); // 2 distincts < quorum (3)
        assert!(link.record_accusation(tricheur, pid(3))); // 3e distinct → quorum
        assert!(link.is_muted(tricheur));
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
