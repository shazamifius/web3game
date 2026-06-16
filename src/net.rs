//! La couche réseau du jeu — 100 % faite main, uniquement avec la bibliothèque
//! standard de Rust (`std::net`). Aucune dépendance, aucune « boîte noire » :
//! on voit exactement comment une position de joueur devient une suite d'octets,
//! part dans le réseau, et revient.
//!
//! # L'idée de ce premier jet
//! Deux sessions du jeu, sur le MÊME PC, s'envoient leur position en **UDP**.
//! - UDP = on jette un paquet vers une adresse, sans garantie qu'il arrive.
//!   C'est rapide (pas d'accusé de réception), parfait pour un jeu : si un
//!   paquet de position se perd, le suivant arrive 50 ms plus tard de toute façon.
//! - Chaque session « écoute » sur un port (sa boîte aux lettres) et « parle »
//!   vers le port de l'autre.
//!
//! # Comment l'essayer (deux terminaux)
//!   Terminal 1 :  nix-shell --run "cargo run -- net-demo a"
//!   Terminal 2 :  nix-shell --run "cargo run -- net-demo b"
//! Tu verras chaque session afficher les positions qu'elle REÇOIT de l'autre.
//!
//! # Le test qui compte vraiment (plus tard)
//! Tant que c'est du « localhost » nu, tout marche parfaitement et ça ne prouve
//! rien. Pour simuler un vrai mauvais réseau sur ta machine :
//!   sudo tc qdisc add dev lo root netem delay 100ms loss 5%
//! (et pour tout remettre normal :)
//!   sudo tc qdisc del dev lo root
//! Là tu verras des paquets arriver en retard ou disparaître — les vrais défis.

use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

// ============================================================================
// 1) LE MESSAGE : ce qu'un joueur envoie aux autres
// ============================================================================

/// L'état minimal d'un joueur qu'on transmet sur le réseau.
/// Pour l'instant : qui c'est (`id`) et où il est (`x`, `y`, `z`).
/// Plus tard on y ajoutera la rotation, l'animation, etc.
#[derive(Clone, Copy, Debug)]
pub struct PlayerState {
    pub id: u8,  // identifiant du joueur (1 octet = jusqu'à 255 joueurs, suffira pour commencer)
    pub x: f32,  // position sur l'axe gauche/droite
    pub y: f32,  // position sur l'axe haut/bas (la hauteur)
    pub z: f32,  // position sur l'axe avant/arrière
}

// Taille exacte d'un paquet, en octets. On la calcule à la main pour bien
// comprendre : 1 octet pour l'id + 3 nombres `f32` de 4 octets chacun.
//   1 + 4 + 4 + 4 = 13 octets.
const PACKET_SIZE: usize = 1 + 4 + 4 + 4;

/// Transforme un `PlayerState` en une suite d'octets prête à être envoyée.
/// C'est ça, « sérialiser » : passer d'une structure Rust à des octets bruts.
///
/// `to_le_bytes` découpe un nombre en ses 4 octets, en « little-endian » (LE).
/// Peu importe le sens exact ici ; ce qui compte : l'émetteur et le récepteur
/// doivent utiliser LE MÊME sens. On choisit LE des deux côtés, point.
fn encode(p: &PlayerState) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];
    buf[0] = p.id; // le 1er octet : l'identifiant
    buf[1..5].copy_from_slice(&p.x.to_le_bytes()); // octets 1 à 4 : x
    buf[5..9].copy_from_slice(&p.y.to_le_bytes()); // octets 5 à 8 : y
    buf[9..13].copy_from_slice(&p.z.to_le_bytes()); // octets 9 à 12 : z
    buf
}

/// L'opération inverse : à partir des octets reçus, reconstruire un `PlayerState`.
/// Renvoie `None` si le paquet est trop court ou abîmé — on ne fait JAMAIS
/// confiance aveuglément à ce qui vient du réseau.
fn decode(buf: &[u8]) -> Option<PlayerState> {
    // Si on a reçu moins d'octets que prévu, le paquet est invalide : on jette.
    if buf.len() < PACKET_SIZE {
        return None;
    }
    let id = buf[0];
    // `try_into()` transforme une tranche de 4 octets en tableau `[u8; 4]`.
    // Le `?` veut dire : « si ça rate, renvoie None tout de suite ».
    let x = f32::from_le_bytes(buf[1..5].try_into().ok()?);
    let y = f32::from_le_bytes(buf[5..9].try_into().ok()?);
    let z = f32::from_le_bytes(buf[9..13].try_into().ok()?);
    Some(PlayerState { id, x, y, z })
}

// ============================================================================
// 2) LE PAIR (PEER) : la prise réseau d'une session
// ============================================================================

/// Représente la connexion réseau d'UNE session de jeu.
/// - `socket` : notre « boîte aux lettres » UDP (on reçoit et on envoie par là).
/// - `remote` : l'adresse de l'autre joueur à qui on parle.
pub struct NetPeer {
    socket: UdpSocket,
    remote: SocketAddr,
}

