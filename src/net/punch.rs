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

use super::crypto::{PeerId, PUBKEY_LEN};
use super::link::NetLink;
use super::wire::{KIND_PUNCH, PROTO_VERSION};
use bevy::prelude::*;
use std::collections::HashMap;

/// Intervalle entre deux tentatives de perçage vers un même pair (s).
const PUNCH_INTERVAL: f32 = 0.25;
/// Au-delà de ce nombre d'essais sans réponse, on cesse de logguer (le perçage, lui,
/// continue jusqu'à `PUNCH_GIVEUP`).
const PUNCH_LOG_LIMIT: u32 = 8;
/// ABANDON DU PERÇAGE (chap. 8.1b, ferme D23) : au-delà de ce nombre d'essais jamais
/// corroborés (≈ `PUNCH_GIVEUP × PUNCH_INTERVAL` = 10 s), on CESSE de percer cette
/// adresse. Avant, on martelait à VIE → une carte de gossip empoisonnée pointant vers
/// une victime faisait arroser celle-ci pour toujours (réflexion). Désormais une telle
/// carte ne coûte que ~10 s de perçage, puis silence. *Contrepartie assumée : un pair
/// derrière NAT symétrique (qui ne répond jamais au perçage direct) est lui aussi
/// abandonné → il lui faudra un relais (D17, chapitre ultérieur), pas un perçage éternel.*
const PUNCH_GIVEUP: u32 = 40;

/// Faut-il ABANDONNER le perçage d'un trou jamais corroboré ? (chap. 8.1b) Vrai au-delà
/// de `PUNCH_GIVEUP` essais. Fonction PURE, partagée par le jeu ([net_punch]) et le bot
/// ([bot.rs]) → une seule règle, testée une fois (anti-divergence, cf. D2).
pub(crate) fn punch_abandoned(tries: u32) -> bool {
    tries >= PUNCH_GIVEUP
}

/// Taille d'un PUNCH : type + version + identité (clé, 32) = 34.
const PUNCH_SIZE: usize = 2 + PUBKEY_LEN;

/// Fabrique un paquet PUNCH : type + version + notre identité (clé).
pub(crate) fn encode_punch(my_id: PeerId) -> [u8; PUNCH_SIZE] {
    let mut b = [0u8; PUNCH_SIZE];
    b[0] = KIND_PUNCH;
    b[1] = PROTO_VERSION;
    b[2..2 + PUBKEY_LEN].copy_from_slice(my_id.bytes());
    b
}

/// Lit un paquet PUNCH : renvoie l'identité (clé) du pair qui cherche à nous joindre.
pub(crate) fn decode_punch(buf: &[u8]) -> Option<PeerId> {
    if buf.len() < PUNCH_SIZE || buf[0] != KIND_PUNCH || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut pk = [0u8; PUBKEY_LEN];
    pk.copy_from_slice(&buf[2..2 + PUBKEY_LEN]);
    Some(PeerId::from_bytes(pk))
}

/// L'état d'un « trou » vers un pair : confirmé ouvert ou non, nombre d'essais, et
/// le temps écoulé depuis le dernier essai (pour cadencer les tentatives).
pub(crate) struct HoleState {
    pub(crate) open: bool,
    tries: u32,
    acc: f32,
}

impl HoleState {
    /// Le perçage vers ce pair est-il ABANDONNÉ (jamais corroboré après `PUNCH_GIVEUP` essais) ?
    /// C'est le signal du repli relais (12.3 / D17) : on cesse de percer dans le vide, on route via
    /// le rendez-vous. Expose la sémantique sans révéler le compteur interne `tries`.
    pub(crate) fn abandoned(&self) -> bool {
        !self.open && punch_abandoned(self.tries)
    }
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
    pub(crate) map: HashMap<PeerId, HoleState>,
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
        // Trou ouvert (corroboré) OU abandonné (jamais corroboré → carte empoisonnée ou
        // NAT symétrique) : on ne perce plus. L'abandon est l'anti-réflexion du 8.1b.
        if hole.open || punch_abandoned(hole.tries) {
            continue;
        }
        hole.acc += dt;
        if hole.acc < PUNCH_INTERVAL {
            continue;
        }
        hole.acc = 0.0;
        hole.tries += 1;
        let _ = link.socket.send_to(*addr, &punch);

        if hole.tries <= PUNCH_LOG_LIMIT {
            println!("PUNCH vers le pair {} (essai {}) — j'ouvre mon trou de retour.", id.short(), hole.tries);
            if hole.tries == PUNCH_LOG_LIMIT {
                println!(
                    "Pair {} : toujours pas de réponse ; on continue (en silence) jusqu'à l'abandon (NAT symétrique ? → relais plus tard).",
                    id.short()
                );
            }
        } else if hole.tries == PUNCH_GIVEUP {
            println!(
                "Pair {} : jamais corroboré après {PUNCH_GIVEUP} essais — ABANDON du perçage (carte de gossip empoisonnée ? NAT symétrique ?). On cesse d'arroser cette adresse (anti-réflexion 8.1b).",
                id.short()
            );
        }
    }

    // On oublie les trous des pairs qui ont quitté l'annuaire.
    holes.map.retain(|id, _| link.peers.contains_key(id));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 8.1b (c) — on perce tant qu'on n'a pas atteint le seuil, puis on ABANDONNE. C'est
    /// ce qui borne dans le temps un perçage réfléchi vers une victime (carte empoisonnée).
    #[test]
    fn perçage_abandonné_au_seuil() {
        assert!(!punch_abandoned(0)); // premier essai : on perce
        assert!(!punch_abandoned(PUNCH_GIVEUP - 1)); // juste avant le seuil : on perce encore
        assert!(punch_abandoned(PUNCH_GIVEUP)); // au seuil : abandon
        assert!(punch_abandoned(PUNCH_GIVEUP + 100)); // au-delà : toujours abandonné
    }

    /// Aller-retour d'un PUNCH (sérialisation à la main) : ce qu'on encode se redécode.
    #[test]
    fn punch_survit_a_l_aller_retour() {
        let id = PeerId::from_bytes([42u8; PUBKEY_LEN]);
        assert_eq!(decode_punch(&encode_punch(id)), Some(id));
    }

    /// 12.3 — `HoleState::abandoned()` : vrai SEULEMENT si non-ouvert ET au-delà du seuil d'abandon.
    /// C'est le déclencheur du repli relais — un trou ouvert (perçage réussi) n'est JAMAIS « abandonné »,
    /// et un perçage encore en cours non plus (on attend, on ne relaie pas prématurément).
    #[test]
    fn hole_abandoned_seulement_si_ferme_et_au_seuil() {
        let mut h = HoleState::default();
        assert!(!h.abandoned()); // neuf : tries=0, on perce encore
        h.tries = PUNCH_GIVEUP - 1;
        assert!(!h.abandoned()); // juste avant le seuil : encore en cours
        h.tries = PUNCH_GIVEUP;
        assert!(h.abandoned()); // au seuil, trou fermé : ABANDONNÉ → repli relais
        h.open = true;
        assert!(!h.abandoned()); // trou OUVERT (perçage réussi) → jamais abandonné, même au seuil
    }
}
