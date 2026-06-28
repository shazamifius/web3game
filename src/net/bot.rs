//! LE BOT HEADLESS : un client qui fait tourner le VRAI code réseau, SANS la 3D.
//!
//! # Pourquoi ce fichier existe (chapitre 6.0)
//! On ne peut pas tester le jeu « en vrai » à grande échelle : les clients sont des
//! fenêtres graphiques. Ce bot rejoint le rendez-vous, perce les NAT, émet son état
//! SIGNÉ et applique à la réception EXACTEMENT les mêmes décisions de confiance que
//! le jeu (sceau auto-certifié, preuve de travail, anti-rejeu, réputation, contact
//! d'orbe, rate-limit). C'est aussi la BRIQUE de la simulation massive (chap. 6.8) :
//! un `Bot` = un nœud, et `cargo run -- sim` en lance des centaines en threads.
//!
//! Lancement (après `cargo run -- rendezvous`) :  cargo run -- bot alice

use super::accuse::decode_accuse;
use super::anticheat::move_plausible;
use super::aoi::{
    advertised_recv_cap, allocate_tiers, allocate_tiers_bilateral, dist2, relevance_weight, CELL_SIZE,
    RECV_BUDGET_HZ, SEND_BUDGET_HZ,
};
use super::control::{decode_welcome, encode_hello};
use super::crypto::{PeerId, pow_bits};
use super::gossip::{decode_gossip, encode_gossip, sample_cards};
use super::link::NetLink;
use super::message::{
    claimed_id, decode_canonical, decode_recv_budget, decode_state_bundle, encode_recv_budget,
    encode_relay_fwd, encode_signed, encode_state_bundle, sig_ok, PlayerState,
};
use super::orb::{apply_incoming, claimed_owner, decode_orb, orb_sig_ok, Orb, OrbApply};
use super::punch::{decode_punch, encode_punch, punch_abandoned, punch_retry_tries};
use super::skin::random_color;
use super::cell::{
    decode_cell_summary, decode_cell_summary_v2, encode_cell_summary, encode_cell_summary_v2, CellSummary,
};
use super::wire::{
    kind, KIND_ACCUSE, KIND_CELL_SUMMARY, KIND_CELL_SUMMARY_V2, KIND_GOSSIP, KIND_ORB, KIND_PUNCH,
    KIND_RECV_BUDGET, KIND_RELAY, KIND_STATE, KIND_STATE_BUNDLE, KIND_WELCOME, PROTO_VERSION,
};
use crate::math::Vec3;
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Le repli relais NAT est-il ACTIF ? (lu dans l'environnement, `RELAY_FALLBACK`.) Relogé ici
/// depuis l'ancien `netcode/send.rs` (le client Bevy retiré) : le `Bot` est désormais le seul à
/// décider d'émettre, donc le seul à lire ce drapeau.
pub(crate) fn relay_fallback_enabled() -> bool {
    relay_fallback_on(std::env::var("RELAY_FALLBACK").ok().as_deref())
}

/// Politique du drapeau (PURE, testable sans toucher l'environnement). Défaut sûr = OFF.
fn relay_fallback_on(v: Option<&str>) -> bool {
    matches!(v, Some("1") | Some("true"))
}

/// COUCHE 2 — l'AoI BILATÉRALE est-elle ACTIVE ? (`AOI_BILATERAL`, défaut **OFF** = byte-pour-byte.)
/// Quand ON, le nœud ANNONCE son budget de réception (`KIND_RECV_BUDGET`) et RESPECTE celui des
/// pairs (`allocate_tiers_bilateral`). OFF → `recv_caps` reste vide → comportement historique exact.
/// Gaté pour pouvoir embarquer le mécanisme dans la flotte sans changer le live tant qu'on ne l'allume
/// pas (la preuve live = un test de mesure dédié, on n'expose rien au hasard).
pub(crate) fn aoi_bilateral_enabled() -> bool {
    aoi_bilateral_on(std::env::var("AOI_BILATERAL").ok().as_deref())
}
/// Politique du drapeau couche 2 (PURE, testable). Défaut sûr = OFF.
fn aoi_bilateral_on(v: Option<&str>) -> bool {
    matches!(v, Some("1") | Some("true"))
}

/// REDONDANCE D'ÉMISSION SUR LE CHEMIN RELAIS (`RELAY_REDUNDANCY`, défaut **1** = byte-pour-byte).
/// Sur un relais lossy (4G/CGNAT, ~88 % de perte mesurée), émettre le MÊME état scellé `k` fois
/// back-to-back fait chuter la traîne p95 : à perte indépendante `p`, un trou ne survient que si les
/// `k` copies sont perdues (`p^k`). Le récepteur DÉDOUBLONNE nativement (le seq le plus récent gagne,
/// fenêtre de rejeu) → des copies n'ajoutent QUE de la robustesse, jamais d'effet de bord. Borné à 8
/// (le relais facture chaque copie au budget anti-amplification : au-delà, le rate-limit les jette).
pub(crate) fn relay_redundancy_from_env() -> usize {
    relay_redundancy_of(std::env::var("RELAY_REDUNDANCY").ok().as_deref())
}

/// Politique PURE (testable sans toucher l'environnement) : parse + borne à [1, 8] ; absent ou
/// invalide → 1 (inchangé = byte-pour-byte). Bornage haut = anti-abus du budget relais.
fn relay_redundancy_of(v: Option<&str>) -> usize {
    v.and_then(|s| s.parse::<usize>().ok()).unwrap_or(1).clamp(1, 8)
}

/// 12.3 (sidecar) — DÉCISION D'ÉMISSION par pair, **pure et testable**. C'est le portage EXACT
/// dans le `Bot` headless de la logique de [netcode/send.rs] : sans elle, le sidecar RECEVAIT un
/// état relayé mais ne RENVOYAIT jamais le sien → sens unique observé en RÉEL le 24 juin (A voit
/// le sidecar… non, l'inverse : le sidecar voit A, A ne voit rien). Les bancs sur `lo` ne l'ont
/// jamais vu car le perçage y réussit toujours (trou ouvert → branche relais jamais prise).
///
/// Défaut (`relay_fallback=false`, `force_relay=false`) : `Direct` si le trou est ouvert, sinon
/// `Skip` — comportement HISTORIQUE byte-pour-byte (le `Bot` n'émettait QUE vers les trous ouverts).
/// `relay_fallback=true` (agent CGNAT) : trou fermé → `Relay`, pour que le FILET CONSCIENCE atteigne
/// les pairs non-perçés (couche 1 inclusivité, 29 juin) — borné par l'anti-amplification du rendez-vous.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum SendKind {
    Direct,
    Relay,
    Skip,
}

pub(crate) fn bot_send_kind(
    open: bool,
    abandoned: bool,
    relays_to_us: bool,
    relay_fallback: bool,
    force_relay: bool,
) -> SendKind {
    // `abandoned`/`relays_to_us` ne décident plus (couche 1, 29 juin) — gardés dans la signature pour
    // la couche 2/3 (priorisation par budget du receveur). cf. le commentaire d'inclusivité ci-dessous.
    let _ = (abandoned, relays_to_us);
    // Banc déterministe : NAT infranchissable simulé → tout l'état part par le relais.
    if force_relay {
        return SendKind::Relay;
    }
    // Trou direct ouvert → on émet en direct (inchangé, byte-pour-byte).
    if open {
        return SendKind::Direct;
    }
    // INCLUSIVITÉ (couche 1) : trou fermé + repli relais ACTIF → on RELAIE, y compris le FILET
    // CONSCIENCE (2 Hz), pour qu'un pair CGNAT hors-focus reste VIVANT au lieu de tomber dans le noir
    // (le « bimodal » mesuré : plein débit OU silence). On ne fait PLUS dépendre ça d'un perçage
    // abandonné ni d'une réciprocité (qui CLIGNOTENT → c'était la source du silence). Borné par
    // l'anti-amplification du rendez-vous (RELAY_RATE = 30/s) → budget-driven, jamais un plafond en
    // dur ; au-delà du budget = champ de densité continu (couche 3, à venir).
    if relay_fallback {
        return SendKind::Relay;
    }
    SendKind::Skip
}

