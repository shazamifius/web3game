//! La couche réseau du jeu — faite main : transport UDP brut + intégration Bevy.
//! Aucune « boîte noire » : on voit chaque octet partir et revenir.
//!
//! # Organisation (un sous-fichier = une responsabilité)
//!   - `message`   : le format d'un paquet (`PlayerState`, encode/decode)
//!   - `transport` : la prise réseau UDP (`NetPeer`) — la « connexion »
//!   - `skin`      : la couleur de skin aléatoire d'une session
//!   - `demo`      : le mode texte `net-demo` (observer les paquets sans la 3D)
//!   - `link`      : `NetLink`, la ressource qui relie le réseau au jeu
//!   - `netcode/`  : le rattrapage de latence (interpolation, prédiction,
//!                   réconciliation, horloge adaptative)
//!
//! # Jouer à deux fenêtres (même PC)
//!   Terminal 1 :  nix-shell --run "cargo run -- a"
//!   Terminal 2 :  nix-shell --run "cargo run -- b"

mod demo;
mod link;
mod message;
mod netcode;
mod skin;
mod transport;

// L'API publique du module réseau, utilisée par le reste du jeu (main, player).
pub use demo::run_demo;
pub use link::NetLink;
pub use netcode::{net_interpolate, net_receive, net_send, RemoteAvatars};
pub use skin::{random_color, MyColor};
