//! La couche réseau du jeu — faite main : transport UDP brut + intégration Bevy.
//! Aucune « boîte noire » : on voit chaque octet partir et revenir.
//!
//! # Organisation (un sous-fichier = une responsabilité)
//!   - `wire`       : le TYPE d'un paquet (1er octet) + le port du rendez-vous
//!   - `message`    : le format d'un état de joueur (`PlayerState`, encode/decode)
//!   - `control`    : les messages de l'annuaire (HELLO / WELCOME)
//!   - `aoi`        : Area of Interest (allocation de budget : qui reçoit quel débit)
//!   - `punch`      : hole punching (percer les NAT pour une connexion directe)
//!   - `transport`  : la prise réseau UDP générique (`Socket`)
//!   - `rendezvous` : le serveur d'annuaire qui présente les joueurs entre eux
//!   - `skin`       : la couleur de skin aléatoire d'une session
//!   - `demo`       : le mode texte `net-demo` (observer les paquets sans la 3D)
//!   - `link`       : `NetLink`, la ressource qui relie le réseau au jeu
//!   - `netcode/`   : le rattrapage de latence (interpolation, prédiction,
//!                    réconciliation, horloge adaptative)
//!
//! # Jouer à plusieurs (même PC)
//!   Terminal 1 :  nix-shell --run "cargo run -- rendezvous"
//!   Terminal 2 :  nix-shell --run "cargo run -- a"
//!   Terminal 3 :  nix-shell --run "cargo run -- b"   (… et autant qu'on veut)

mod aoi;
mod control;
mod demo;
mod link;
mod message;
mod natdemo;
mod netcode;
mod punch;
mod rendezvous;
mod skin;
mod transport;
mod wire;

// L'API publique du module réseau, utilisée par le reste du jeu (main, player).
pub use demo::run_demo;
pub use link::NetLink;
pub use natdemo::run_nat_test;
pub use netcode::{net_interpolate, net_receive, net_send, RemoteAvatars};
pub use punch::{net_punch, Holes};
pub use rendezvous::run_rendezvous;
pub use skin::{random_color, MyColor};