// Miroir des réglages de réception du jeu (cf. netcode/receive.rs et state.rs).
const BUCKET_RATE: f32 = 150.0;
const BUCKET_CAP: f32 = 300.0;
const MAX_BUCKETS: usize = 4096;
const RELAY_RATE: f32 = 30.0;
const RELAY_CAP: f32 = 60.0;
const MAX_RELAY_FANOUT: usize = 12;
const SEND_HZ: f32 = 20.0;
/// COUCHE 2 — période de (re)calcul + d'annonce du budget de réception (`KIND_RECV_BUDGET`), en s.
/// 1 s = on réévalue sa surcharge une fois par seconde (assez réactif, négligeable en débit).
const BUDGET_PERIOD: f32 = 1.0;
/// COUCHE 2 — durée de vie d'un plafond ANNONCÉ par un pair : passé ce délai sans nouvelle annonce,
/// on l'oublie (∞ = plus de bride). Le pair n'annonce QUE s'il est en surcharge → l'absence d'annonce
/// fraîche signifie « je vais bien, envoie normalement ». > BUDGET_PERIOD pour tolérer une annonce ratée.
const RECV_CAP_TTL: f32 = 3.0;
const HELLO_PERIOD: f32 = 1.0;
const PUNCH_PERIOD: f32 = 0.25;
/// DURCISSEMENT RELAIS (29 juin) — période de RE-SALVE du perçage des liens abandonnés : toutes les
/// 30 s, un lien abandonné au trou fermé re-tente une courte salve (cf. `punch_retry_tries`). Assez
/// rare pour que re-sonder un vrai NAT symétrique reste négligeable, assez fréquent pour sortir vite
/// un lien du relais lossy dès que le direct redevient possible (échec initial transitoire).
const PUNCH_RETRY_PERIOD: f32 = 30.0;
const SUMMARY_PERIOD: f32 = 2.0;
/// Période d'émission du gossip (chap. 8.1) : à chaque tic, on présente un lot de
/// « cartes de visite » à quelques pairs. 0,5 s = découverte rapide sans bavardage.
const GOSSIP_PERIOD: f32 = 0.5;
/// Nombre de destinataires (au trou ouvert) à qui on envoie notre lot de cartes par
/// tic. Petit : la diffusion épidémique couvre la foule en quelques rounds (log N).
const GOSSIP_FANOUT: usize = 4;
/// RÉSUMÉS DE CELLULE (chap. 8.3c) : période d'émission/relais, nombre de destinataires par tic,
/// et nombre max de résumés relayés par tic. Mêmes principes que le gossip (propagation épidémique
/// bornée) → l'hôte n'inonde personne (O(fanout), pas O(N) : ferme le piège « hôte qui fond », D4).
const CELL_SUMMARY_PERIOD: f32 = 2.0;
const CELL_SUMMARY_FANOUT: usize = 4;
const MAX_RELAY_SUMMARIES: usize = 8;
/// Nombre de PREUVES auto-signées (182 o) jointes à MON propre claim de cellule (chap. 8.3★ C-sécu-2,
/// gaté `SIGNED_SAMPLES`). Sous-ensemble TOURNANT → l'union vérifiée chez le receveur remonte au fil du
/// temps sans gonfler un paquet. Valeur de DÉPART du papier (réglable, mesuré étape 5) ; borne MTU :
/// 16 samples + 4×182 ≈ 1488 o, sous le MTU Ethernet ~1500. Ne JAMAIS monter sans re-mesurer le débit.
const K_PROOF: usize = 4;
/// Seuil (Hz) au-delà duquel on a entendu un pair « au FOCUS » (plein débit) plutôt qu'en
/// « conscience » (basse fidélité) — chap. 8.2b. Entre le plafond conscience (`CONSCIENCE_HZ` = 2)
/// et le plein débit (`SEND_HZ` = 20) : tout seuil intermédiaire sépare nettement les deux tiers.
const FOCUS_RATE_MIN: f32 = 5.0;
const TICK: Duration = Duration::from_millis(50);
const WANDER_RADIUS: f32 = 3.0;
/// Sans nouvel état d'un distant depuis ce délai (s) on cesse de l'exposer (miroir netcode/state.rs).
const REMOTE_TIMEOUT: f32 = 5.0;

/// Un nœud headless : tout l'état d'un client de jeu, SANS le rendu. Réutilisé par le
/// mode `bot` (un seul, bavard) et par la simulation `sim` (des centaines, silencieux).
pub(crate) struct Bot {
    label: String,
    verbose: bool,
    link: NetLink,
    holes: HashMap<PeerId, bool>,
    /// Nombre d'essais de perçage par pair non corroboré (chap. 8.1b) : au-delà de
    /// `PUNCH_GIVEUP` (cf. [punch.rs]) on abandonne, comme le vrai jeu → anti-réflexion.
    punch_tries: HashMap<PeerId, u32>,
    buckets: HashMap<SocketAddr, f32>,
    relay_credits: HashMap<PeerId, f32>,
    /// 12.3 (sidecar) — pairs qui nous joignent par le RELAIS du rendez-vous (état reçu FROM le
    /// rendez-vous, pas en direct) : on doit relayer NOTRE état en RETOUR (réciprocité). Vide par
    /// défaut → chemin direct intact.
    relays_to_us: HashSet<PeerId>,
    /// 12.3 (sidecar) — repli relais activé (`RELAY_FALLBACK`), lu UNE fois à la construction.
    /// `false` par défaut → le `Bot` n'émet jamais vers un trou fermé (byte-pour-byte historique).
    relay_fallback: bool,
    /// Banc déterministe — simule un NAT infranchissable (perçage jamais « ouvert » côté émission →
    /// tout passe par le relais), pour PROUVER le repli bidirectionnel sans vrai mobile. `false` par défaut.
    force_relay: bool,
    /// Redondance d'émission sur le chemin RELAIS (`RELAY_REDUNDANCY`, défaut 1 = inchangé). Initialisé
    /// depuis l'env à la construction, PUIS pilotable par session via [Bot::set_relay_redundancy]
    /// (campagne `redundancy=K`). Cf. `relay_redundancy_from_env` : écrase la traîne p95 sur lien lossy.
    relay_redundancy: usize,
    /// Anneau de NOS K derniers états signés (le plus ancien d'abord), pour le LOT relais
    /// (`KIND_STATE_BUNDLE`) : redondance temporelle budget-free. Borné à `relay_redundancy` (≤ 8).
    recent_self_states: VecDeque<Vec<u8>>,
    /// Crédits d'émission AoI par pair (chap. 7.4b) : même cadencement par water-filling
    /// que le vrai client ([netcode/send.rs]). Chaque pair accumule `débit × dt` ; à 1,
    /// on lui envoie un paquet. C'est ce qui fait que le bot mesure DÉSORMAIS le coût
    /// réel du jeu (budget réparti par pertinence), pas un envoi naïf plein débit à tous.
    send_credits: HashMap<PeerId, f32>,
    last_state: HashMap<PeerId, (Vec3, f32)>,
    /// MÉTRIQUE DE FIDÉLITÉ (chap. 8.2b) : combien d'états on a ACCEPTÉS de chaque pair sur
    /// la fenêtre de mesure. Sert à classer, à la fin, qui on a ENTENDU à plein débit (focus)
    /// vs en basse fidélité (conscience) — au lieu de juste compter les pairs CONNUS.
    heard_count: HashMap<PeerId, u64>,
    /// INSTRUMENT (agent) : journal des ARRIVÉES d'état par pair sur la fenêtre — `(recv_ms, seq)`.
    /// Lecture pure (peuplé aux points d'acceptation, drainé par l'agent à chaque fenêtre). Permet à
    /// `metrics::link_stats` de chiffrer perte/gigue/ré-ordre des VRAIS liens, pas juste la fraîcheur.
    /// Borné par pair (`MAX_ARRIVALS_PER_PEER`) → jamais de fuite mémoire sur longue session.
    link_arrivals: HashMap<PeerId, Vec<(f64, u64)>>,
    orb: Orb,
    seq: u64,
    hello_acc: f32,
    punch_acc: f32,
    /// DURCISSEMENT RELAIS — accumulateur de la période de RE-SALVE du perçage (`PUNCH_RETRY_PERIOD`).
    punch_retry_acc: f32,
    send_acc: f32,
    gossip_acc: f32,
    gossip_cursor: usize,
    cell_summary_acc: f32, // 8.3c : cadence d'émission/relais des résumés de cellule
    summary_cursor: usize, // 8.3c : curseur tournant pour relayer les résumés détenus
    proof_cursor: usize,   // 8.3★ C-sécu-2 : curseur tournant des preuves auto-signées de MON claim
    wander: f32,
    last_pos: Option<Vec3>,
    warned_version: bool,
    accepted: u64,
    rejected: u64,
    relayed: u64,
    /// SIDECAR (palier 2) : pose imposée de l'EXTÉRIEUR (Unreal). None = balade par défaut →
    /// chemin bot/simu strictement INCHANGÉ (byte-pour-byte).
    external_pose: Option<(Vec3, f32, f32)>,
    /// SIDECAR (palier 2) : puits des avatars distants COMPLETS (PlayerState + instant), pour les
    /// exposer à Unreal. None = désactivé (défaut) → aucune écriture, comportement inchangé.
    avatar_sink: Option<HashMap<PeerId, (PlayerState, f32)>>,
    /// COUCHE 2 — AoI BILATÉRALE active (`AOI_BILATERAL`, lu UNE fois). `false` par défaut → tout ce
    /// qui suit est inerte et l'émission reste byte-pour-byte historique.
    aoi_bilateral: bool,
    /// COUCHE 2 — plafonds de réception ANNONCÉS PAR les pairs : `id du pair → (cap Hz, instant `now`)`.
    /// On n'émet jamais vers un pair plus vite que `cap`. Entrée périmée (`> RECV_CAP_TTL`) = ignorée (∞).
    recv_caps: HashMap<PeerId, (f32, f32)>,
    /// COUCHE 2 — compteur d'états ACCEPTÉS depuis le dernier calcul de budget (→ Hz reçu mesuré).
    recv_in_window: u32,
    /// COUCHE 2 — accumulateur de la période d'annonce de budget (`BUDGET_PERIOD`).
    budget_acc: f32,
}

