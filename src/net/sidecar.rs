//! LE SIDECAR — le pont entre le cœur réseau Rust et Unreal Engine.
//!
//! # Pourquoi ce fichier existe (bascule Unreal, voir CONTRAT_SIDECAR.md)
//! Unreal est un CLIENT MINCE : il ne fait pas de réseau, il pousse SA position et lit
//! les avatars distants par une **socket locale TCP** (`127.0.0.1:47800`). Le cœur Rust
//! garde toute l'autorité (relais NAT, anti-triche, sceau) — on lui ajoute une SORTIE.
//!
//! # Palier 2 — le VRAI cœur branché
//! Le sidecar fait tourner un vrai nœud réseau (`Bot`, le code prouvé : gossip, relais,
//! anti-triche, AoI), piloté par la position qu'Unreal pousse (`PUSH_SELF`), et il expose
//! les avatars distants RÉELS reçus du réseau (`SNAPSHOT`). On REUTILISE `Bot` (anti-
//! divergence D2) avec deux crochets gatés (pose externe + puits d'avatars) — le défaut
//! bot/simu reste byte-pour-byte. Le `Bot` rejoint le rendez-vous (`RENDEZVOUS_ADDR`,
//! défaut `127.0.0.1:4000`).
//!
//! Lancer (après un `jeu rendezvous`) :  cargo run -- sidecar

use super::bot::Bot;
use super::message::PlayerState;
use bevy::prelude::Vec3;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// --- Le contrat (cf. CONTRAT_SIDECAR.md §3) -------------------------------
// UE → Rust (types < 128)
const HELLO: u8 = 1;
const PUSH_SELF: u8 = 2;
const PING: u8 = 3;
// Rust → UE (types ≥ 128)
const WELCOME: u8 = 128;
const SNAPSHOT: u8 = 129;
const PONG: u8 = 130;

/// Cadence d'émission des snapshots vers Unreal (miroir de `SEND_HZ` du netcode).
const SEND_HZ: f32 = 20.0;
/// Cadence de la boucle du cœur (on step le `Bot` à ~50 Hz ; il accumule lui-même ses périodes).
const LOOP_HZ: f32 = 50.0;
/// Adresse d'écoute par défaut (réglable par `SIDECAR_ADDR`).
const DEFAULT_ADDR: &str = "127.0.0.1:47800";

/// La pose que MON joueur (Unreal) nous a poussée en dernier — partagée entre le thread
/// lecteur (qui l'écrit) et la boucle du cœur (qui l'injecte dans le `Bot`).
#[derive(Clone, Copy, Default)]
struct SelfPose {
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    pitch: f32,
    updates: u64, // combien de PUSH_SELF reçus (preuve que le sens UE→Rust vit)
}

/// Point d'entrée : `cargo run -- sidecar`. Écoute, et sert un client Unreal à la fois.
pub fn run_sidecar() {
    let addr = std::env::var("SIDECAR_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[sidecar] impossible d'écouter sur {addr} : {e}");
            return;
        }
    };
    println!("[sidecar] palier 2 — j'écoute sur {addr} (TCP loopback). En attente d'Unreal…");

    // Un seul client à la fois (un joueur = un UE = un cœur). On boucle pour ré-accepter si
    // Unreal tombe et revient. Le `Bot` est (re)créé à chaque session UE (identité éphémère —
    // ⚠ dette : le sidecar devrait à terme utiliser l'identité PERSISTANTE, cf. CONTRAT §6).
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                println!("[sidecar] Unreal connecté depuis {peer}.");
                if let Err(e) = serve_client(stream) {
                    println!("[sidecar] session terminée ({e}).");
                }
                println!("[sidecar] Unreal déconnecté — en attente d'une nouvelle connexion…");
            }
            Err(e) => eprintln!("[sidecar] accept a échoué : {e}"),
        }
    }
}

