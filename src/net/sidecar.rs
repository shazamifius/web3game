//! LE SIDECAR — le pont entre le cœur réseau Rust et Unreal Engine (palier 1).
//!
//! # Pourquoi ce fichier existe (bascule Unreal, voir CONTRAT_SIDECAR.md)
//! Unreal sera un CLIENT MINCE : il ne fait pas de réseau, il pousse SA position et
//! lit les avatars distants par une **socket locale TCP** (`127.0.0.1:47800`). Le cœur
//! Rust garde toute l'autorité (relais NAT, anti-triche, sceau) — on lui ajoute juste
//! une SORTIE, on ne le change pas (Règle 1).
//!
//! # Ce palier (PALIER 1 — preuve de vie + MESURE de latence)
//! Sidecar **bidon** : il n'y a PAS encore de vrai réseau ici. On fabrique 2-3 faux
//! avatars qui tournent en cercle, on les envoie en `SNAPSHOT` à 20 Hz, on accepte les
//! `PUSH_SELF` d'Unreal, et on répond au `PING` par un `PONG` immédiat. **Le but réel
//! n'est pas joli : c'est de CHIFFRER la latence/jitter de la socket** (l'inconnue de
//! fond) AVANT de brancher le vrai cœur au palier 2. Le transport ici est NON-JETABLE :
//! le palier 2 le réutilise tel quel, il remplace juste « faux avatars » par « vrais ».
//!
//! Lancer :  cargo run -- sidecar     (puis Unreal se connecte en client)

use super::skin::random_color;
use std::f32::consts::PI;
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

/// Cadence d'émission des snapshots (miroir de `SEND_HZ` du netcode).
const SEND_HZ: f32 = 20.0;
/// Adresse d'écoute par défaut (réglable par `SIDECAR_ADDR`).
const DEFAULT_ADDR: &str = "127.0.0.1:47800";

/// La pose que MON joueur (Unreal) nous a poussée en dernier. Au palier 2, c'est elle
/// qu'on émettra sur le réseau ; au palier 1 on se contente de la journaliser.
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
    println!("[sidecar] palier 1 — j'écoute sur {addr} (TCP loopback). En attente d'Unreal…");

    // Un seul client à la fois (un joueur = un UE = un cœur). On boucle pour ré-accepter
    // si Unreal tombe et revient (reconnexion propre : on garde notre identité).
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

/// Sert UN client Unreal : envoie WELCOME, lit ses messages dans un thread, et émet les
/// SNAPSHOT à 20 Hz depuis ce thread-ci. Toutes les ÉCRITURES passent par le même mutex
/// (sérialise les trames : pas d'entrelacement WELCOME/SNAPSHOT/PONG).
fn serve_client(stream: TcpStream) -> io::Result<()> {
    stream.set_nodelay(true)?; // pas de Nagle : on veut la latence la plus basse (contrat §6)
    let reader_stream = stream.try_clone()?;
    let writer = Arc::new(Mutex::new(stream));
    let alive = Arc::new(AtomicBool::new(true));
    let pose = Arc::new(Mutex::new(SelfPose::default()));

    // WELCOME : mon identité (clé pub) + ma couleur. Au palier 1 l'id est un faux stable ;
    // au palier 2 ce sera ma vraie clé persistante (a.key).
    let (r, g, b) = random_color();
    let mut welcome = Vec::with_capacity(32 + 12);
    let mut my_id = [0u8; 32];
    my_id[0] = 0xEE; // marqueur « MOI » (faux id du palier 1)
    welcome.extend_from_slice(&my_id);
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
                    // PONG immédiat, même nonce : c'est lui qui mesure le RTT côté UE.
                    let mut w = r_writer.lock().unwrap();
                    if write_frame(&mut *w, PONG, &payload[0..8]).is_err() {
                        break;
                    }
                }
                Ok(_) => {} // trame inconnue ou trop courte : on ignore (robustesse)
                Err(_) => break, // Unreal a fermé la socket
            }
        }
        r_alive.store(false, Ordering::SeqCst);
    });

    // Boucle ÉMETTRICE : un SNAPSHOT de faux avatars toutes les 50 ms, + un log/s de vie.
    let start = Instant::now();
    let tick = Duration::from_secs_f32(1.0 / SEND_HZ);
    let mut log_acc = 0.0f32;
    while alive.load(Ordering::SeqCst) {
        let t = start.elapsed().as_secs_f32();
        let snap = build_fake_snapshot(t);
        {
            let mut w = writer.lock().unwrap();
            if write_frame(&mut *w, SNAPSHOT, &snap).is_err() {
                break; // Unreal est parti
            }
        }
        log_acc += 1.0 / SEND_HZ;
        if log_acc >= 1.0 {
            log_acc = 0.0;
            let p = *pose.lock().unwrap();
            println!(
                "[sidecar] t={t:.0}s | 3 faux avatars émis à {SEND_HZ:.0} Hz | \
                 PUSH_SELF reçus={} (dernier: x={:.1} y={:.1} z={:.1} yaw={:.2} pitch={:.2})",
                p.updates, p.x, p.y, p.z, p.yaw, p.pitch
            );
        }
        std::thread::sleep(tick);
    }
    alive.store(false, Ordering::SeqCst);
    let _ = reader.join();
    Ok(())
}

/// Fabrique le payload d'un SNAPSHOT : 3 faux avatars qui tournent en cercle (rayons et
/// phases distincts). Format exact = CONTRAT_SIDECAR.md §3 (u16 count + N×AvatarRec 76 o).
fn build_fake_snapshot(t: f32) -> Vec<u8> {
    // (index, rayon, phase, couleur RVB)
    let avatars: [(u8, f32, f32, (f32, f32, f32)); 3] = [
        (0, 4.0, 0.0, (1.0, 0.25, 0.25)),
        (1, 6.0, 2.0, (0.25, 1.0, 0.35)),
        (2, 3.0, 4.0, (0.35, 0.5, 1.0)),
    ];
    let mut p = Vec::with_capacity(2 + avatars.len() * 76);
    p.extend_from_slice(&(avatars.len() as u16).to_le_bytes());
    let w = 0.5f32; // vitesse angulaire (rad/s)
    for (i, radius, phase, (r, g, b)) in avatars {
        let a = w * t + phase;
        let (sa, ca) = a.sin_cos();
        let x = radius * ca;
        let z = radius * sa;
        let y = 0.7;
        // vitesse analytique = dérivée du cercle (sert à l'interpolation côté UE)
        let vx = -radius * w * sa;
        let vz = radius * w * ca;
        let vy = 0.0;
        let yaw = a + PI / 2.0; // face dans le sens de la marche (tangente)
        let pitch = 0.0;
        let mut id = [0u8; 32];
        id[0] = i;
        id[1] = 0xA1; // marqueur « faux avatar palier 1 »
        p.extend_from_slice(&id);
        for f in [x, y, z, vx, vy, vz, yaw, pitch, r, g, b] {
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