impl Bot {
    /// Crée un nœud : ouvre une prise, MINE son identité (preuve de travail, 6.2) et
    /// se prépare à rejoindre le rendez-vous. `None` si la prise ne s'ouvre pas.
    /// `phase` décale la position de départ de chaque nœud (pour étaler la « foule »).
    pub(crate) fn new(label: impl Into<String>, verbose: bool, phase: f32) -> Option<Bot> {
        let link = NetLink::new(random_color()).ok()?;
        Some(Bot::from_link(label, verbose, phase, link))
    }

    /// Comme `new`, mais avec une identité PERSISTANTE (clé `~/.web3game/<profil>.key`) — pour le
    /// sidecar : un nœud STABLE entre redémarrages (le pair distant le redécouvre vite, pas une
    /// nouvelle identité à chaque relance). Mine à `pow_bits()` au 1er lancement.
    pub(crate) fn new_persistent(label: impl Into<String>, profile: &str) -> Option<Bot> {
        let link = NetLink::new_persistent(random_color(), profile).ok()?;
        Some(Bot::from_link(label, false, 0.0, link))
    }

    /// Crée un bot sur un `NetLink` DÉJÀ construit (banc bus mémoire, dette D25) : même bot que `new`,
    /// mais la prise/le rendez-vous viennent du bus. Réservé à `coopsim` sur bus.
    pub(crate) fn new_on(
        label: impl Into<String>,
        verbose: bool,
        phase: f32,
        socket: super::transport::Socket,
        rendezvous: std::net::SocketAddr,
    ) -> Bot {
        let link = NetLink::new_on(socket, rendezvous, random_color());
        Bot::from_link(label, verbose, phase, link)
    }

    /// Assemble un `Bot` autour d'un `NetLink` (anti-divergence D2 : UNE construction, partagée par
    /// `new` (UDP) et `new_on` (bus)). C'est le bot HONNÊTE complet — il ne dépend en rien du backend.
    fn from_link(label: impl Into<String>, verbose: bool, phase: f32, link: NetLink) -> Bot {
        Bot {
            label: label.into(),
            verbose,
            link,
            holes: HashMap::new(),
            punch_tries: HashMap::new(),
            buckets: HashMap::new(),
            relay_credits: HashMap::new(),
            relays_to_us: HashSet::new(),
            relay_fallback: relay_fallback_enabled(),
            force_relay: false,
            relay_redundancy: relay_redundancy_from_env(),
            recent_self_states: VecDeque::new(),
            send_credits: HashMap::new(),
            last_state: HashMap::new(),
            heard_count: HashMap::new(),
            link_arrivals: HashMap::new(),
            orb: Orb::headless(),
            seq: 0,
            hello_acc: HELLO_PERIOD,
            punch_acc: 0.0,
            punch_retry_acc: 0.0,
            send_acc: 0.0,
            gossip_acc: 0.0,
            gossip_cursor: 0,
            cell_summary_acc: 0.0,
            summary_cursor: 0,
            proof_cursor: 0,
            wander: phase,
            last_pos: None,
            warned_version: false,
            accepted: 0,
            rejected: 0,
            relayed: 0,
            external_pose: None,
            avatar_sink: None,
            aoi_bilateral: aoi_bilateral_enabled(),
            recv_caps: HashMap::new(),
            recv_in_window: 0,
            budget_acc: 0.0,
        }
    }

    /// SIDECAR (palier 2) : impose la pose émise depuis l'extérieur (Unreal). Tant qu'appelée, le
    /// bot émet CETTE position au lieu du cercle de balade.
    pub(crate) fn set_external_pose(&mut self, pos: Vec3, yaw: f32, pitch: f32) {
        self.external_pose = Some((pos, yaw, pitch));
    }

    /// SIDECAR (palier 2) : active la capture des avatars distants complets (pour Unreal).
    pub(crate) fn enable_avatar_sink(&mut self) {
        if self.avatar_sink.is_none() {
            self.avatar_sink = Some(HashMap::new());
        }
    }

    /// BANC DÉTERMINISTE (12.3) : simule un NAT infranchissable — le perçage n'« ouvre » jamais côté
    /// émission, tout l'état part par le RELAIS du rendez-vous. Prouve le repli bidirectionnel SANS
    /// vrai mobile (les bancs sur `lo` perçaient toujours → la branche relais n'était jamais testée).
    pub(crate) fn enable_force_relay(&mut self) {
        self.force_relay = true;
    }

    /// COUCHE 2 — active l'AoI BILATÉRALE sur CE bot (sans toucher l'environnement) : pour la piloter
    /// depuis la CAMPAGNE (`aoi=1`) le temps d'une session de mesure, au lieu d'un rebuild avec
    /// `AOI_BILATERAL=1`. Reste gaté/par-session : on n'allume rien en dehors d'une mesure consentie.
    pub(crate) fn enable_aoi_bilateral(&mut self) {
        self.aoi_bilateral = true;
    }

    /// RELAIS — règle la REDONDANCE d'émission (`KIND_STATE_BUNDLE`, les K derniers états par envoi
    /// relais) sur CE bot, le temps d'une session, depuis la CAMPAGNE (`redundancy=K`) — au lieu d'un
    /// rebuild avec `RELAY_REDUNDANCY=K`. Borné à [1, 8] (anti-abus du budget relais) ; 1 = inchangé.
    /// C'est le levier B : battre la perte des VRAIS liens CGNAT lossy (relais obligatoire) par `p^k`.
    pub(crate) fn set_relay_redundancy(&mut self, k: usize) {
        self.relay_redundancy = k.clamp(1, 8);
    }

    /// SIDECAR (palier 2) : les avatars distants FRAIS (PlayerState copiés, filtrés ≤ REMOTE_TIMEOUT).
    /// Vide si le puits n'est pas activé.
    pub(crate) fn avatars(&self, now: f32) -> Vec<PlayerState> {
        match &self.avatar_sink {
            Some(sink) => sink
                .values()
                .filter(|(_, t)| now - t < REMOTE_TIMEOUT)
                .map(|(st, _)| *st)
                .collect(),
            None => Vec::new(),
        }
    }

    /// La couleur de skin de ce nœud (pour le WELCOME du sidecar).
    pub(crate) fn my_color(&self) -> (f32, f32, f32) {
        self.link.my_color
    }

