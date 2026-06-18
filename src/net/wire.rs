//! LE TYPE D'UN PAQUET : son tout premier octet dit « de quel message il s'agit ».
//!
//! Maintenant qu'il y a plusieurs sortes de paquets (état d'un joueur, et
//! discussion avec le rendez-vous), on les distingue par cet octet de tête.

/// Port du serveur de rendez-vous (sur 127.0.0.1 pour l'instant).
pub(crate) const RENDEZVOUS_PORT: u16 = 4000;

/// VERSION du protocole applicatif. Le 2e octet de CHAQUE paquet (juste après le
/// KIND) la porte. On l'incrémente dès qu'un format de paquet change (l'ajout du
/// champ `parent` au chapitre 4.1 en était un). Un récepteur qui lit une version
/// différente REJETTE le paquet au lieu de le lire de travers : c'est ce qui
/// évite le « bonhomme invisible » causé par deux binaires de versions différentes.
pub(crate) const PROTO_VERSION: u8 = 1;

pub(crate) const KIND_STATE: u8 = 1; // pair → pair : un PlayerState
pub(crate) const KIND_HELLO: u8 = 2; // client → rendez-vous : « je suis là »
pub(crate) const KIND_WELCOME: u8 = 3; // rendez-vous → client : ton id + la liste des autres
pub(crate) const KIND_PUNCH: u8 = 4; // pair → pair : « j'ouvre mon trou NAT vers toi » (hole punching)
pub(crate) const KIND_ORB: u8 = 5; // maître → pairs : l'état de l'orbe partagée (objet du monde)
pub(crate) const KIND_RELAY: u8 = 6; // faible → parent : « recopie mon état à mes voisins à ma place »
pub(crate) const KIND_ACCUSE: u8 = 7; // témoin → pairs : « j'ai banni ce tricheur » (réputation partagée, 6.7)
pub(crate) const KIND_GOSSIP: u8 = 8; // pair → pairs : « cartes de visite » d'autres pairs (découverte décentralisée, 8.1)

/// Lit le type d'un paquet (son 1er octet), ou `None` s'il est vide.
pub(crate) fn kind(bytes: &[u8]) -> Option<u8> {
    bytes.first().copied()
}
