//! HOLE PUNCHING : percer les NAT pour obtenir une connexion DIRECTE entre pairs.
//!
//! # Le problème
//! La box (NAT) jette tout paquet entrant « non sollicité ». Deux joueurs derrière
//! leur box ne peuvent donc pas se parler d'entrée : aucun n'a invité l'autre.
//!
//! # L'astuce
//! ENVOYER un paquet vers un pair ouvre, dans NOTRE box, un « trou de retour » sur
//! le port utilisé : tant que la conversation vit, ce qui revient par ce port est
//! laissé entrer. Si les DEUX pairs s'envoient un paquet l'un vers l'autre (presque)
//! en même temps, leurs premiers paquets MEURENT (le trou d'en face n'existe pas
//! encore) mais les suivants PASSENT : les deux trous sont ouverts → connexion.
//!
//! # Ici
//! Le paquet de perçage est un petit PUNCH (type + notre id). On le RÉPÈTE vers
//! chaque pair tant que le trou n'est pas confirmé ouvert : la répétition absorbe
//! le décalage de timing entre les deux pairs (pas besoin de tir parfaitement
//! synchronisé). Dès qu'on reçoit quoi que ce soit du pair (cf. `receive.rs`), le
//! trou est « ouvert » et on arrête de percer.

use super::link::NetLink;
use super::wire::KIND_PUNCH;
use bevy::prelude::*;
use std::collections::HashMap;

/// Intervalle entre deux tentatives de perçage vers un même pair (s).
const PUNCH_INTERVAL: f32 = 0.25;
/// Au-delà de ce nombre d'essais sans réponse, on cesse de logguer (mais on
/// continue d'essayer) : c'est sûrement un NAT symétrique → ce sera le rôle du
/// relais (TURN/supernœud), prévu plus tard.
const PUNCH_LOG_LIMIT: u32 = 8;

/// Fabrique un paquet PUNCH : type + notre identifiant.
pub(crate) fn encode_punch(my_id: u8) -> [u8; 2] {
    [KIND_PUNCH, my_id]
}

/// Lit un paquet PUNCH : renvoie l'identifiant du pair qui cherche à nous joindre.
pub(crate) fn decode_punch(buf: &[u8]) -> Option<u8> {
    if buf.len() < 2 || buf[0] != KIND_PUNCH {
        return None;
    }
    Some(buf[1])
}

/// L'état d'un « trou » vers un pair : confirmé ouvert ou non, nombre d'essais, et
/// le temps écoulé depuis le dernier essai (pour cadencer les tentatives).
pub(crate) struct HoleState {
    pub(crate) open: bool,
    tries: u32,
    acc: f32,
}

impl Default for HoleState {
    fn default() -> Self {
        // `acc` démarre à PUNCH_INTERVAL pour percer DÈS la première image.
        HoleState { open: false, tries: 0, acc: PUNCH_INTERVAL }
    }
}

/// Les trous vers chaque pair connu, par identifiant. C'est `receive.rs` qui passe
/// un trou à `open = true` quand un paquet du pair nous parvient.
#[derive(Resource, Default)]
pub struct Holes {
    pub(crate) map: HashMap<u8, HoleState>,
}

/// SYSTÈME : pour chaque pair dont le trou n'est PAS confirmé ouvert, envoyer un
/// PUNCH à intervalle régulier (la salve de perçage). Chaque PUNCH ouvre, dans
/// NOTRE box, le trou de retour vers ce pair. Quand le pair fait de même, les deux
/// trous coïncident et nos paquets commencent à passer.
pub fn net_punch(time: Res<Time>, link: Res<NetLink>, mut holes: ResMut<Holes>) {
    // Tant que le rendez-vous ne nous a pas donné d'id, on n'a personne à percer.
    let Some(my_id) = link.my_id else {
        return;
    };
    let dt = time.delta_secs();
    let punch = encode_punch(my_id);

    for (id, addr) in &link.peers {
        let hole = holes.map.entry(*id).or_default();
        if hole.open {
            continue; // trou déjà ouvert : plus besoin de percer
        }
        hole.acc += dt;
        if hole.acc < PUNCH_INTERVAL {
            continue;
        }
        hole.acc = 0.0;
        hole.tries += 1;
        let _ = link.socket.send_to(*addr, &punch);

        if hole.tries <= PUNCH_LOG_LIMIT {
            println!("PUNCH vers le pair {id} (essai {}) — j'ouvre mon trou de retour.", hole.tries);
            if hole.tries == PUNCH_LOG_LIMIT {
                println!(
                    "Pair {id} : toujours pas de réponse ; on continue en silence (NAT symétrique ? → relais plus tard)."
                );
            }
        }
    }

    // On oublie les trous des pairs qui ont quitté l'annuaire.
    holes.map.retain(|id, _| link.peers.contains_key(id));
}