    pub(crate) fn neighbors(&self) -> usize {
        self.link.peers.len()
    }

    /// Mon identité (clé), si déjà connue (après le 1er WELCOME). Pour instrumenter le graphe (D25).
    pub(crate) fn id(&self) -> Option<PeerId> {
        self.link.my_id
    }

    /// Les pairs vers qui mon trou est OUVERT (= j'ai reçu leur PUNCH → je peux leur relayer états/
    /// résumés). C'est l'ARÊTE du graphe de communication réel : sert au diagnostic de percolation (D25).
    pub(crate) fn open_holes(&self) -> Vec<PeerId> {
        self.holes.iter().filter(|&(_, &open)| open).map(|(id, _)| *id).collect()
    }
    pub(crate) fn accepted(&self) -> u64 {
        self.accepted
    }
    pub(crate) fn rejected(&self) -> u64 {
        self.rejected
    }
    pub(crate) fn relayed(&self) -> u64 {
        self.relayed
    }
    /// Total d'octets ÉMIS / REÇUS par ce nœud depuis l'ouverture de sa prise (7.4).
    /// Sert à mesurer la bande passante réelle par nœud dans la simulation.
    pub(crate) fn bytes_up(&self) -> u64 {
        self.link.socket.bytes_sent()
    }
    pub(crate) fn bytes_down(&self) -> u64 {
        self.link.socket.bytes_recv()
    }
    /// Combien de pairs ce nœud a-t-il mis en sourdine (directement ou par quorum) ?
    pub(crate) fn muted(&self) -> usize {
        self.link.muted_count() // 9.3 : score décru ≥ seuil (la réhabilitation est prise en compte)
    }
    pub(crate) fn orb_master(&self) -> Option<PeerId> {
        self.orb.owner
    }

    /// MESURE (agent v0) : l'ÂGE (ms) du dernier état reçu de chaque pair connu — la FRAÎCHEUR,
    /// la grandeur « est-ce vivant ? » (≤ 500 ms = jouable). Lecture SEULE : on ne touche pas au
    /// cœur, on LIT `peer_seen` (l'horodatage que le réseau tient déjà). `saturating_` = jamais de
    /// panique si l'horloge bouge d'un poil.
    pub(crate) fn peer_freshness_ms(&self) -> Vec<(PeerId, f64)> {
        let now = Instant::now();
        self.link
            .peer_seen
            .iter()
            .map(|(id, seen)| (*id, now.saturating_duration_since(*seen).as_secs_f64() * 1000.0))
            .collect()
    }

    /// INSTRUMENT (agent) : note une arrivée d'état acceptée `(now_s, seq)` pour le pair `id`. Borné :
    /// au-delà de `MAX_ARRIVALS_PER_PEER` on oublie la plus ancienne (anneau) → mémoire plate même si
    /// l'agent tourne des heures sans drainer. Appelé UNIQUEMENT aux points d'acceptation (lecture pure).
    fn record_arrival(&mut self, id: PeerId, now: f32, seq: u64) {
        const MAX_ARRIVALS_PER_PEER: usize = 8192;
        let v = self.link_arrivals.entry(id).or_default();
        v.push((now as f64 * 1000.0, seq));
        if v.len() > MAX_ARRIVALS_PER_PEER {
            v.remove(0);
        }
    }

    /// Traite UN état signé reçu (forme KIND_STATE, 182 o), qu'il arrive en DIRECT, par RELAIS, ou
    /// éclaté d'un LOT (`KIND_STATE_BUNDLE`). Logique IDENTIQUE à l'ex-bras `KIND_STATE` (extraite
    /// pour être partagée) : sceau → PoW/sourdine/anti-rejeu → anti-téléport → acceptation. Le seau
    /// par-source en tête de boucle a déjà gaté le DATAGRAMME ; on ne re-facture rien par état du lot.
    fn ingest_state(&mut self, bytes: &[u8], from: SocketAddr, now: f32) {
        if !sig_ok(bytes) {
            return;
        }
        match decode_canonical(bytes) {
            Some(state) => {
                if state.id.has_pow(pow_bits())
                    && !self.link.is_muted(state.id)
                    && self.link.accept_seq(state.id, state.seq)
                {
                    let np = Vec3::new(state.x, state.y, state.z);
                    let teleport = match self.last_state.get(&state.id) {
                        Some((prev, t)) => !move_plausible(*prev, np, now - t),
                        None => false,
                    };
                    if teleport {
                        self.link.punish(state.id, "téléport (vitesse impossible)");
                        self.rejected += 1;
                    } else {
                        self.last_state.insert(state.id, (np, now));
                        self.link.note_pos(state.id, (np.x, np.z));
                        self.link.remember_signed_state(state.id, bytes.to_vec()); // C-sécu-2 (gaté)
                        if let Some(sink) = self.avatar_sink.as_mut() {
                            sink.insert(state.id, (state, now)); // sidecar : avatar complet
                        }
                        // 12.3 (fix REPRISE §2 porté dans Bot) : un état RELAYÉ arrive FROM le
                        // rendez-vous → NE PAS croire à un trou direct (sinon on émettrait en direct
                        // dans le vide ET on ne relaierait jamais en retour = le sens unique observé
                        // en réel). On note la réciprocité.
                        if from == self.link.rendezvous {
                            self.relays_to_us.insert(state.id);
                        } else {
                            self.holes.insert(state.id, true);
                        }
                        *self.heard_count.entry(state.id).or_insert(0) += 1; // 8.2b
                        self.record_arrival(state.id, now, state.seq); // instrument agent
                        self.recv_in_window = self.recv_in_window.saturating_add(1); // couche 2 : Hz reçu mesuré
                        self.accepted += 1;
                    }
                }
            }
            None => {
                if let Some(id) = claimed_id(bytes) {
                    self.link.punish(id, "état signé impossible (NaN)");
                    self.rejected += 1;
                }
            }
        }
    }

    /// INSTRUMENT (agent) : DRAINE le journal des arrivées par pair, converti en `metrics::Arrival`
    /// (instant de réception + `seq`). Vide le tampon → la prochaine fenêtre repart de zéro. Nourrit
    /// `metrics::link_stats` (perte/gigue/ré-ordre) sur de VRAIS liens. Lecture pure, cœur intouché.
    pub(crate) fn take_link_arrivals(&mut self) -> HashMap<PeerId, Vec<super::metrics::Arrival>> {
        std::mem::take(&mut self.link_arrivals)
            .into_iter()
            .map(|(id, v)| {
                let arr = v.into_iter().map(|(recv_ms, seq)| super::metrics::Arrival { recv_ms, seq }).collect();
                (id, arr)
            })
            .collect()
    }

    /// 8.3 : foule TOTALE perçue via les résumés de cellule détenus (somme des cellules suivies).
    /// ≈ taille de la foule ⇒ l'invariant tient (toute la foule via quelques flux, pas N états).
    pub(crate) fn summary_perceived(&self) -> u32 {
        self.link.summary_perceived()
    }

    /// Compteurs d'ingestion de résumé (D25, instrumentation) : pourquoi les résumés sont
    /// acceptés/rejetés — pour trancher « la découverte ne livre pas » vs « D26 couche 1 rejette ».
    pub(crate) fn summary_stats(&self) -> super::link::SummaryStats {
        self.link.summary_stats
    }

    /// Remet à zéro le compteur d'écoute (chap. 8.2b) : appelé au DÉBUT de la fenêtre de
    /// mesure, pour que les tiers focus/conscience ne reflètent QUE la fenêtre.
    pub(crate) fn reset_heard(&mut self) {
        self.heard_count.clear();
    }

    /// Tiers de fidélité ENTENDUS sur la fenêtre (chap. 8.2b) : `(focus, conscience)`. Un pair
    /// entendu à ≥ `FOCUS_RATE_MIN` Hz est au focus (plein débit) ; entendu mais moins = conscience
    /// (basse fidélité). Les pairs CONNUS mais jamais entendus ne comptent dans aucun tier — c'est
    /// la correction d'honnêteté du 8.2b (avant, on comptait les connus, pas les entendus).
    pub(crate) fn heard_tiers(&self, secs: f32) -> (usize, usize) {
        let mut focus = 0usize;
        let mut conscience = 0usize;
        for &c in self.heard_count.values() {
            if c == 0 {
                continue;
            }
            if c as f32 / secs.max(1.0) >= FOCUS_RATE_MIN {
                focus += 1;
            } else {
                conscience += 1;
            }
        }
        (focus, conscience)
    }

