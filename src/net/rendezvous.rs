//! LE RENDEZ-VOUS : le seul morceau qui n'est PAS du pair-à-pair.
//!
//! Ce petit serveur ne fait qu'une chose : présenter les joueurs entre eux. Quand
//! un client dit « HELLO », le serveur retient son adresse (qu'il LIT dans le
//! paquet reçu — pas besoin que le client la connaisse), lui attribue un
//! identifiant, et lui renvoie la liste de tous les autres. Ensuite, les clients
//! s'envoient leur état DIRECTEMENT, sans repasser par lui.
//!
//! Lancement :  cargo run -- rendezvous

use super::aoi::{dist2, keep_nearest, within_radius, MAX_NEIGHBORS};
use super::control::{decode_hello, encode_welcome};
use super::crypto::{PeerId, pow_bits};
use super::skin::random_hue;
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Plafond du nombre de clients que le rendez-vous garde en mémoire (chap. 9.5a, D21). Sa table
/// est indexée par ADRESSE SOURCE (lue dans le paquet), or une source UDP est usurpable → sans
/// borne, un flood de HELLO depuis des adresses bidon ferait croître la table jusqu'à épuiser la
/// RAM (le rendez-vous est notre SEUL point central → le protéger compte, D21). Large devant une
/// vraie instance ; l'éviction des silencieux (5 s) libère en continu.
/// *Limite ASSUMÉE (résidu) : une fois plein, un nouveau venu HONNÊTE peut être refusé tant que la
/// table est saturée de sources usurpées. La vraie parade = routabilité (handshake prouvant que la
/// source est réelle) → plus lourd, étape ultérieure (anti-spoofing). Ici on borne la MÉMOIRE
/// (anti-crash), pas encore l'éviction par usurpation.*
const MAX_CLIENTS: usize = 8192;

/// Admet-on ce HELLO dans la table du rendez-vous (chap. 9.5a) ? Un client DÉJÀ connu est toujours
/// admis (on rafraîchit). Un NOUVEAU n'est admis que s'il reste de la place sous le plafond. Pur →
/// testable sans lancer la boucle réseau.
fn should_admit(is_known: bool, current_len: usize, cap: usize) -> bool {
    is_known || current_len < cap
}

/// Ce que le rendez-vous retient d'un client : son id, sa dernière activité, sa
/// position (pour l'AoI), et son dernier nombre de voisins (pour ne logger qu'au
/// changement).
struct ClientInfo {
    id: PeerId, // identité (clé publique) du client : redistribuée aux autres
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

    loop {
        for (from, bytes) in socket.poll() {
            // HELLO porte la position du joueur (pour l'AoI) ET son identité (clé
            // publique). Depuis le chap. 6.1, le rendez-vous n'ATTRIBUE plus de numéro :
            // l'identité, c'est la clé. Il ne fait que présenter les joueurs entre eux.
            let Some((px, pz, id)) = decode_hello(&bytes) else {
                continue; // le rendez-vous ne comprend que HELLO
            };
            // 6.2 : une identité sans preuve de travail n'est même pas listée.
            if !id.has_pow(pow_bits()) {
                continue;
            }
            // 9.5a (D21) : borne MÉMOIRE. Un client déjà connu est rafraîchi ; un nouveau n'entre
            // que s'il reste de la place sous le plafond → un flood de sources usurpées ne peut
            // plus faire enfler la table sans fin (au pire il sature, l'éviction 5 s la draine).
            if !should_admit(clients.contains_key(&from), clients.len(), MAX_CLIENTS) {
                continue;
            }
            let pos = (px, pz);
            let now = Instant::now();
            // Nouveau venu (adresse jamais vue) ? On le signale une fois.
            let last_count = match clients.get(&from) {
                Some(info) => info.last_count,
                None => {
                    println!("Joueur {} rejoint ({from}).", id.short());
                    usize::MAX // force le log au premier roster
                }
            };

            // VOISINAGE BORNÉ (chap. 6.6) : pré-filtre grossier par rayon, puis on ne
            // garde que les MAX_NEIGHBORS pairs les PLUS PROCHES. C'est la borne
            // d'échelle : le WELCOME ne déborde plus (trou n°2) et chacun ne parle qu'à
            // ~K voisins → O(N·K) au lieu d'O(N²) (trou n°3). Le water-filling côté
            // client répartit ensuite le débit entre ces voisins.
            let cands: Vec<((PeerId, SocketAddr), f32)> = clients
                .iter()
                .filter(|(addr, info)| **addr != from && within_radius(info.pos, pos))
                .map(|(addr, info)| ((info.id, *addr), dist2(info.pos, pos)))
                .collect();
            let roster: Vec<(PeerId, SocketAddr)> = keep_nearest(cands, MAX_NEIGHBORS);

            if roster.len() != last_count {
                println!("Joueur {} : {} a portee.", id.short(), roster.len());
            }
            clients.insert(from, ClientInfo { id, seen: now, pos, last_count: roster.len() });
            let _ = socket.send_to(from, &encode_welcome(world_hue, &roster));
        }

        // On oublie les clients silencieux depuis plus de 5 s (déconnectés).
        let now = Instant::now();
        clients.retain(|addr, info| {
            let keep = now.duration_since(info.seen) < Duration::from_secs(5);
            if !keep {
                println!("Joueur {} parti ({addr}).", info.id.short());
            }
            keep
        });

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 9.5a — la politique d'admission borne la table : un NOUVEAU venu est refusé quand c'est
    /// plein (anti-DoS mémoire), mais un client DÉJÀ connu est toujours rafraîchi (il ne perd pas
    /// sa place parce que la table est pleine).
    #[test]
    fn admission_borne_la_table_mais_garde_les_connus() {
        // De la place : tout le monde entre.
        assert!(should_admit(false, 10, 8192)); // nouveau, place libre
        assert!(should_admit(true, 10, 8192)); // connu
        // Pleine : le nouveau est refusé, le connu reste admis.
        assert!(!should_admit(false, 8192, 8192)); // nouveau + plein → refusé
        assert!(should_admit(true, 8192, 8192)); // connu + plein → rafraîchi quand même
        assert!(!should_admit(false, 9000, 8192)); // au-delà du plafond aussi
    }
}