impl NetPeer {
    /// Ouvre la prise réseau locale et mémorise à qui on parle.
    /// - `local_port`  : le port sur lequel CETTE session écoute.
    /// - `remote_port` : le port de l'AUTRE session, sur la même machine (127.0.0.1).
    pub fn bind(local_port: u16, remote_port: u16) -> std::io::Result<NetPeer> {
        // 127.0.0.1 = « moi-même » (localhost). On reste sur le même PC pour l'instant.
        let socket = UdpSocket::bind(("127.0.0.1", local_port))?;

        // TRÈS IMPORTANT : mode « non-bloquant ». Par défaut, lire le réseau MET
        // EN PAUSE le programme jusqu'à ce qu'un paquet arrive. Dans un jeu on ne
        // peut pas se figer : on veut juste « y a-t-il du courrier ? sinon tant pis,
        // on continue ». Ce réglage fait exactement ça.
        socket.set_nonblocking(true)?;

        let remote = SocketAddr::from(([127, 0, 0, 1], remote_port));
        Ok(NetPeer { socket, remote })
    }

    /// Envoie notre position à l'autre joueur. Un seul paquet, et on n'attend
    /// aucune confirmation (c'est ça, l'UDP : on lance et on oublie).
    pub fn send(&self, state: &PlayerState) -> std::io::Result<()> {
        let bytes = encode(state);
        self.socket.send_to(&bytes, self.remote)?;
        Ok(())
    }

    /// Relève la boîte aux lettres : récupère TOUS les paquets arrivés depuis le
    /// dernier appel, et renvoie les positions décodées. Ne bloque jamais (grâce
    /// au mode non-bloquant) : s'il n'y a rien, on renvoie une liste vide.
    pub fn poll(&self) -> Vec<PlayerState> {
        let mut received = Vec::new();
        let mut buf = [0u8; 64]; // tampon de lecture (un peu plus grand que nécessaire)

        // On boucle tant qu'il reste des paquets en attente.
        loop {
            match self.socket.recv_from(&mut buf) {
                // On a lu `n` octets venant de `_from`. On tente de les décoder.
                Ok((n, _from)) => {
                    if let Some(state) = decode(&buf[..n]) {
                        received.push(state);
                    }
                    // (si decode renvoie None, on ignore simplement ce paquet abîmé)
                }
                // `WouldBlock` = « la boîte est vide pour l'instant ». Ce n'est PAS
                // une erreur : c'est le signal normal pour arrêter de relever.
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // Toute autre erreur réseau : on arrête la relève pour ce tour.
                Err(_) => break,
            }
        }
        received
    }
}

// ============================================================================
// 3) LE MODE DÉMO : pour VOIR les paquets circuler, sans le jeu
// ============================================================================

/// Lance une session de démonstration réseau. On l'appelle depuis `main.rs`
/// avec : `cargo run -- net-demo a`  (ou `b` pour l'autre session).
///
/// Chaque session invente une position qui tourne en cercle, l'envoie à l'autre
/// 5 fois par seconde, et affiche tout ce qu'elle reçoit. Tu lances les deux et
/// tu vois la conversation réseau en direct.
pub fn run_demo(role: &str) {
    // Selon le rôle, on choisit nos ports et notre identifiant.
    // 'a' écoute sur 5000 et parle à 5001 ; 'b' fait l'inverse.
    let (local_port, remote_port, id) = match role {
        "b" | "B" => (5001u16, 5000u16, 2u8),
        _ => (5000u16, 5001u16, 1u8), // 'a' par défaut
    };

    let peer = match NetPeer::bind(local_port, remote_port) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le port {local_port} : {e}");
            eprintln!("(le port est peut-être déjà pris par une autre session ?)");
            return;
        }
    };

    println!("Session '{role}' prête.");
    println!("  J'écoute sur 127.0.0.1:{local_port}");
    println!("  Je parle  à 127.0.0.1:{remote_port}");
    println!("  Je suis le joueur n°{id}.");
    println!("  (Ctrl-C pour arrêter.)\n");

    // `Instant` = une horloge MONOTONE : elle ne recule jamais, même si l'heure
    // système est modifiée. C'est la bonne horloge pour mesurer du temps de jeu.
    let start = Instant::now();

    loop {
        let t = start.elapsed().as_secs_f32(); // secondes écoulées depuis le début

        // Position fictive : on tourne en cercle pour avoir un mouvement visible.
        let me = PlayerState {
            id,
            x: t.cos() * 2.0,
            y: 0.7,
            z: t.sin() * 2.0,
        };

        // 1) On envoie NOTRE position.
        if let Err(e) = peer.send(&me) {
            eprintln!("Envoi raté : {e}");
        }

        // 2) On relève le courrier et on affiche ce que l'AUTRE nous a envoyé.
        for other in peer.poll() {
            println!(
                "  ← reçu du joueur {} : x={:.2}  y={:.2}  z={:.2}",
                other.id, other.x, other.y, other.z
            );
        }

        // 3) On attend un peu avant le tour suivant : 5 messages par seconde.
        //    (200 ms, pour que l'affichage reste lisible à l'œil.)
        std::thread::sleep(Duration::from_millis(200));
    }
}
