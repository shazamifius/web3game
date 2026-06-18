//! LE MODE DÉMO : observer les paquets en texte, sans la 3D ni le rendez-vous.
//!
//! `cargo run -- net-demo a` (ou `b`) : deux sessions à ports fixes s'envoient une
//! position qui tourne en cercle et affichent ce qu'elles reçoivent. C'est resté
//! volontairement simple (2 pairs codés en dur) pour observer le transport seul.

use super::message::{decode, encode, PlayerState};
use super::skin::random_color;
use super::transport::Socket;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub fn run_demo(role: &str) {
    let (local_port, remote_port, id) = ports_for_role(role);
    let socket = match Socket::bind(local_port) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le port {local_port} : {e}");
            return;
        }
    };
    let remote = SocketAddr::from(([127, 0, 0, 1], remote_port));
    let (r, g, b) = random_color();
    println!("Démo '{role}' : écoute {local_port}, parle à {remote_port}, joueur {id}.\n");

    let start = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        // Position sur un cercle, et la vitesse = sa dérivée (tangente au cercle).
        let me = PlayerState {
            id,
            x: t.cos() * 2.0,
            y: 0.7,
            z: t.sin() * 2.0,
            vx: -t.sin() * 2.0,
            vy: 0.0,
            vz: t.cos() * 2.0,
            yaw: t,
            pitch: 0.0,
            r,
            g,
            b,
            parent: 0,
            seq: 0,
        };
        if let Err(e) = socket.send_to(remote, &encode(&me)) {
            eprintln!("Envoi raté : {e}");
        }
        for (_from, bytes) in socket.poll() {
            if let Some(other) = decode(&bytes) {
                println!(
                    "  ← reçu du joueur {} : x={:.2}  y={:.2}  z={:.2}",
                    other.id, other.x, other.y, other.z
                );
            }
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Selon le rôle ('a' ou 'b'), choisit les ports et l'identifiant (démo seulement).
fn ports_for_role(role: &str) -> (u16, u16, u8) {
    match role {
        "b" | "B" => (5001, 5000, 2),
        _ => (5000, 5001, 1), // 'a' par défaut
    }
}