    /// UNE itération du nœud : HELLO, rate-limit, réception (mêmes décisions de
    /// confiance que le jeu), émission de notre état, perçage NAT.
    pub(crate) fn step(&mut self, dt: f32, now: f32) {
        self.wander += dt * 0.6;
        // Position émise : soit IMPOSÉE de l'extérieur (sidecar piloté par Unreal, palier 2), soit le
        // cercle de balade par défaut. Foule centrée au CENTRE d'une cellule (chap. 8.3d) : le cercle
        // de rayon `WANDER_RADIUS` (≪ CELL_SIZE) tient dans UNE cellule (0,0) → un hôte unique la résume
        // (count ≈ N). Headless only. Le chemin par défaut (external_pose = None) est INCHANGÉ.
        let (pos, send_yaw, send_pitch) = match self.external_pose {
            Some((p, yaw, pitch)) => (p, yaw, pitch),
            None => (
                Vec3::new(
                    CELL_SIZE * 0.5 + WANDER_RADIUS * self.wander.cos(),
                    0.7,
                    CELL_SIZE * 0.5 + WANDER_RADIUS * self.wander.sin(),
                ),
                self.wander,
                0.0,
            ),
        };

        // 1) HELLO vers le rendez-vous (porte notre identité = notre clé).
        self.hello_acc += dt;
        if self.hello_acc >= HELLO_PERIOD {
            self.hello_acc = 0.0;
            let hello = encode_hello(pos.x, pos.z, self.link.identity.id());
            let _ = self.link.socket.send_to(self.link.rendezvous, &hello);
        }

        // 2) Recharge des seaux + budget de relais ; éviction si trop d'adresses.
        for credit in self.buckets.values_mut() {
            *credit = (*credit + dt * BUCKET_RATE).min(BUCKET_CAP);
        }
        for credit in self.relay_credits.values_mut() {
            *credit = (*credit + dt * RELAY_RATE).min(RELAY_CAP);
        }
        if self.buckets.len() > MAX_BUCKETS {
            self.buckets.retain(|_, c| *c < BUCKET_CAP);
        }
        // 8.1b : recharge des seaux d'apprentissage de gossip (rate-limit par source, D23).
        self.link.recharge_gossip_credit(dt);

        // 3) Réception.
        let inbox = self.link.socket.poll();
        for (from, bytes) in inbox {
            let credit = self.buckets.entry(from).or_insert(BUCKET_CAP);
            if *credit < 1.0 {
                continue;
            }
            *credit -= 1.0;

            if bytes.len() >= 2 && bytes[1] != PROTO_VERSION {
                if !self.warned_version {
                    self.warned_version = true;
                    if self.verbose {
                        eprintln!("[bot {}] ⚠ version protocole différente — paquets ignorés.", self.label);
                    }
                }
                continue;
            }

            match kind(&bytes) {
                Some(KIND_WELCOME) => {
                    if let Some((_hue, roster)) = decode_welcome(&bytes) {
                        self.link.my_id = Some(self.link.identity.id());
                        // 8.1 : le WELCOME AMORCE la table (il ne l'ÉCRASE plus). Le
                        // rendez-vous n'est qu'un point de départ ; le gossip fera le reste.
                        for (id, addr) in &roster {
                            if self.link.learn_peer(*id, *addr, None) {
                                self.holes.entry(*id).or_insert(false);
                                if self.verbose {
                                    println!("[bot {}] nouveau pair {} (amorçage).", self.label, id.short());
                                }
                            }
                        }
                    }
                }
                // 8.1 : CARTES DE VISITE d'autres pairs. On apprend les inconnus (puis on
                // les perce). C'est ce qui lève le plafond de 32 : le 33e devient APPRENABLE.
                Some(KIND_GOSSIP) => {
                    if let Some(cards) = decode_gossip(&bytes) {
                        for c in cards {
                            // 8.1b : apprentissage DURCI (PoW + pas d'écrasement d'adresse +
                            // rate-limit par source `from`) → ferme la porte DoS de D23.
                            if self.link.learn_from_gossip(from, c.id, c.addr, (c.x, c.z)) {
                                self.holes.entry(c.id).or_insert(false);
                                if self.verbose {
                                    println!("[bot {}] pair {} appris par gossip.", self.label, c.id.short());
                                }
                            }
                        }
                    }
                }
                // 8.3c : RÉSUMÉ DE CELLULE (AUTHENTIFIÉ, D26-couche-1). On le retient (dernier frais
                // par cellule) → on perçoit la foule via UN flux. `ingest_summary` vérifie d'abord
                // (pas cher) que l'émetteur est l'hôte attendu, PUIS (cher) le sceau → un faux d'un
                // non-hôte est jeté SANS vérif de signature ; la menace CPU de forge est donc bornée
                // sans seau dédié, le seau général `buckets` ci-dessus gatant déjà chaque paquet/source.
                Some(KIND_CELL_SUMMARY) => {
                    if let (Some(s), Some(my_id)) = (decode_cell_summary(&bytes), self.link.my_id) {
                        self.link.ingest_summary(s, (pos.x, pos.z), my_id);
                    }
                }
                // 8.3★ C-sécu-2 : résumé v2 (KIND 10) = même résumé + un trailer de preuves auto-signées.
                // Étape 4 : sous `SIGNED_SAMPLES`, on VÉRIFIE chaque preuve (sceau + cache (id,seq)) et on en
                // tire le plancher (IDs vérifiés ∈ cellule → `summary_perceived`). Le résumé lui-même reste
                // ingéré comme un v1 (corroboration des counts inchangée). Le gating au site d'appel garde le
                // défaut byte-intact : hors `SIGNED_SAMPLES` personne n'émet de v2 ET on n'appelle pas
                // `verify_proof` (zéro mémoire même si un attaquant nous pousse un KIND 10).
                Some(KIND_CELL_SUMMARY_V2) => {
                    if let (Some((s, proofs)), Some(my_id)) =
                        (decode_cell_summary_v2(&bytes), self.link.my_id)
                    {
                        if crate::net::link::signed_samples_mode() {
                            for proof in &proofs {
                                self.link.verify_proof(proof);
                            }
                        }
                        self.link.ingest_summary(s, (pos.x, pos.z), my_id);
                    }
                }
                Some(KIND_PUNCH) => {
                    if let Some(id) = decode_punch(&bytes) {
                        if self.holes.insert(id, true) != Some(true) && self.verbose {
                            println!("[bot {}] trou ouvert avec {}.", self.label, id.short());
                        }
                    }
                }
                // COUCHE 2 — un pair nous ANNONCE son plafond de réception (gaté AOI_BILATERAL). On ne
                // l'accepte QUE depuis l'ADRESSE du pair concerné (anti-usurpation simple : un tiers ne
                // peut pas brider le trafic d'autrui). Stocké avec `now` → expire après RECV_CAP_TTL
                // (le silence = « je vais bien, envoie normalement »).
                Some(KIND_RECV_BUDGET) => {
                    if self.aoi_bilateral {
                        if let Some((id, cap)) = decode_recv_budget(&bytes) {
                            if self.link.peers.get(&id) == Some(&from) {
                                self.recv_caps.insert(id, (cap, now));
                            }
                        }
                    }
                }
                Some(KIND_STATE) => self.ingest_state(&bytes, from, now),
                // LOT relais (12.3 / D17) : on ÉCLATE le lot et on traite chaque état par le MÊME
                // chemin que KIND_STATE (du + ancien au + récent) → accept_seq dédoublonne, et un seq
                // contenu dans le lot COMBLE un trou s'il avait été perdu auparavant. Borné par le seau
                // par-source en tête de boucle (un lot = UN datagramme = 1 crédit), anti-DoS inchangé.
                Some(KIND_STATE_BUNDLE) => {
                    if let Some(parts) = decode_state_bundle(&bytes) {
                        for s in parts {
                            self.ingest_state(s, from, now);
                        }
                    }
                }
                Some(KIND_RELAY) => {
                    if !sig_ok(&bytes) {
                        continue;
                    }
                    if let Some(state) = decode_canonical(&bytes) {
                        if state.id.has_pow(pow_bits())
                            && !self.link.is_muted(state.id)
                            && self.link.accept_seq(state.id, state.seq)
                        {
                            let np = Vec3::new(state.x, state.y, state.z);
                            let teleport = match self.last_state.get(&state.id) {
                                Some((prev, t)) => !move_plausible(*prev, np, now - t),
                                None => false,
                            };
                            if teleport {
                                self.link.punish(state.id, "relais : téléport (vitesse impossible)");
                                self.rejected += 1;
                            } else {
                                self.last_state.insert(state.id, (np, now));
                                self.record_arrival(state.id, now, state.seq); // instrument agent (relais)
                                self.link.note_pos(state.id, (np.x, np.z));
                                self.link.remember_signed_state(state.id, bytes.to_vec()); // C-sécu-2 (gaté)
                                if let Some(sink) = self.avatar_sink.as_mut() {
                                    sink.insert(state.id, (state, now)); // sidecar : avatar complet (relais)
                                }
                                let rc = self.relay_credits.entry(state.id).or_insert(RELAY_CAP);
                                let mut n = 0u32;
                                if *rc >= 1.0 {
                                    *rc -= 1.0;
                                    let mut forward = bytes.clone();
                                    forward[0] = KIND_STATE;
                                    let targets: Vec<(PeerId, SocketAddr)> =
                                        self.link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                                    for (id, addr) in targets {
                                        if id != state.id {
                                            let _ = self.link.socket.send_to(addr, &forward);
                                            n += 1;
                                            if n as usize >= MAX_RELAY_FANOUT {
                                                break;
                                            }
                                        }
                                    }
                                }
                                *self.heard_count.entry(state.id).or_insert(0) += 1; // 8.2b
                                self.accepted += 1;
                                self.relayed += n as u64;
                                if n > 0 && self.verbose {
                                    println!("[bot {}] ↪ RELAY de {} recopié à {n} pairs (≤ fanout {MAX_RELAY_FANOUT}).", self.label, state.id.short());
                                }
                            }
                        }
                    }
                }
                Some(KIND_ORB) => {
                    // SIDECAR (palier 2-3) : l'orbe est PALIER 4 → on ne la traite pas encore. Surtout :
                    // un pair distant relaie ses claims d'orbe légitimes, mais le sidecar ne partage pas
                    // l'historique de CONTACT → `apply_incoming` répondrait NoContact → punition → 5 strikes
                    // → le pair MUET → son avatar invisible. On ignore donc l'orbe tant qu'on rend des avatars.
                    if self.avatar_sink.is_some() {
                        continue;
                    }
                    if !orb_sig_ok(&bytes) {
                        continue;
                    }
                    match decode_orb(&bytes) {
                        Some(w) => {
                            let owner = w.owner;
                            if owner.has_pow(pow_bits()) && !self.link.is_muted(owner) {
                                let claimer_pos = self.last_state.get(&owner).map(|(p, _)| *p);
                                match apply_incoming(&mut self.orb, w, now, claimer_pos) {
                                    OrbApply::Implausible => self.link.punish(owner, "orbe : saut de version aberrant"),
                                    OrbApply::NoContact => self.link.punish(owner, "orbe : revendiquée sans contact"),
                                    _ => {}
                                }
                            }
                        }
                        None => {
                            if let Some(id) = claimed_owner(&bytes) {
                                self.link.punish(id, "orbe : état signé impossible (NaN)");
                            }
                        }
                    }
                }
                Some(KIND_ACCUSE) => {
                    if let Some((accuser, offender)) = decode_accuse(&bytes) {
                        if accuser.has_pow(pow_bits()) && accuser != offender && !self.link.is_muted(accuser) {
                            self.link.record_accusation(offender, accuser);
                        }
                    }
                }
                _ => {}
            }
        }

        // 4) Émission de NOTRE état signé, via l'AoI WATER-FILLING — EXACTEMENT comme le
        //    vrai client (netcode/send.rs) : un budget d'émission fini (SEND_BUDGET_HZ)
        //    réparti entre les voisins par pertinence (distance), au lieu d'un envoi naïf
        //    plein débit à tous. C'est ce qui rend la mesure 7.4 FIDÈLE au jeu (7.4b).
        self.send_acc += dt;
        if self.send_acc >= 1.0 / SEND_HZ {
            let dt_send = self.send_acc;
            self.send_acc = 0.0;
            if let Some(my_id) = self.link.my_id {
                let velocity = match self.last_pos {
                    Some(prev) => (pos - prev) / dt_send.max(1e-3),
                    None => Vec3::ZERO,
                };
                self.last_pos = Some(pos);
                self.seq += 1;
                let (r, g, b) = self.link.my_color;
                let me = PlayerState {
                    id: my_id,
                    x: pos.x, y: pos.y, z: pos.z,
                    vx: velocity.x, vy: velocity.y, vz: velocity.z,
                    yaw: send_yaw, pitch: send_pitch,
                    r, g, b,
                    parent: None, seq: self.seq,
                };
                let bytes = encode_signed(&me, &self.link.identity);
                let me_xz = (pos.x, pos.z);

                // LOT relais (redondance temporelle) : on garde nos `relay_redundancy` derniers états
                // signés (le plus ancien d'abord). Sans frais quand redondance=1 (anneau d'un seul élément,
                // jamais lu car le chemin relais reste l'envoi brut). Cf. encode_state_bundle / D17.
                if self.relay_redundancy > 1 {
                    self.recent_self_states.push_back(bytes.to_vec());
                    while self.recent_self_states.len() > self.relay_redundancy {
                        self.recent_self_states.pop_front();
                    }
                }

                // 0) FOCUS COLLANT (chap. 8.2a-bis) : mise à jour hystérétique de l'ensemble plein
                //    débit AVANT d'allouer → pas de recomposition du top-K à chaque tick (fin du churn).
                self.link.refresh_focus(me_xz);

                // a) PERTINENCE : un poids par pair selon sa dernière position connue
                //    (inconnu → distance 0 → poids max, pour le découvrir vite).
                let peers: Vec<(PeerId, SocketAddr)> =
                    self.link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                let weights: Vec<f32> = peers
                    .iter()
                    .map(|(id, _)| {
                        let d2 = self
                            .last_state
                            .get(id)
                            .map(|(p, _)| dist2(me_xz, (p.x, p.z)))
                            .unwrap_or(0.0);
                        relevance_weight(d2)
                    })
                    .collect();
                // b) AoI À DEUX TIERS (chap. 8.2 / 8.2a-bis) : le FOCUS COLLANT au plein débit,
                //    la conscience (le reste) en basse fidélité — comme le vrai client.
                let is_focus: Vec<bool> = peers.iter().map(|(id, _)| self.link.is_focus(id)).collect();
                // COUCHE 2 — quand l'AoI bilatérale est ACTIVE, on respecte le plafond de réception
                // ANNONCÉ par chaque pair (∞ si pas d'annonce fraîche) → on n'émet jamais vers un pair
                // plus vite qu'il ne peut encaisser. OFF → on appelle l'`allocate_tiers` ORIGINAL
                // intouché (chemin par défaut byte-pour-byte, aucune entrée bilatérale possible).
                let rates = if self.aoi_bilateral {
                    let recv_caps: Vec<f32> = peers
                        .iter()
                        .map(|(id, _)| match self.recv_caps.get(id) {
                            Some(&(cap, t)) if now - t < RECV_CAP_TTL => cap,
                            _ => f32::INFINITY,
                        })
                        .collect();
                    allocate_tiers_bilateral(&weights, &is_focus, SEND_BUDGET_HZ, SEND_HZ, &recv_caps)
                } else {
                    allocate_tiers(&weights, &is_focus, SEND_BUDGET_HZ, SEND_HZ)
                };
                // c) CADENCEMENT par crédit. Direct vers les trous OUVERTS ; REPLI (gaté) via le
                //    rendez-vous vers les pairs qui ne percent pas. Défaut : trou ouvert seulement
                //    (byte-pour-byte). Décision isolée et testée dans `bot_send_kind`.
                for ((id, addr), rate) in peers.iter().zip(&rates) {
                    let open = *self.holes.get(id).unwrap_or(&false);
                    let abandoned = punch_abandoned(*self.punch_tries.get(id).unwrap_or(&0));
                    let relays_back = self.relays_to_us.contains(id);
                    let decision =
                        bot_send_kind(open, abandoned, relays_back, self.relay_fallback, self.force_relay);
                    if decision == SendKind::Skip {
                        continue;
                    }
                    let credit = self.send_credits.entry(*id).or_insert(0.0);
                    *credit += rate * dt_send;
                    if *credit >= 1.0 {
                        *credit -= 1.0;
                        match decision {
                            SendKind::Direct => {
                                let _ = self.link.socket.send_to(*addr, &bytes);
                            }
                            SendKind::Relay => {
                                // On demande au rendez-vous de porter notre état SCELLÉ jusqu'à ce pair.
                                // REDONDANCE (défaut 1 = inchangé, byte-pour-byte) : à k≥2 on porte les
                                // K DERNIERS états dans UN seul lot (budget-free : 1 envoi/tick, réparti
                                // sur K ticks → bat la perte indépendante p^K ET la perte en rafale). Le
                                // récepteur éclate le lot et dédoublonne nativement (accept_seq).
                                let env = if self.relay_redundancy > 1 && !self.recent_self_states.is_empty() {
                                    let slices: Vec<&[u8]> =
                                        self.recent_self_states.iter().map(|v| v.as_slice()).collect();
                                    encode_relay_fwd(*id, &encode_state_bundle(&slices))
                                } else {
                                    encode_relay_fwd(*id, &bytes)
                                };
                                let _ = self.link.socket.send_to(self.link.rendezvous, &env);
                            }
                            SendKind::Skip => {}
                        }
                    }
                }
                self.send_credits.retain(|id, _| self.link.peers.contains_key(id));
            }
        }

        // 4ter) COUCHE 2 — ANNONCE DU BUDGET DE RÉCEPTION (gaté AOI_BILATERAL, défaut OFF → bloc
        //       INERTE, émission byte-pour-byte). Une fois par BUDGET_PERIOD : si on encaisse PLUS
        //       que notre budget (`measured_recv_hz > RECV_BUDGET_HZ`), on annonce à nos émetteurs
        //       un plafond par-lien (part équitable) ; sinon on se TAIT (∞ = pas de bride, le cas
        //       courant n'émet rien). Advisory, best-effort, négligeable (38 o/pair, 1×/s).
        if self.aoi_bilateral {
            self.budget_acc += dt;
            if self.budget_acc >= BUDGET_PERIOD {
                let measured_recv_hz = self.recv_in_window as f32 / self.budget_acc.max(1.0e-3);
                let n_senders = self.link.peers.len();
                let cap = advertised_recv_cap(RECV_BUDGET_HZ, measured_recv_hz, n_senders);
                if cap.is_finite() {
                    if let Some(my_id) = self.link.my_id {
                        let advert = encode_recv_budget(&my_id, cap);
                        let addrs: Vec<SocketAddr> = self.link.peers.values().copied().collect();
                        for addr in addrs {
                            let _ = self.link.socket.send_to(addr, &advert);
                        }
                    }
                }
                self.recv_in_window = 0;
                self.budget_acc = 0.0;
                // Oubli des plafonds périmés (le silence d'un pair = il va bien → ∞). TTL seul suffit
                // (un pair parti cesse d'annoncer → son entrée expire) : pas de fuite, pas de bride fantôme.
                self.recv_caps.retain(|_, (_, t)| now - *t < RECV_CAP_TTL);
            }
        }

        // 4bis) GOSSIP (chap. 8.1) : on présente un lot de cartes de visite (un
        //       sous-ensemble DIVERS, par curseur tournant) à quelques pairs au trou
        //       ouvert. De proche en proche, chacun finit par apprendre toute la foule
        //       → le plafond de 32 du rendez-vous est levé, SANS serveur qui énumère.
        self.gossip_acc += dt;
        if self.gossip_acc >= GOSSIP_PERIOD {
            self.gossip_acc = 0.0;
            if let Some(my_id) = self.link.my_id {
                let open: Vec<SocketAddr> = self
                    .link
                    .peers
                    .iter()
                    .filter(|(id, _)| *self.holes.get(id).unwrap_or(&false))
                    .map(|(_, a)| *a)
                    .take(GOSSIP_FANOUT)
                    .collect();
                if !open.is_empty() {
                    let cards = sample_cards(&self.link.peers, &self.link.peer_pos, my_id, self.gossip_cursor);
                    self.gossip_cursor = self.gossip_cursor.wrapping_add(cards.len());
                    if !cards.is_empty() {
                        let pkt = encode_gossip(&cards);
                        for addr in open {
                            let _ = self.link.socket.send_to(addr, &pkt);
                        }
                    }
                }
            }
        }

        // 4ter) RÉSUMÉS DE CELLULE (chap. 8.3c) : si je suis l'HÔTE de ma cellule, j'émets son
        //       résumé ; et je RELAIE un échantillon des résumés que je détiens. Propagation
        //       ÉPIDÉMIQUE bornée (comme le gossip) → chacun finit par percevoir la foule lointaine
        //       via UN flux par cellule, à fraîcheur fixe (fin de l'effondrement 1/N de la conscience),
        //       SANS que l'hôte n'inonde tout le monde (O(fanout), pas O(N) — le piège « hôte qui fond »).
        self.cell_summary_acc += dt;
        if self.cell_summary_acc >= CELL_SUMMARY_PERIOD {
            self.cell_summary_acc = 0.0;
            if let Some(my_id) = self.link.my_id {
                let open: Vec<SocketAddr> = self
                    .link
                    .peers
                    .iter()
                    .filter(|(id, _)| *self.holes.get(id).unwrap_or(&false))
                    .map(|(_, a)| *a)
                    .take(CELL_SUMMARY_FANOUT)
                    .collect();
                if !open.is_empty() {
                    // (a) Mon propre résumé si je suis hôte → SIGNÉ (D26-couche-1) et ingéré (compté
                    //     ET relayé). La FRAÎCHEUR n'est plus l'horloge `ts` (forgeable) mais mon `seq`
                    //     monotone interne, porté verbatim par les relais (8.3d) → ma copie fraîche bat
                    //     les vieilles encore en vol, et un non-hôte ne peut pas forger ce seq.
                    // 8.3★ C-sécu : sous corroboration, CHAQUE nœud publie l'estimation de SA cellule
                    // (pas seulement l'hôte → plusieurs signataires/cellule, la corroboration est exercée) ;
                    // sinon (défaut/DENSITY_MAX) seul l'hôte élu résume sa cellule (modèle 8.3c/8.3d).
                    if crate::net::link::density_corrob_mode() {
                        let s = self.link.build_own_cell_claim((pos.x, pos.z), my_id);
                        self.link.ingest_summary(s, (pos.x, pos.z), my_id);
                    } else if let Some(s) = self.link.build_my_cell_summary((pos.x, pos.z), my_id) {
                        self.link.ingest_summary(s, (pos.x, pos.z), my_id);
                    }
                    // (b) Relais borné : un échantillon TOURNANT des résumés détenus (épidémie). Sous CORROB
                    // on relaie les claims multi-signataires (`cell_claims`) ; sinon les résumés d'hôte.
                    let summaries: Vec<CellSummary> = if crate::net::link::density_corrob_mode() {
                        self.link.cell_claims.values().cloned().collect()
                    } else {
                        self.link.cell_summaries.values().cloned().collect()
                    };
                    if !summaries.is_empty() {
                        let start = self.summary_cursor % summaries.len();
                        self.summary_cursor = self.summary_cursor.wrapping_add(1);
                        for k in 0..summaries.len().min(MAX_RELAY_SUMMARIES) {
                            let s = &summaries[(start + k) % summaries.len()];
                            // 8.3★ C-sécu-2 étape 3 : à MON propre claim (host == moi), je JOINS un
                            // sous-ensemble tournant de preuves auto-signées de mes occupants (recopie
                            // des états déjà signés que j'ai entendus → zéro re-signature). Les relais
                            // d'autrui restent légers (v1). Hors `SIGNED_SAMPLES` → v1 partout (défaut intact).
                            let pkt = if crate::net::link::signed_samples_mode() && s.host == my_id {
                                let ids: Vec<PeerId> = s.samples.iter().map(|&(id, _, _)| id).collect();
                                let proofs = self.link.proofs_for(&ids, self.proof_cursor, K_PROOF);
                                encode_cell_summary_v2(s, &proofs)
                            } else {
                                encode_cell_summary(s)
                            };
                            for addr in &open {
                                let _ = self.link.socket.send_to(*addr, &pkt);
                            }
                        }
                        // la rotation des preuves avance d'un cran par période → couvre tous les ids au fil du temps
                        self.proof_cursor = self.proof_cursor.wrapping_add(1);
                    }
                }
            }
        }

        // 5bis) DURCISSEMENT RELAIS (29 juin) — RE-SALVE périodique : on ré-arme une courte salve de
        //       perçage pour les liens ABANDONNÉS encore au trou FERMÉ. Un échec INITIAL transitoire
        //       (perte, arrivée tardive, salves désynchronisées) ne condamne plus le lien au relais
        //       lossy à vie ; dès que le direct redevient possible, la boucle 5) le rouvre → DIRECT frais.
        self.punch_retry_acc += dt;
        if self.punch_retry_acc >= PUNCH_RETRY_PERIOD {
            self.punch_retry_acc = 0.0;
            let to_rearm: Vec<PeerId> = self
                .punch_tries
                .iter()
                .filter(|(id, t)| punch_abandoned(**t) && !*self.holes.get(*id).unwrap_or(&false))
                .map(|(id, _)| *id)
                .collect();
            for id in to_rearm {
                if let Some(t) = self.punch_tries.get(&id).copied() {
                    self.punch_tries.insert(id, punch_retry_tries(t));
                }
            }
        }

        // 5) Perçage NAT des pairs au trou fermé.
        self.punch_acc += dt;
        if self.punch_acc >= PUNCH_PERIOD {
            self.punch_acc = 0.0;
            if let Some(my_id) = self.link.my_id {
                let punch = encode_punch(my_id);
                let targets: Vec<(PeerId, SocketAddr)> =
                    self.link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                for (id, addr) in targets {
                    let open = *self.holes.get(&id).unwrap_or(&false);
                    let tries = *self.punch_tries.get(&id).unwrap_or(&0);
                    // 8.1b : on ne perce ni un trou ouvert, ni un trou abandonné (jamais
                    // corroboré → carte empoisonnée / NAT symétrique) → anti-réflexion.
                    if open || punch_abandoned(tries) {
                        continue;
                    }
                    let _ = self.link.socket.send_to(addr, &punch);
                    *self.punch_tries.entry(id).or_insert(0) += 1;
                }
            }
        }
    }
}

