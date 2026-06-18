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
use super::aoi::{allocate_rates, dist2, relevance_weight, SEND_BUDGET_HZ};
use super::control::{decode_welcome, encode_hello};
use super::crypto::{PeerId, POW_BITS};
use super::gossip::{decode_gossip, encode_gossip, sample_cards};
use super::link::{NetLink, MAX_STRIKES};
use super::message::{claimed_id, decode_canonical, encode_signed, sig_ok, PlayerState};
use super::orb::{apply_incoming, claimed_owner, decode_orb, orb_sig_ok, Orb, OrbApply};
use super::punch::{decode_punch, encode_punch, punch_abandoned};
use super::skin::random_color;
use super::wire::{
    kind, KIND_ACCUSE, KIND_GOSSIP, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME,
    PROTO_VERSION,
};
use bevy::prelude::Vec3;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

// Miroir des réglages de réception du jeu (cf. netcode/receive.rs et state.rs).
const BUCKET_RATE: f32 = 150.0;
const BUCKET_CAP: f32 = 300.0;
const MAX_BUCKETS: usize = 4096;
const RELAY_RATE: f32 = 30.0;
const RELAY_CAP: f32 = 60.0;
const MAX_RELAY_FANOUT: usize = 12;
const SEND_HZ: f32 = 20.0;
const HELLO_PERIOD: f32 = 1.0;
const PUNCH_PERIOD: f32 = 0.25;
const SUMMARY_PERIOD: f32 = 2.0;
/// Période d'émission du gossip (chap. 8.1) : à chaque tic, on présente un lot de
/// « cartes de visite » à quelques pairs. 0,5 s = découverte rapide sans bavardage.
const GOSSIP_PERIOD: f32 = 0.5;
/// Nombre de destinataires (au trou ouvert) à qui on envoie notre lot de cartes par
/// tic. Petit : la diffusion épidémique couvre la foule en quelques rounds (log N).
const GOSSIP_FANOUT: usize = 4;
const TICK: Duration = Duration::from_millis(50);
const WANDER_RADIUS: f32 = 3.0;

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
    /// Crédits d'émission AoI par pair (chap. 7.4b) : même cadencement par water-filling
    /// que le vrai client ([netcode/send.rs]). Chaque pair accumule `débit × dt` ; à 1,
    /// on lui envoie un paquet. C'est ce qui fait que le bot mesure DÉSORMAIS le coût
    /// réel du jeu (budget réparti par pertinence), pas un envoi naïf plein débit à tous.
    send_credits: HashMap<PeerId, f32>,
    last_state: HashMap<PeerId, (Vec3, f32)>,
    orb: Orb,
    seq: u64,
    hello_acc: f32,
    punch_acc: f32,
    send_acc: f32,
    gossip_acc: f32,
    gossip_cursor: usize,
    wander: f32,
    last_pos: Option<Vec3>,
    warned_version: bool,
    accepted: u64,
    rejected: u64,
    relayed: u64,
}

impl Bot {
    /// Crée un nœud : ouvre une prise, MINE son identité (preuve de travail, 6.2) et
    /// se prépare à rejoindre le rendez-vous. `None` si la prise ne s'ouvre pas.
    /// `phase` décale la position de départ de chaque nœud (pour étaler la « foule »).
    pub(crate) fn new(label: impl Into<String>, verbose: bool, phase: f32) -> Option<Bot> {
        let link = NetLink::new(random_color(), false).ok()?;
        Some(Bot {
            label: label.into(),
            verbose,
            link,
            holes: HashMap::new(),
            punch_tries: HashMap::new(),
            buckets: HashMap::new(),
            relay_credits: HashMap::new(),
            send_credits: HashMap::new(),
            last_state: HashMap::new(),
            orb: Orb::headless(),
            seq: 0,
            hello_acc: HELLO_PERIOD,
            punch_acc: 0.0,
            send_acc: 0.0,
            gossip_acc: 0.0,
            gossip_cursor: 0,
            wander: phase,
            last_pos: None,
            warned_version: false,
            accepted: 0,
            rejected: 0,
            relayed: 0,
        })
    }

    pub(crate) fn neighbors(&self) -> usize {
        self.link.peers.len()
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
        self.link
            .strikes
            .values()
            .filter(|n| **n >= MAX_STRIKES)
            .count()
    }
    pub(crate) fn orb_master(&self) -> Option<PeerId> {
        self.orb.owner
    }

    /// UNE itération du nœud : HELLO, rate-limit, réception (mêmes décisions de
    /// confiance que le jeu), émission de notre état, perçage NAT.
    pub(crate) fn step(&mut self, dt: f32, now: f32) {
        self.wander += dt * 0.6;
        let pos = Vec3::new(
            WANDER_RADIUS * self.wander.cos(),
            0.7,
            WANDER_RADIUS * self.wander.sin(),
        );

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
                Some(KIND_PUNCH) => {
                    if let Some(id) = decode_punch(&bytes) {
                        if self.holes.insert(id, true) != Some(true) && self.verbose {
                            println!("[bot {}] trou ouvert avec {}.", self.label, id.short());
                        }
                    }
                }
                Some(KIND_STATE) => {
                    if !sig_ok(&bytes) {
                        continue;
                    }
                    match decode_canonical(&bytes) {
                        Some(state) => {
                            if state.id.has_pow(POW_BITS)
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
                                    self.holes.insert(state.id, true);
                                    self.accepted += 1;
                                }
                            }
                        }
                        None => {
                            if let Some(id) = claimed_id(&bytes) {
                                self.link.punish(id, "état signé impossible (NaN)");
                                self.rejected += 1;
                            }
                        }
                    }
                }
                Some(KIND_RELAY) => {
                    if !sig_ok(&bytes) {
                        continue;
                    }
                    if let Some(state) = decode_canonical(&bytes) {
                        if state.id.has_pow(POW_BITS)
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
                                self.link.note_pos(state.id, (np.x, np.z));
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
                    if !orb_sig_ok(&bytes) {
                        continue;
                    }
                    match decode_orb(&bytes) {
                        Some(w) => {
                            let owner = w.owner;
                            if owner.has_pow(POW_BITS) && !self.link.is_muted(owner) {
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
                        if accuser.has_pow(POW_BITS) && accuser != offender && !self.link.is_muted(accuser) {
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
                    yaw: self.wander, pitch: 0.0,
                    r, g, b,
                    parent: None, seq: self.seq,
                };
                let bytes = encode_signed(&me, &self.link.identity);
                let me_xz = (pos.x, pos.z);

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
                // b) WATER-FILLING : un débit (Hz) par pair, plafonné à SEND_HZ, somme ≤ budget.
                let rates = allocate_rates(&weights, SEND_BUDGET_HZ, SEND_HZ);
                // c) CADENCEMENT par crédit, vers les pairs au trou OUVERT seulement.
                for ((id, addr), rate) in peers.iter().zip(&rates) {
                    if !*self.holes.get(id).unwrap_or(&false) {
                        continue;
                    }
                    let credit = self.send_credits.entry(*id).or_insert(0.0);
                    *credit += rate * dt_send;
                    if *credit >= 1.0 {
                        *credit -= 1.0;
                        let _ = self.link.socket.send_to(*addr, &bytes);
                    }
                }
                self.send_credits.retain(|id, _| self.link.peers.contains_key(id));
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
