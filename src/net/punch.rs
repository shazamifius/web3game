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
use super::wire::{KIND_PUNCH, PROTO_VERSION};

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

// NOTE — LE SUIVI DES TROUS (qui est ouvert, qui requiert un relais, cadence du perçage)
// vivait dans un système client (`net_punch` + ressource `Holes`/`HoleState`). Le nœud
// headless (`Bot`) en porte sa PROPRE version (cf. `bot.rs` : `holes` + `relays_to_us` +
// la décision de repli relais), et le client Unreal ne perce pas lui-même (le cœur le fait
// dans son thread). Ne reste donc ici que la frontière WIRE pure du perçage (encode/decode
// + la règle d'abandon `punch_abandoned`), partagée par tous les chemins.

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
}