/// Mode `bot` : UN nœud, bavard (imprime un « ledger » périodique).
pub fn run_bot(label: &str) {
    let mut bot = match Bot::new(label, true, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[bot {label}] réseau indisponible.");
            return;
        }
    };
    println!("[bot {label}] démarré — je fais tourner le VRAI protocole, sans fenêtre 3D.");

    let start = Instant::now();
    let mut last = Instant::now();
    let mut summary_acc = 0.0f32;

    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();

        bot.step(dt, now);

        summary_acc += dt;
        if summary_acc >= SUMMARY_PERIOD {
            summary_acc = 0.0;
            let maitre = bot.orb_master().map(|o| o.short()).unwrap_or_else(|| "—".to_string());
            println!(
                "[bot {label}] t={now:.0}s | pairs={} | orbe: maître={maitre} | acceptés={} rejetés={} relayés={} muets={}",
                bot.neighbors(), bot.accepted(), bot.rejected(), bot.relayed(), bot.muted()
            );
        }

        std::thread::sleep(TICK);
    }
}

#[cfg(test)]
mod tests {
    use super::{aoi_bilateral_on, bot_send_kind, relay_fallback_on, relay_redundancy_of, SendKind};

    /// Le drapeau de repli relais : par défaut OFF (chemin direct historique intact), ON seulement
    /// sur `1`/`true`. Relogé depuis l'ancien `netcode/send.rs` avec sa logique.
    #[test]
    fn relay_fallback_eteint_par_defaut() {
        assert!(!relay_fallback_on(None)); // absent → OFF (chemin par défaut intact)
        assert!(!relay_fallback_on(Some("0")));
        assert!(!relay_fallback_on(Some("nope"))); // valeur inattendue → OFF (défaut sûr)
        assert!(relay_fallback_on(Some("1")));
        assert!(relay_fallback_on(Some("true")));
    }

