//! L'ACCUSATION SIGNÉE (chapitre 6.7) : la réputation PARTAGÉE entre nœuds.
//!
//! # L'idée (« Own + Shields », version décentralisée)
//! Jusqu'ici chaque nœud bannit localement ce qu'il VOIT tricher (chap. 5.4). Mais
//! un tricheur que JE n'ai pas vu reste audible pour moi. La solution : quand un
//! nœud bannit quelqu'un, il l'ANNONCE par une petite accusation signée. Les autres
//! n'y croient pas sur parole — sinon un menteur ferait bannir un innocent (framing).
//! Ils attendent un **quorum** d'accusateurs DISTINCTS (et chaque identité coûte une
//! preuve de travail, chap. 6.2) : pour piéger un innocent, il faudrait miner et
//! coordonner `ACCUSE_QUORUM` fausses identités → cher et difficile. C'est la version
//! légère, byzantine-tolérante, de la réputation EigenTrust.
//!
//! # Le paquet (KIND_ACCUSE)
//!   accusateur (sa clé) + accusé (sa clé), le tout SCELLÉ par l'accusateur.
//! Auto-certifiant comme les états : on vérifie le sceau contre la clé de
//! l'accusateur, embarquée dans le paquet. On ne porte PAS de raison détaillée :
//! l'accusation dit seulement « moi, X, j'ai banni Y » — le quorum fait le reste.

use super::crypto::{verify, Identity, PeerId, PUBKEY_LEN, SIG_LEN};
use super::wire::{KIND_ACCUSE, PROTO_VERSION};

// [0]=type [1]=version [2..34]=accusateur (clé) [34..66]=accusé (clé) + sceau (64).
const OFF_ACCUSER: usize = 2;
const OFF_OFFENDER: usize = OFF_ACCUSER + PUBKEY_LEN; // 34
const BODY: usize = OFF_OFFENDER + PUBKEY_LEN; // 66
/// Taille d'une accusation signée : corps (66) + sceau (64) = 130 octets.
pub(crate) const ACCUSE_SIZE: usize = BODY + SIG_LEN;

/// Fabrique une accusation : « moi (`identity`), j'ai banni `offender` ». Scellée.
pub(crate) fn encode_accuse(offender: PeerId, identity: &Identity) -> [u8; ACCUSE_SIZE] {
    let mut buf = [0u8; ACCUSE_SIZE];
    buf[0] = KIND_ACCUSE;
    buf[1] = PROTO_VERSION;
    buf[OFF_ACCUSER..OFF_ACCUSER + PUBKEY_LEN].copy_from_slice(identity.id().bytes());
    buf[OFF_OFFENDER..OFF_OFFENDER + PUBKEY_LEN].copy_from_slice(offender.bytes());
    let sig = identity.sign(&buf[..BODY]);
    buf[BODY..].copy_from_slice(&sig);
    buf
}

/// Vérifie le sceau et renvoie `(accusateur, accusé)` si l'accusation est authentique.
/// `None` si trop court, mauvais type/version, ou sceau qui ne colle pas à la clé de
/// l'accusateur embarquée (on ne fait jamais confiance sans preuve).
pub(crate) fn decode_accuse(buf: &[u8]) -> Option<(PeerId, PeerId)> {
    if buf.len() < ACCUSE_SIZE || buf[0] != KIND_ACCUSE || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut accuser = [0u8; PUBKEY_LEN];
    accuser.copy_from_slice(&buf[OFF_ACCUSER..OFF_ACCUSER + PUBKEY_LEN]);
    let mut offender = [0u8; PUBKEY_LEN];
    offender.copy_from_slice(&buf[OFF_OFFENDER..OFF_OFFENDER + PUBKEY_LEN]);
    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[BODY..ACCUSE_SIZE]);
    if !verify(&buf[..BODY], &sig, &accuser) {
        return None;
    }
    Some((PeerId::from_bytes(accuser), PeerId::from_bytes(offender)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accusation_se_verifie_et_porte_les_deux_identites() {
        let temoin = Identity::generate();
        let tricheur = Identity::generate().id();
        let buf = encode_accuse(tricheur, &temoin);
        let (accuser, offender) = decode_accuse(&buf).expect("sceau valide");
        assert_eq!(accuser, temoin.id());
        assert_eq!(offender, tricheur);
    }

    #[test]
    fn accusation_alteree_est_rejetee() {
        let temoin = Identity::generate();
        let mut buf = encode_accuse(Identity::generate().id(), &temoin);
        buf[OFF_OFFENDER] ^= 0xFF; // on change l'accusé après coup → sceau cassé
        assert!(decode_accuse(&buf).is_none());
    }

    #[test]
    fn accusation_au_sceau_d_un_autre_est_rejetee() {
        // Corps fabriqué par un imposteur qui se prétend être `temoin` : sceau invalide.
        let temoin = Identity::generate();
        let imposteur = Identity::generate();
        let mut buf = encode_accuse(Identity::generate().id(), &imposteur);
        // On remplace la clé de l'accusateur par celle de `temoin` (usurpation).
        buf[OFF_ACCUSER..OFF_ACCUSER + PUBKEY_LEN].copy_from_slice(temoin.id().bytes());
        assert!(decode_accuse(&buf).is_none());
    }
}
