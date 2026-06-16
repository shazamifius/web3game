//! LE MODE DÉMO : observer les paquets en texte, sans la 3D.
//!
//! `cargo run -- net-demo a` (ou `b`) : deux sessions s'envoient une position qui
//! tourne en cercle et affichent ce qu'elles reçoivent. Utile pour voir le réseau
//! tout seul. (Le vrai jeu, lui, se lance avec `cargo run -- a` / `b`.)

use super::message::PlayerState;
use super::skin::random_color;
use super::transport::{ports_for_role, NetPeer};
use std::time::{Duration, Instant};

pub fn run_demo(role: &str) {
    let (local_port, remote_port, id) = ports_for_role(role);
    let peer = match NetPeer::bind(local_port, remote_port) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le port {local_port} : {e}");
            return;
        }
    };
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
        };
        if let Err(e) = peer.send(&me) {
            eprintln!("Envoi raté : {e}");
        }
        for other in peer.poll() {
            println!(
                "  ← reçu du joueur {} : x={:.2}  y={:.2}  z={:.2}",
                other.id, other.x, other.y, other.z
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}
