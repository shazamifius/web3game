//! LE BOT HEADLESS : un client qui fait tourner le VRAI code réseau, SANS la 3D.
//!
//! # Pourquoi ce fichier existe (chapitre 6.0)
//! On ne peut pas tester le jeu « en vrai » à grande échelle : les clients sont des
//! fenêtres graphiques (un GPU par joueur, impossible d'en lancer 50). Or, pour
//! durcir l'architecture (chapitre 6), il faut POUVOIR PROUVER chaque correctif
//! sans écran et sans humain devant. Ce bot est la réponse : il rejoint le
//! rendez-vous, perce les NAT, émet son état SIGNÉ et — surtout — applique à la
//! réception EXACTEMENT les mêmes décisions de confiance que le jeu (sceau via
//! `sig_ok`, anti-rejeu via `accept_seq`, réputation via `add_strike`, autorité
//! d'orbe via `apply_incoming`, rate-limit par seau à jetons). Il imprime un
//! « ledger » : ce qu'il a accepté, refusé, relayé, et qui il a mis en sourdine.
//!
//! Ainsi, « rendez-vous + N bots honnêtes + 1 attaquant », entièrement en
//! terminaux, suffit à VOIR un trou s'ouvrir (aujourd'hui) puis se refermer (après
//! les étapes 6.1→6.5). C'est l'embryon de la simulation 55K du chapitre 6.8.
//!
//! Lancement (après `cargo run -- rendezvous`) :
//!   cargo run -- bot alice
//!   cargo run -- bot bob      (… et autant qu'on veut)
//!
//! Note d'architecture : la BOUCLE de réception est ici réécrite (orchestration),
//! mais TOUTES les décisions de confiance réutilisent les mêmes fonctions que le
//! jeu — la sécurité ne peut donc pas « diverger » entre le bot et le vrai client.
//! L'unification complète boucle/jeu viendra avec le durcissement (6.5).

use super::control::{decode_welcome, encode_hello};
use super::link::NetLink;
use super::message::{decode_canonical, encode_signed, sig_ok, PlayerState};
use super::orb::{apply_incoming, decode_orb, orb_sig_ok, Orb, OrbApply};
use super::punch::{decode_punch, encode_punch};
use super::skin::random_color;
use super::wire::{kind, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME, PROTO_VERSION};
use bevy::prelude::Vec3;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

