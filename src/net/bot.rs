//! LE BOT HEADLESS : un client qui fait tourner le VRAI code réseau, SANS la 3D.
//!
//! # Pourquoi ce fichier existe (chapitre 6.0)
//! On ne peut pas tester le jeu « en vrai » à grande échelle : les clients sont des
//! fenêtres graphiques. Ce bot rejoint le rendez-vous, perce les NAT, émet son état
//! SIGNÉ et applique à la réception EXACTEMENT les mêmes décisions de confiance que
//! le jeu (sceau auto-certifié via `sig_ok`, anti-rejeu, réputation, rate-limit,
//! autorité d'orbe). Il imprime un « ledger » : accepté / refusé / relayé / muets.
//!
//! Ainsi, « rendez-vous + N bots + 1 attaquant », en terminaux et sans GPU, suffit
//! à VOIR un trou s'ouvrir puis se refermer. C'est l'embryon de la simulation 55K.
//!
//! Lancement (après `cargo run -- rendezvous`) :  cargo run -- bot alice

use super::accuse::decode_accuse;
use super::anticheat::move_plausible;
use super::control::{decode_welcome, encode_hello};
use super::crypto::{PeerId, POW_BITS};
use super::link::NetLink;
use super::message::{claimed_id, decode_canonical, encode_signed, sig_ok, PlayerState};
use super::orb::{apply_incoming, claimed_owner, decode_orb, orb_sig_ok, Orb, OrbApply};
use super::punch::{decode_punch, encode_punch};
use super::skin::random_color;
use super::wire::{
    kind, KIND_ACCUSE, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME, PROTO_VERSION,
};
use bevy::prelude::Vec3;
use std::collections::{HashMap, HashSet};
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
const TICK: Duration = Duration::from_millis(50);
const WANDER_RADIUS: f32 = 3.0;