/// Sert UN client Unreal : crée le nœud réseau, lit les messages d'Unreal dans un thread, et
/// fait tourner le cœur + émet les SNAPSHOT depuis ce thread-ci.
fn serve_client(stream: TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?; // pas de Nagle : latence mini (contrat §6)
    let reader_stream = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));
    let alive = Arc::new(AtomicBool::new(true));
    let pose = Arc::new(Mutex::new(SelfPose::default()));

    // Le VRAI nœud réseau : il rejoint le rendez-vous, perce, émet/reçoit comme le jeu.
    let mut bot = match Bot::new("sidecar", false, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[sidecar] réseau indisponible (prise non ouverte).");
            return Ok(());
        }
    };
    bot.enable_avatar_sink(); // on veut exposer les avatars distants complets à Unreal

    // WELCOME : ma couleur (l'id réel n'est connu qu'après le 1er WELCOME du rendez-vous → on
    // envoie l'id quand on l'a ; ici on met des zéros, Unreal ne s'en sert que pour l'affichage).
    let (r, g, b) = bot.my_color();
    let mut welcome = Vec::with_capacity(32 + 12);
    welcome.extend_from_slice(&[0u8; 32]);
    for f in [r, g, b] {
        welcome.extend_from_slice(&f.to_le_bytes());
    }
    {
        let mut w = writer.lock().unwrap();
        write_frame(&mut *w, WELCOME, &welcome)?;
    }

    // Thread LECTEUR : décode les trames d'Unreal (HELLO / PUSH_SELF / PING).
    let r_writer = Arc::clone(&writer);
    let r_alive = Arc::clone(&alive);
    let r_pose = Arc::clone(&pose);
    let reader = std::thread::spawn(move || {
        let mut rs = io::BufReader::new(reader_stream);
        loop {
            match read_frame(&mut rs) {
                Ok((HELLO, payload)) => {
                    let v = payload.get(0..2).map(|b| u16::from_le_bytes([b[0], b[1]])).unwrap_or(0);
                    println!("[sidecar] HELLO d'Unreal (version protocole {v}).");
                }
                Ok((PUSH_SELF, payload)) if payload.len() >= 20 => {
                    let f = |i: usize| f32::from_le_bytes(payload[i..i + 4].try_into().unwrap());
                    let mut p = r_pose.lock().unwrap();
                    *p = SelfPose {
                        x: f(0), y: f(4), z: f(8), yaw: f(12), pitch: f(16),
                        updates: p.updates + 1,
                    };
                }
                Ok((PING, payload)) if payload.len() >= 8 => {
                    let mut w = r_writer.lock().unwrap();
                    if write_frame(&mut *w, PONG, &payload[0..8]).is_err() {
                        break;
                    }
                }
                Ok(_) => {}
                Err(_) => break, // Unreal a fermé la socket
            }
        }
        r_alive.store(false, Ordering::SeqCst);
    });

    // Boucle du CŒUR : step le bot avec la pose d'Unreal, et émet un SNAPSHOT des avatars réels.
    let start = Instant::now();
    let mut last = Instant::now();
    let loop_tick = Duration::from_secs_f32(1.0 / LOOP_HZ);
    let mut send_acc = 0.0f32;
    let mut log_acc = 0.0f32;
    while alive.load(Ordering::SeqCst) {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();

        // 1) injecter la pose d'Unreal dans le nœud (il l'émettra sur le réseau).
        let p = *pose.lock().unwrap();
        bot.set_external_pose(Vec3::new(p.x, p.y, p.z), p.yaw, p.pitch);

        // 2) faire tourner le vrai protocole (émission/réception/relais/gossip).
        bot.step(dt, now);

        // 3) émettre un SNAPSHOT des avatars distants RÉELS vers Unreal, à 20 Hz.
        send_acc += dt;
        if send_acc >= 1.0 / SEND_HZ {
            send_acc = 0.0;
            let avatars = bot.avatars(now);
            let snap = encode_snapshot(&avatars);
            let mut w = writer.lock().unwrap();
            if write_frame(&mut *w, SNAPSHOT, &snap).is_err() {
                break; // Unreal est parti
            }
        }

        log_acc += dt;
        if log_acc >= 2.0 {
            log_acc = 0.0;
            let id = bot.id().map(|i| i.short()).unwrap_or_else(|| "—".to_string());
            println!(
                "[sidecar] t={now:.0}s | id={id} | avatars distants={} | PUSH_SELF reçus={}",
                bot.avatars(now).len(),
                p.updates
            );
        }
        std::thread::sleep(loop_tick);
    }
    alive.store(false, Ordering::SeqCst);
    let _ = reader.join();
    Ok(())
}

/// Sérialise un SNAPSHOT : `u16 count` + N×AvatarRec (76 o), miroir de `PlayerState` sans
/// `parent`/`seq`. Format exact = CONTRAT_SIDECAR.md §3.
fn encode_snapshot(avatars: &[PlayerState]) -> Vec<u8> {
    let mut p = Vec::with_capacity(2 + avatars.len() * 76);
    p.extend_from_slice(&(avatars.len() as u16).to_le_bytes());
    for a in avatars {
        p.extend_from_slice(a.id.bytes());
        for f in [a.x, a.y, a.z, a.vx, a.vy, a.vz, a.yaw, a.pitch, a.r, a.g, a.b] {
            p.extend_from_slice(&f.to_le_bytes());
        }
    }
    p
}

/// Écrit une trame : `[u32 LE longueur][u8 type][payload]` (longueur = type + payload).
fn write_frame<W: Write>(w: &mut W, ty: u8, payload: &[u8]) -> io::Result<()> {
    let len = (1 + payload.len()) as u32;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&[ty])?;
    w.write_all(payload)?;
    w.flush()
}

/// Lit une trame complète (bloquant). Renvoie `(type, payload)`. Erreur = socket fermée
/// ou trame aberrante (longueur nulle ou > 1 Mo → on coupe, anti-paquet-fou).
fn read_frame<R: Read>(r: &mut R) -> io::Result<(u8, Vec<u8>)> {
    let mut len_buf = [0u8; 4];
    r.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len == 0 || len > (1 << 20) {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "longueur de trame aberrante"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    let ty = body[0];
    Ok((ty, body[1..].to_vec()))
}
