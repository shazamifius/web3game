//! LE RENDEZ-VOUS : le seul morceau qui n'est PAS du pair-à-pair.
//!
//! Ce petit serveur ne fait qu'une chose : présenter les joueurs entre eux. Quand
//! un client dit « HELLO », le serveur retient son adresse (qu'il LIT dans le
//! paquet reçu — pas besoin que le client la connaisse), lui attribue un
//! identifiant, et lui renvoie la liste de tous les autres. Ensuite, les clients
//! s'envoient leur état DIRECTEMENT, sans repasser par lui.
//!
//! Lancement :  cargo run -- rendezvous

use super::aoi::within_radius;
use super::control::{decode_hello, encode_welcome};
use super::skin::random_hue;
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Ce que le rendez-vous retient d'un client : son id, sa dernière activité, sa
/// position (pour l'AoI), et son dernier nombre de voisins (pour ne logger qu'au
/// changement).
struct ClientInfo {
    id: u8,
    seen: Instant,
    pos: (f32, f32),
    last_count: usize,
}

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

    let mut clients: HashMap<SocketAddr, ClientInfo> = HashMap::new();
    let mut next_id: u8 = 1;

    loop {
        for (from, bytes) in socket.poll() {
            // HELLO porte la position du joueur ; on s'en sert pour l'Area of Interest.
            let Some(pos) = decode_hello(&bytes) else {
                continue; // le rendez-vous ne comprend que HELLO
            };
            let now = Instant::now();
            // Nouveau venu ? On lui donne le prochain identifiant libre.
            let (id, last_count) = match clients.get(&from) {
                Some(info) => (info.id, info.last_count),
                None => {
                    let id = next_id;
                    next_id = next_id.checked_add(1).unwrap_or(1);
                    println!("Joueur {id} rejoint ({from}).");
                    (id, usize::MAX) // force le log au premier roster
                }
            };

            // Borne GROSSIÈRE de candidats : on ne garde que les joueurs dans un
            // très grand rayon (ici ça revient à « tout le monde » dans une salle).
            // Ce n'est PAS la règle de jeu : la vraie répartition (qui reçoit quel
            // débit) se fait côté client par water-filling. Personne n'est exclu.
            let roster: Vec<(u8, SocketAddr)> = clients
                .iter()
                .filter(|(addr, info)| **addr != from && within_radius(info.pos, pos))
                .map(|(addr, info)| (info.id, *addr))
                .collect();

            if roster.len() != last_count {
                println!("Joueur {id} : {} a portee.", roster.len());
            }
            clients.insert(from, ClientInfo { id, seen: now, pos, last_count: roster.len() });
            let _ = socket.send_to(from, &encode_welcome(id, world_hue, &roster));
        }

        // On oublie les clients silencieux depuis plus de 5 s (déconnectés).
        let now = Instant::now();
        clients.retain(|addr, info| {
            let keep = now.duration_since(info.seen) < Duration::from_secs(5);
            if !keep {
                println!("Joueur {} parti ({addr}).", info.id);
            }
            keep
        });

        std::thread::sleep(Duration::from_millis(50));
    }
}