    /// COUCHE 2 — le drapeau de l'AoI BILATÉRALE : OFF par défaut (émission byte-pour-byte historique),
    /// ON seulement sur `1`/`true`. Tant qu'il est OFF, le bot appelle l'`allocate_tiers` original et
    /// n'annonce/n'applique aucun budget → garantie « zéro impact tant qu'on ne l'allume pas ».
    #[test]
    fn aoi_bilateral_eteint_par_defaut() {
        assert!(!aoi_bilateral_on(None)); // absent → OFF (le défaut sûr : rien ne change dans la flotte)
        assert!(!aoi_bilateral_on(Some("0")));
        assert!(!aoi_bilateral_on(Some("nope")));
        assert!(aoi_bilateral_on(Some("1")));
        assert!(aoi_bilateral_on(Some("true")));
    }

    /// REDONDANCE relais : défaut 1 (byte-pour-byte), bornée à [1, 8] ; absent/invalide → 1.
    #[test]
    fn redondance_relais_bornee_et_neutre_par_defaut() {
        assert_eq!(relay_redundancy_of(None), 1); // absent → inchangé
        assert_eq!(relay_redundancy_of(Some("bzz")), 1); // invalide → 1
        assert_eq!(relay_redundancy_of(Some("0")), 1); // 0 n'a pas de sens → ramené à 1
        assert_eq!(relay_redundancy_of(Some("3")), 3);
        assert_eq!(relay_redundancy_of(Some("99")), 8); // borné haut (anti-abus budget relais)
    }