// Miroir des réglages de réception du jeu (cf. netcode/receive.rs et state.rs).
// On les recopie ici pour rester FIDÈLE au comportement réel ; le chapitre 6.5
// (durcissement DoS) unifiera le rate-limit en un seul endroit.
const BUCKET_RATE: f32 = 150.0;
const BUCKET_CAP: f32 = 300.0;
const SEND_HZ: f32 = 20.0;
const HELLO_PERIOD: f32 = 1.0;
const PUNCH_PERIOD: f32 = 0.25;
const SUMMARY_PERIOD: f32 = 2.0;
const TICK: Duration = Duration::from_millis(50);
/// Rayon du petit cercle que le bot parcourt (pour produire un trafic plausible).
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

    // État local du bot (ce que les `Local`/ressources Bevy portent dans le jeu).
    let mut holes: HashMap<u8, bool> = HashMap::new();
    let mut buckets: HashMap<SocketAddr, f32> = HashMap::new();
    let mut orb = Orb::headless();
    let mut seq: u64 = 0;

    // Cadenceurs.
    let mut hello_acc = HELLO_PERIOD;
    let mut punch_acc = 0.0f32;
    let mut send_acc = 0.0f32;
    let mut summary_acc = 0.0f32;
    let mut wander = 0.0f32;
    let mut last_pos: Option<Vec3> = None;
    let mut warned_version = false;

    // Ledger (les compteurs observables qui prouvent les trous).
    let mut accepted: u64 = 0;
    let mut rejected: u64 = 0;
    let mut relayed: u64 = 0;

    let start = Instant::now();
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();

        // Position qui se balade en cercle (trafic honnête, vitesse plausible).
        wander += dt * 0.6;
        let pos = Vec3::new(WANDER_RADIUS * wander.cos(), 0.7, WANDER_RADIUS * wander.sin());

        // 1) Battement HELLO vers le rendez-vous (porte notre clé publique).
        hello_acc += dt;
        if hello_acc >= HELLO_PERIOD {
            hello_acc = 0.0;
            let hello = encode_hello(pos.x, pos.z, &link.identity.public());
            let _ = link.socket.send_to(link.rendezvous, &hello);
        }

        // 2) Recharge des seaux à jetons (rate-limit), comme net_receive.
        for credit in buckets.values_mut() {
            *credit = (*credit + dt * BUCKET_RATE).min(BUCKET_CAP);
        }

        // 3) On relève le courrier et on applique LES VRAIES décisions de confiance.
        let inbox = link.socket.poll();
        for (from, bytes) in inbox {
            // Rate-limit : ce paquet a-t-il un jeton ? Sinon, inondation → on jette.
            let credit = buckets.entry(from).or_insert(BUCKET_CAP);
            if *credit < 1.0 {
                continue;
            }
            *credit -= 1.0;

            // Garde de version protocole.
            if bytes.len() >= 2 && bytes[1] != PROTO_VERSION {
                if !warned_version {
                    warned_version = true;
                    eprintln!("[bot {label}] ⚠ version protocole différente — paquets ignorés.");
                }
                continue;
            }

            match kind(&bytes) {
                // Annuaire : notre id + la liste des pairs (avec leurs clés publiques).
                Some(KIND_WELCOME) => {
                    if let Some((your_id, _hue, roster)) = decode_welcome(&bytes) {
                        link.my_id = Some(your_id);
                        for (id, addr, pk) in &roster {
                            if !link.peers.contains_key(id) {
                                println!("[bot {label}] nouveau pair {id}.");
                            }
                            link.peers.insert(*id, *addr);
                            link.pubkeys.insert(*id, *pk);
                            holes.entry(*id).or_insert(false);
                        }
                        let present: std::collections::HashSet<u8> =
                            roster.iter().map(|(id, _, _)| *id).collect();
                        link.peers.retain(|id, _| present.contains(id));
                    }
                }
                // Un pair nous a percés : trou de retour ouvert.
                Some(KIND_PUNCH) => {
                    if let Some(id) = decode_punch(&bytes) {
                        if holes.insert(id, true) != Some(true) {
                            println!("[bot {label}] trou ouvert avec {id}.");
                        }
                    }
                }
                // État direct d'un pair : sceau + anti-rejeu + réputation (≡ check_packet).
                Some(KIND_STATE) => {
                    let Some(claimed) = bytes.get(2).copied() else { continue };
                    let Some(pk) = link.pubkeys.get(&claimed).copied() else { continue };
                    if !sig_ok(&bytes, &pk) {
                        continue; // sceau invalide → jeté SANS accuser (anti-framing)
                    }
                    match decode_canonical(&bytes) {
                        Some(state) => {
                            if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                                holes.insert(state.id, true);
                                accepted += 1;
                            }
                        }
                        None => {
                            link.add_strike(claimed, "état signé impossible (NaN)");
                            rejected += 1;
                        }
                    }
                }
                // RELAY : on est PARENT → on vérifie puis on RECOPIE à tous nos voisins.
                // C'est exactement ce comportement qui rend l'amplification possible
                // (1 paquet entrant → N sortants). On l'expose ici pour le PROUVER ;
                // 6.5 le bornera (consentement + AoI sur la rediffusion).
                Some(KIND_RELAY) => {
                    let Some(claimed) = bytes.get(2).copied() else { continue };
                    let Some(pk) = link.pubkeys.get(&claimed).copied() else { continue };
                    if !sig_ok(&bytes, &pk) {
                        continue;
                    }
                    if let Some(state) = decode_canonical(&bytes) {
                        if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                            let mut forward = bytes.clone();
                            forward[0] = KIND_STATE;
                            let targets: Vec<(u8, SocketAddr)> =
                                link.peers.iter().map(|(i, a)| (*i, *a)).collect();
                            let mut n = 0u32;
                            for (id, addr) in targets {
                                if id != state.id {
                                    let _ = link.socket.send_to(addr, &forward);
                                    n += 1;
                                }
                            }
                            accepted += 1;
                            relayed += n as u64;
                            if n > 0 {
                                println!("[bot {label}] ↪ RELAY de {} recopié à {n} pairs (amplification ×{n}).", state.id);
                            }
                        }
                    }
                }
                // Orbe : sceau + autorité + borne de version (≡ check_orb + apply_incoming).
                Some(KIND_ORB) => {
                    let Some(owner) = bytes.get(2).copied() else { continue };
                    let Some(pk) = link.pubkeys.get(&owner).copied() else { continue };
                    if !orb_sig_ok(&bytes, &pk) {
                        continue;
                    }
                    if let Some(w) = decode_orb(&bytes) {
                        if !link.is_muted(owner) {
                            if let OrbApply::Implausible = apply_incoming(&mut orb, w, now) {
                                link.add_strike(owner, "orbe : saut de version aberrant");
                            }
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
                    parent: 0, seq,
                };
                let bytes = encode_signed(&me, &link.identity);
                let targets: Vec<(u8, SocketAddr)> =
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
                let targets: Vec<(u8, SocketAddr)> =
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
            println!(
                "[bot {label}] t={now:.0}s | pairs={} | orbe: maître={:?} v={} | acceptés={accepted} rejetés={rejected} relayés={relayed} muets={muted}",
                link.peers.len(), orb.owner, orb.version
            );
        }

        std::thread::sleep(TICK);
    }
}
