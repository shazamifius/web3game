//! LE LIEN : `NetLink`, la ressource Bevy qui tient le réseau d'un client.
//!
//! Présente uniquement en mode multijoueur. Elle contient la prise UDP, l'adresse
//! du rendez-vous, notre identifiant (attribué par le rendez-vous → `Option`
//! tant qu'on ne l'a pas), notre couleur, et l'ANNUAIRE des autres joueurs.

use super::accuse::encode_accuse;
use super::aoi::{cell_of, dist2, relevance_weight, FOCUS_SWAP_MARGIN, K_FOCUS};
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
            gossip_credit: HashMap::new(),
            focus: Vec::new(),
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
        if id.is_none() || Some(id) == self.my_id || !id.has_pow(POW_BITS) {
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
        if id.is_none() || Some(id) == self.my_id || !id.has_pow(POW_BITS) {
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

    /// Note la dernière position (x, z) d'un pair (chap. 8.1), à chaque état accepté.
    /// Sert à l'AoI et à fabriquer ses cartes de visite quand on le présente aux autres.
    pub(crate) fn note_pos(&mut self, id: PeerId, xz: (f32, f32)) {
        self.peer_pos.insert(id, xz);
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
    /// ⏸ 8.3 EN PAUSE (pivot ch.9, cf. `aoi::CELL_SIZE`) : élection posée et testée, pas encore
    /// utilisée par l'émission de résumés (8.3c) → `#[allow(dead_code)]` assumé jusqu'au câblage.
    #[allow(dead_code)]
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

    /// Suis-je l'hôte de MA propre cellule (chap. 8.3) ? `me` = ma position. ⏸ 8.3 en pause.
    #[allow(dead_code)]
    pub(crate) fn am_i_cell_host(&self, me: (f32, f32), my_id: PeerId) -> bool {
        self.cell_host(cell_of(me.0, me.1), me, my_id) == Some(my_id)
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
        assert!(!sans_pow.has_pow(POW_BITS));
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
