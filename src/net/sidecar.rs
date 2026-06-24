//! LE SIDECAR — le pont entre le cœur réseau Rust et Unreal Engine.
//!
//! # Pourquoi ce fichier existe (bascule Unreal, voir CONTRAT_SIDECAR.md)
//! Unreal est un CLIENT MINCE : il ne fait pas de réseau, il pousse SA position et lit
//! les avatars distants par une **socket locale TCP** (`127.0.0.1:47800`). Le cœur Rust
//! garde toute l'autorité (relais NAT, anti-triche, sceau) — on lui ajoute une SORTIE.
//!
//! # Architecture (palier 2-3) — le CŒUR tourne en CONTINU
//! Un thread CŒUR fait tourner un vrai nœud réseau (`Bot`, code prouvé) **en permanence**,
//! qu'Unreal soit connecté ou non — le nœud est l'autorité, il ne doit pas s'éteindre quand
//! la fenêtre 3D se déconnecte. Le thread cœur publie en continu, dans un état partagé : les
//! avatars distants RÉELS, et lit la pose qu'Unreal pousse. Chaque session Unreal ne fait
//! qu'ATTACHER l'I/O (lire la pose, écrire les snapshots). On REUTILISE `Bot` (anti-divergence
//! D2) via 2 crochets gatés (pose externe + puits d'avatars) — défaut bot/simu byte-pour-byte.
//! Identité PERSISTANTE (`sidecar.key`) → nœud STABLE entre redémarrages.
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
/// Cadence de la boucle du cœur (on step le `Bot` à ~50 Hz ; il accumule ses propres périodes).
const LOOP_HZ: f32 = 50.0;
/// Adresse d'écoute par défaut (réglable par `SIDECAR_ADDR`).
const DEFAULT_ADDR: &str = "127.0.0.1:47800";

/// La pose que MON joueur (Unreal) a poussée en dernier — partagée entre la session UE (qui
/// l'écrit) et le thread cœur (qui l'injecte dans le `Bot`).
#[derive(Clone, Copy, Default)]
struct SelfPose {
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
    pitch: f32,
    updates: u64, // combien de PUSH_SELF reçus (preuve que le sens UE→Rust vit)
}

/// L'état partagé entre le thread CŒUR et les sessions Unreal.
struct Shared {
    avatars: Mutex<Vec<PlayerState>>, // dernier instantané des distants RÉELS (publié par le cœur)
    pose: Mutex<SelfPose>,            // la pose qu'Unreal pousse (lue par le cœur)
    color: Mutex<(f32, f32, f32)>,    // ma couleur (fixée par le cœur, lue au WELCOME)
}

/// Point d'entrée : `cargo run -- sidecar`. Démarre le cœur (continu) puis sert Unreal.
pub fn run_sidecar() {
    let addr = std::env::var("SIDECAR_ADDR").unwrap_or_else(|_| DEFAULT_ADDR.to_string());
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[sidecar] impossible d'écouter sur {addr} : {e}");
            return;
        }
    };

    let shared = Arc::new(Shared {
        avatars: Mutex::new(Vec::new()),
        pose: Mutex::new(SelfPose::default()),
        color: Mutex::new((0.5, 0.5, 0.5)),
    });

    // Le CŒUR tourne en continu, indépendamment d'Unreal.
    {
        let shared = Arc::clone(&shared);
        std::thread::spawn(move || run_core(shared));
    }

    println!("[sidecar] j'écoute sur {addr} (TCP loopback). Le cœur réseau tourne en continu.");
    // Une session Unreal à la fois ; on ré-accepte si elle tombe (le cœur, lui, ne s'arrête pas).
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream.peer_addr().map(|a| a.to_string()).unwrap_or_default();
                println!("[sidecar] Unreal connecté depuis {peer}.");
                if let Err(e) = serve_ue(stream, &shared) {
                    println!("[sidecar] session Unreal terminée ({e}).");
                }
                println!("[sidecar] Unreal déconnecté (le cœur continue) — en attente d'une reconnexion…");
            }
            Err(e) => eprintln!("[sidecar] accept a échoué : {e}"),
        }
    }
}