pub fn run_bot(label: &str) {
    let color = random_color();
    let mut link = match NetLink::new(color, false) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[bot {label}] réseau indisponible : {e}");
            return;
        }
    };
    println!("[bot {label}] démarré — je fais tourner le VRAI protocole, sans fenêtre 3D.");

    let mut holes: HashMap<PeerId, bool> = HashMap::new();
    let mut buckets: HashMap<SocketAddr, f32> = HashMap::new();
    let mut relay_credits: HashMap<PeerId, f32> = HashMap::new();
    // Dernière position acceptée de chaque pair (+ instant) : pour valider le mouvement.
    let mut last_state: HashMap<PeerId, (Vec3, f32)> = HashMap::new();
    let mut orb = Orb::headless();
    let mut seq: u64 = 0;

    let mut hello_acc = HELLO_PERIOD;
    let mut punch_acc = 0.0f32;
    let mut send_acc = 0.0f32;
    let mut summary_acc = 0.0f32;
    let mut wander = 0.0f32;
    let mut last_pos: Option<Vec3> = None;
    let mut warned_version = false;

    let mut accepted: u64 = 0;
    let mut rejected: u64 = 0;
    let mut relayed: u64 = 0;

    let start = Instant::now();
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();

        wander += dt * 0.6;
        let pos = Vec3::new(WANDER_RADIUS * wander.cos(), 0.7, WANDER_RADIUS * wander.sin());

        // 1) Battement HELLO vers le rendez-vous (porte notre identité = notre clé).
        hello_acc += dt;
        if hello_acc >= HELLO_PERIOD {
            hello_acc = 0.0;
            let hello = encode_hello(pos.x, pos.z, link.identity.id());
            let _ = link.socket.send_to(link.rendezvous, &hello);
        }

        // 2) Recharge des seaux (rate-limit) + budget de relais ; éviction si trop d'adresses.
        for credit in buckets.values_mut() {
            *credit = (*credit + dt * BUCKET_RATE).min(BUCKET_CAP);
        }
        for credit in relay_credits.values_mut() {
            *credit = (*credit + dt * RELAY_RATE).min(RELAY_CAP);
        }
        if buckets.len() > MAX_BUCKETS {
            buckets.retain(|_, c| *c < BUCKET_CAP);
        }

        // 3) On relève le courrier et on applique LES VRAIES décisions de confiance.
        let inbox = link.socket.poll();
        for (from, bytes) in inbox {
            let credit = buckets.entry(from).or_insert(BUCKET_CAP);
            if *credit < 1.0 {
                continue;
            }
            *credit -= 1.0;

            if bytes.len() >= 2 && bytes[1] != PROTO_VERSION {
                if !warned_version {
                    warned_version = true;
                    eprintln!("[bot {label}] ⚠ version protocole différente — paquets ignorés.");
                }
                continue;
            }

            match kind(&bytes) {
                // Annuaire : on note qu'on est inscrit + la liste des pairs.
                Some(KIND_WELCOME) => {
                    if let Some((_hue, roster)) = decode_welcome(&bytes) {
                        link.my_id = Some(link.identity.id());
                        for (id, addr) in &roster {
                            if !link.peers.contains_key(id) {
                                println!("[bot {label}] nouveau pair {}.", id.short());
                            }
                            link.peers.insert(*id, *addr);
                            holes.entry(*id).or_insert(false);
                        }
                        let present: HashSet<PeerId> = roster.iter().map(|(id, _)| *id).collect();
                        link.peers.retain(|id, _| present.contains(id));
                    }
                }
                Some(KIND_PUNCH) => {
                    if let Some(id) = decode_punch(&bytes) {
                        if holes.insert(id, true) != Some(true) {
                            println!("[bot {label}] trou ouvert avec {}.", id.short());
                        }
                    }
                }
                // État direct : sceau auto-certifié + anti-rejeu + réputation.
                Some(KIND_STATE) => {
                    if !sig_ok(&bytes) {
                        continue; // sceau invalide → jeté SANS accuser (anti-framing)
                    }
                    match decode_canonical(&bytes) {
                        Some(state) => {
                            if state.id.has_pow(POW_BITS) && !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                                let np = Vec3::new(state.x, state.y, state.z);
                                let teleport = match last_state.get(&state.id) {
                                    Some((prev, t)) => !move_plausible(*prev, np, now - t),
                                    None => false,
                                };
                                if teleport {
                                    link.punish(state.id, "téléport (vitesse impossible)");
                                    rejected += 1;
                                } else {
                                    last_state.insert(state.id, (np, now));
                                    holes.insert(state.id, true);
                                    accepted += 1;
                                }
                            }
                        }
                        None => {
                            if let Some(id) = claimed_id(&bytes) {
                                link.punish(id, "état signé impossible (NaN)");
                                rejected += 1;
                            }
                        }
                    }
                }
                // RELAY : on est PARENT → on vérifie puis on RECOPIE à tous nos voisins.
                Some(KIND_RELAY) => {
                    if !sig_ok(&bytes) {
                        continue;
                    }
                    if let Some(state) = decode_canonical(&bytes) {
                        if state.id.has_pow(POW_BITS) && !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                            let np = Vec3::new(state.x, state.y, state.z);
                            let teleport = match last_state.get(&state.id) {
                                Some((prev, t)) => !move_plausible(*prev, np, now - t),
                                None => false,
                            };
                            if teleport {
                                link.punish(state.id, "relais : téléport (vitesse impossible)");
                                rejected += 1;
                            } else {
                                last_state.insert(state.id, (np, now));
                                // 6.5 : budget de relais par protégé + plafond de ré-émission.
                                let rc = relay_credits.entry(state.id).or_insert(RELAY_CAP);
                                let mut n = 0u32;
                                if *rc >= 1.0 {
                                    *rc -= 1.0;
                                    let mut forward = bytes.clone();
                                    forward[0] = KIND_STATE;
                                    let targets: Vec<(PeerId, SocketAddr)> =
                                        link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                                    for (id, addr) in targets {
                                        if id != state.id {
                                            let _ = link.socket.send_to(addr, &forward);
                                            n += 1;
                                            if n as usize >= MAX_RELAY_FANOUT {
                                                break;
                                            }
                                        }
                                    }
                                }
                                accepted += 1;
                                relayed += n as u64;
                                if n > 0 {
                                    println!("[bot {label}] ↪ RELAY de {} recopié à {n} pairs (≤ fanout {MAX_RELAY_FANOUT}).", state.id.short());
                                }
                            }
                        }
                    }
                }
                // Orbe : sceau + autorité + borne de version.
                Some(KIND_ORB) => {
                    if !orb_sig_ok(&bytes) {
                        continue;
                    }
                    match decode_orb(&bytes) {
                        Some(w) => {
                            let owner = w.owner;
                            if owner.has_pow(POW_BITS) && !link.is_muted(owner) {
                                let claimer_pos = last_state.get(&owner).map(|(p, _)| *p);
                                match apply_incoming(&mut orb, w, now, claimer_pos) {
                                    OrbApply::Implausible => link.punish(owner, "orbe : saut de version aberrant"),
                                    OrbApply::NoContact => link.punish(owner, "orbe : revendiquée sans contact"),
                                    _ => {}
                                }
                            }
                        }
                        None => {
                            if let Some(id) = claimed_owner(&bytes) {
                                link.punish(id, "orbe : état signé impossible (NaN)");
                            }
                        }
                    }
                }
                // Accusation d'un témoin (6.7) : on agit au QUORUM (anti-framing).
                Some(KIND_ACCUSE) => {
                    if let Some((accuser, offender)) = decode_accuse(&bytes) {
                        if accuser.has_pow(POW_BITS) && accuser != offender && !link.is_muted(accuser) {
                            link.record_accusation(offender, accuser);
                        }
                    }
                }
                _ => {}
            }
        }

        // 4) On émet NOTRE état signé aux pairs dont le trou est ouvert.
        send_acc += dt;
        if send_acc >= 1.0 / SEND_HZ {
            let dt_send = send_acc;
            send_acc = 0.0;
            if let Some(my_id) = link.my_id {
                let velocity = match last_pos {
                    Some(prev) => (pos - prev) / dt_send.max(1e-3),
                    None => Vec3::ZERO,
                };
                last_pos = Some(pos);
                seq += 1;
                let (r, g, b) = link.my_color;
                let me = PlayerState {
                    id: my_id,
                    x: pos.x, y: pos.y, z: pos.z,
                    vx: velocity.x, vy: velocity.y, vz: velocity.z,
                    yaw: wander, pitch: 0.0,
                    r, g, b,
                    parent: None, seq,
                };
                let bytes = encode_signed(&me, &link.identity);
                let targets: Vec<(PeerId, SocketAddr)> =
                    link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                for (id, addr) in targets {
                    if *holes.get(&id).unwrap_or(&false) {
                        let _ = link.socket.send_to(addr, &bytes);
                    }
                }
            }
        }

        // 5) On perce les pairs dont le trou n'est pas encore ouvert.
        punch_acc += dt;
        if punch_acc >= PUNCH_PERIOD {
            punch_acc = 0.0;
            if let Some(my_id) = link.my_id {
                let punch = encode_punch(my_id);
                let targets: Vec<(PeerId, SocketAddr)> =
                    link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                for (id, addr) in targets {
                    if !*holes.get(&id).unwrap_or(&false) {
                        let _ = link.socket.send_to(addr, &punch);
                    }
                }
            }
        }

        // 6) Résumé périodique : le « ledger » observable.
        summary_acc += dt;
        if summary_acc >= SUMMARY_PERIOD {
            summary_acc = 0.0;
            let muted = link.strikes.keys().filter(|id| link.is_muted(**id)).count();
            let maitre = orb.owner.map(|o| o.short()).unwrap_or_else(|| "—".to_string());
            println!(
                "[bot {label}] t={now:.0}s | pairs={} | orbe: maître={maitre} v={} | acceptés={accepted} rejetés={rejected} relayés={relayed} muets={muted}",
                link.peers.len(), orb.version
            );
        }

        std::thread::sleep(TICK);
    }
}
