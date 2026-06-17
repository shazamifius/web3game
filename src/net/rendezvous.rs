//! LE RENDEZ-VOUS : le seul morceau qui n'est PAS du pair-à-pair.
//!
//! Ce petit serveur ne fait qu'une chose : présenter les joueurs entre eux. Quand
//! un client dit « HELLO », le serveur retient son adresse (qu'il LIT dans le
//! paquet reçu — pas besoin que le client la connaisse), lui attribue un
//! identifiant, et lui renvoie la liste de tous les autres. Ensuite, les clients
//! s'envoient leur état DIRECTEMENT, sans repasser par lui.
//!
//! Lancement :  cargo run -- rendezvous

use super::aoi::is_neighbor;
use super::control::{decode_hello, encode_welcome};
use super::skin::random_hue;
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

pub fn run_rendezvous() {
    let socket = match Socket::bind(RENDEZVOUS_PORT) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le rendez-vous sur {RENDEZVOUS_PORT} : {e}");
            return;
        }
    };
    // La couleur de salle de CETTE session de serveur : tous les joueurs connectés
    // l'adopteront. Deux fenêtres de couleur différente = pas le même serveur.
    let world_hue = random_hue();
    println!(
        "Rendez-vous : écoute sur 127.0.0.1:{RENDEZVOUS_PORT} (couleur de salle : teinte {world_hue}°). En attente de joueurs…"
    );

    // Pour chaque client : son id, la dernière fois qu'on l'a vu, et sa case (AoI).
    let mut clients: HashMap<SocketAddr, (u8, Instant, (i8, i8))> = HashMap::new();
    let mut next_id: u8 = 1;

    loop {
        for (from, bytes) in socket.poll() {
            // HELLO porte la case du joueur ; on s'en sert pour l'Area of Interest.
            let Some(cell) = decode_hello(&bytes) else {
                continue; // le rendez-vous ne comprend que HELLO
            };
            let now = Instant::now();
            // Nouveau venu ? On lui donne le prochain identifiant libre.
            // `changed` = il a aussi changé de case depuis la dernière fois.
            let (id, changed) = match clients.get(&from) {
                Some((id, _, old_cell)) => (*id, *old_cell != cell),
                None => {
                    let id = next_id;
                    next_id = next_id.checked_add(1).unwrap_or(1);
                    println!("Joueur {id} rejoint ({from}).");
                    (id, true)
                }
            };
            clients.insert(from, (id, now, cell));

            // AREA OF INTEREST : on ne renvoie que les VOISINS (même case ou case
            // adjacente), pas tout le monde. C'est ça qui fait tenir la charge.
            let roster: Vec<(u8, SocketAddr)> = clients
                .iter()
                .filter(|(addr, (_, _, c))| **addr != from && is_neighbor(*c, cell))
                .map(|(addr, (id, _, _))| (*id, *addr))
                .collect();
            if changed {
                println!("Joueur {id} en case {cell:?} : {} voisin(s).", roster.len());
            }
            let _ = socket.send_to(from, &encode_welcome(id, world_hue, &roster));
        }

        // On oublie les clients silencieux depuis plus de 5 s (déconnectés).
        let now = Instant::now();
        clients.retain(|addr, (id, seen, _)| {
            let keep = now.duration_since(*seen) < Duration::from_secs(5);
            if !keep {
                println!("Joueur {id} parti ({addr}).");
            }
            keep
        });

        std::thread::sleep(Duration::from_millis(50));
    }
}