/// LE CŒUR : un vrai nœud réseau (`Bot`) qui tourne pour toujours, piloté par la pose d'Unreal,
/// publiant les avatars distants réels dans l'état partagé.
fn run_core(shared: Arc<Shared>) {
    let mut bot = match Bot::new_persistent("sidecar", "sidecar") {
        Some(b) => b,
        None => {
            eprintln!("[sidecar] réseau indisponible (prise non ouverte) — cœur non démarré.");
            return;
        }
    };
    bot.enable_avatar_sink();
    *shared.color.lock().unwrap() = bot.my_color();

    let start = Instant::now();
    let mut last = Instant::now();
    let loop_tick = Duration::from_secs_f32(1.0 / LOOP_HZ);
    let mut log_acc = 0.0f32;
    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();

        // 1) injecter la pose d'Unreal (le nœud l'émettra sur le réseau).
        let p = *shared.pose.lock().unwrap();
        bot.set_external_pose(Vec3::new(p.x, p.y, p.z), p.yaw, p.pitch);
        // 2) faire tourner le vrai protocole (émission/réception/relais/gossip).
        bot.step(dt, now);
        // 3) publier les avatars distants RÉELS pour la session Unreal.
        *shared.avatars.lock().unwrap() = bot.avatars(now);

        log_acc += dt;
        if log_acc >= 2.0 {
            log_acc = 0.0;
            let id = bot.id().map(|i| i.short()).unwrap_or_else(|| "—".to_string());
            println!(
                "[sidecar/cœur] t={now:.0}s | id={id} | pairs={} | avatars={} | acceptés={} rejetés={} relayés={} | PUSH_SELF={}",
                bot.neighbors(), bot.avatars(now).len(), bot.accepted(), bot.rejected(), bot.relayed(), p.updates
            );
        }
        std::thread::sleep(loop_tick);
    }
}

/// Sert UNE session Unreal : lit ses messages (thread lecteur) et émet les SNAPSHOT à 20 Hz
/// depuis l'état partagé. Quand Unreal se déconnecte, le cœur continue de tourner.
fn serve_ue(stream: TcpStream, shared: &Arc<Shared>) -> io::Result<()> {
    stream.set_nodelay(true)?; // pas de Nagle : latence mini (contrat §6)
    let reader_stream = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));
    let alive = Arc::new(AtomicBool::new(true));

    // WELCOME : ma couleur (publiée par le cœur). L'id réel n'est pas requis côté UE (affichage).
    let (r, g, b) = *shared.color.lock().unwrap();
    let mut welcome = Vec::with_capacity(32 + 12);
    welcome.extend_from_slice(&[0u8; 32]);
    for f in [r, g, b] {
        welcome.extend_from_slice(&f.to_le_bytes());
    }
    {
        let mut w = writer.lock().unwrap();
        write_frame(&mut *w, WELCOME, &welcome)?;
    }

    // Thread LECTEUR : HELLO / PUSH_SELF / PING.
    let r_writer = Arc::clone(&writer);
    let r_alive = Arc::clone(&alive);
    let r_shared = Arc::clone(shared);
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
                    let mut p = r_shared.pose.lock().unwrap();
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

    // Boucle ÉMETTRICE : un SNAPSHOT des avatars partagés à 20 Hz.
    let tick = Duration::from_secs_f32(1.0 / SEND_HZ);
    while alive.load(Ordering::SeqCst) {
        let snap = {
            let avatars = shared.avatars.lock().unwrap();
            encode_snapshot(&avatars)
        };
        {
            let mut w = writer.lock().unwrap();
            if write_frame(&mut *w, SNAPSHOT, &snap).is_err() {
                break;
            }
        }
        std::thread::sleep(tick);
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