    /// 12.3 — la DÉCISION d'émission du `Bot` headless. Le défaut (repli OFF, force OFF) doit
    /// reproduire EXACTEMENT l'historique : `Direct` si le trou est ouvert, sinon `Skip` (le bot
    /// n'émettait que vers les trous ouverts). Le repli ne s'active QUE sous `RELAY_FALLBACK`.
    #[test]
    fn bot_send_kind_defaut_byte_pour_byte() {
        // Repli ÉTEINT : exactement l'ancien comportement.
        assert_eq!(bot_send_kind(true, false, false, false, false), SendKind::Direct); // trou ouvert
        assert_eq!(bot_send_kind(false, false, false, false, false), SendKind::Skip); // fermé → on attend
        assert_eq!(bot_send_kind(false, true, false, false, false), SendKind::Skip); // abandonné mais repli OFF
        assert_eq!(bot_send_kind(false, false, true, false, false), SendKind::Skip); // pair-relais mais repli OFF
    }

    #[test]
    fn bot_send_kind_repli_garde_le_lien_vivant() {
        // COUCHE 1 (inclusivité) : repli ALLUMÉ + trou fermé → on RELAIE TOUJOURS, pour que le filet
        // conscience atteigne le pair non-perçé (fini le silence bimodal) — que le perçage soit
        // abandonné ou non, réciproque ou non.
        assert_eq!(bot_send_kind(false, true, false, true, false), SendKind::Relay); // abandonné → relais
        assert_eq!(bot_send_kind(false, false, true, true, false), SendKind::Relay); // pair-relais → relais
        // LE FIX : perçage EN COURS (ni abandon ni réciprocité) → on relaie QUAND MÊME, au lieu de
        // se taire et de laisser le pair tomber dans le noir.
        assert_eq!(bot_send_kind(false, false, false, true, false), SendKind::Relay);
        // Trou direct ouvert → JAMAIS de relais, même repli ON (on a une vraie connexion directe).
        assert_eq!(bot_send_kind(true, true, true, true, false), SendKind::Direct);
    }

    #[test]
    fn bot_send_kind_force_relais_du_banc() {
        // Le banc déterministe force le relais quoi qu'il arrive (NAT infranchissable simulé).
        assert_eq!(bot_send_kind(true, false, false, false, true), SendKind::Relay);
        assert_eq!(bot_send_kind(false, false, false, false, true), SendKind::Relay);
    }
}
