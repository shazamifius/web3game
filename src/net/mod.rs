//! La couche réseau du jeu — faite main : transport UDP brut + intégration Bevy.
//! Aucune « boîte noire » : on voit chaque octet partir et revenir.
//!
//! # Organisation (un sous-fichier = une responsabilité)
//!   - `wire`       : le TYPE d'un paquet (1er octet) + le port du rendez-vous
//!   - `message`    : le format d'un état de joueur (`PlayerState`, encode/decode)
//!   - `control`    : les messages de l'annuaire (HELLO / WELCOME)
//!   - `crypto`     : signatures à clé publique (Ed25519) — la frontière de confiance
//!   - `aoi`        : Area of Interest (allocation de budget : qui reçoit quel débit)
//!   - `punch`      : hole punching (percer les NAT pour une connexion directe)
//!   - `orb`        : l'orbe partagée (objet du monde à maître unique + transfert)
//!   - `transport`  : la prise réseau UDP générique (`Socket`)
//!   - `rendezvous` : le serveur d'annuaire qui présente les joueurs entre eux
//!   - `skin`       : la couleur de skin aléatoire d'une session
//!   - `demo`       : le mode texte `net-demo` (observer les paquets sans la 3D)
//!   - `attack`     : un VRAI programme attaquant (`cargo run -- attack …`) qui prouve
//!                    la robustesse en envoyant de vrais paquets malveillants (chap. 5)
//!   - `link`       : `NetLink`, la ressource qui relie le réseau au jeu
//!   - `netcode/`   : le rattrapage de latence (interpolation, prédiction,
//!                    réconciliation, horloge adaptative)
//!
//! # Jouer à plusieurs (même PC)
//!   Terminal 1 :  nix-shell --run "cargo run -- rendezvous"
//!   Terminal 2 :  nix-shell --run "cargo run -- a"
//!   Terminal 3 :  nix-shell --run "cargo run -- b"   (… et autant qu'on veut)

mod aoi;
mod attack;
mod control;
mod crypto;
mod demo;
mod link;
mod message;
mod natdemo;
mod netcode;
mod orb;
mod punch;
mod rendezvous;
mod skin;
mod transport;
mod wire;

// L'API publique du module réseau, utilisée par le reste du jeu (main, player).
pub use attack::run_attack;
pub use demo::run_demo;
pub use link::NetLink;
pub use natdemo::run_nat_test;
pub use netcode::{
    net_interpolate, net_receive, net_send, update_nameplates, Nameplates, RemoteAvatars,
};
pub use orb::{orb_grab, orb_migrate, orb_send, orb_simulate, setup_orb};
pub use punch::{net_punch, Holes};
pub use rendezvous::run_rendezvous;
pub use skin::{random_color, MyColor};
