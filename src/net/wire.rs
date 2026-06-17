//! LE TYPE D'UN PAQUET : son tout premier octet dit « de quel message il s'agit ».
//!
//! Maintenant qu'il y a plusieurs sortes de paquets (état d'un joueur, et
//! discussion avec le rendez-vous), on les distingue par cet octet de tête.

/// Port du serveur de rendez-vous (sur 127.0.0.1 pour l'instant).
pub(crate) const RENDEZVOUS_PORT: u16 = 4000;

pub(crate) const KIND_STATE: u8 = 1; // pair → pair : un PlayerState
pub(crate) const KIND_HELLO: u8 = 2; // client → rendez-vous : « je suis là »
pub(crate) const KIND_WELCOME: u8 = 3; // rendez-vous → client : ton id + la liste des autres
pub(crate) const KIND_PUNCH: u8 = 4; // pair → pair : « j'ouvre mon trou NAT vers toi » (hole punching)

/// Lit le type d'un paquet (son 1er octet), ou `None` s'il est vide.
pub(crate) fn kind(bytes: &[u8]) -> Option<u8> {
    bytes.first().copied()
}
