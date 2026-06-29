//! LE CŒUR RÉSEAU — fait main : transport UDP brut, engine-agnostique (aucun moteur 3D).
//! Aucune « boîte noire » : on voit chaque octet partir et revenir.
//!
//! # Organisation (un sous-fichier = une responsabilité)
//!   - `wire`       : le TYPE d'un paquet (1er octet) + le port du rendez-vous
//!   - `message`    : le format d'un état de joueur (`PlayerState`, encode/decode)
//!   - `control`    : les messages de l'annuaire (HELLO / WELCOME)
//!   - `gossip`     : « cartes de visite » — découverte décentralisée des pairs (8.1, D22)
//!   - `cell`       : RÉSUMÉ de cellule — percevoir une foule lointaine sans N flux (8.3, D22)
//!   - `crypto`     : signatures à clé publique (Ed25519) — la frontière de confiance
//!   - `aoi`        : Area of Interest (allocation de budget : qui reçoit quel débit)
//!   - `punch`      : hole punching (percer les NAT pour une connexion directe)
//!   - `orb`        : l'orbe partagée (objet du monde à maître unique + transfert)
//!   - `transport`  : la prise réseau UDP générique (`Socket`)
//!   - `probe`      : sonde système (temps CPU du thread, RAM crête) pour chiffrer le
//!                    coût RÉEL d'un nœud (chap. 7.4, via /proc — Linux)
//!   - `linkprobe`  : sonde de LIEN — type de NAT via STUN (cône perçable vs symétrique/
//!                    CGNAT) ; Phase 2 du plan réseau, base de la redondance adaptative
//!   - `rendezvous` : le serveur d'annuaire qui présente les joueurs entre eux
//!   - `skin`       : la couleur de skin aléatoire d'une session
//!   - `demo`       : le mode texte `net-demo` (observer les paquets sans la 3D)
//!   - `attack`     : un VRAI programme attaquant (`cargo run -- attack …`) qui prouve
//!                    la robustesse en envoyant de vrais paquets malveillants (chap. 5)
//!   - `link`       : `NetLink`, l'état réseau d'un nœud (table de pairs, réputation…)
//!
//! # Lancer (headless ; la 3D vit dans Unreal, branchée au mode `sidecar`)
//!   Terminal 1 :  nix-shell --run "cargo run -- rendezvous"
//!   Terminal 2 :  nix-shell --run "cargo run -- bot alice"
//!   Terminal 3 :  nix-shell --run "cargo run -- sidecar"

mod accuse;
mod anticheat;
mod aoi;
mod attack;
mod bot;
mod cell;
mod control;
mod coopsim;
mod crypto;
mod demo;
mod gossip;
mod link;
mod linkprobe;
mod lossbench;
mod message;
mod metrics;
mod natdemo;
mod orb;
mod probe;
mod punch;
mod rendezvous;
mod sidecar;
mod sim;
mod skin;
mod stars;
mod transport;
mod wire;

// L'API publique du cœur, utilisée par `main` (aiguillage des modes headless).
pub use attack::run_attack;
pub use bot::run_bot;
pub use coopsim::{run_coopsim, run_coopsim_bus};
pub use demo::run_demo;
pub use linkprobe::run_natcheck;
pub use lossbench::run_phase1;
pub use metrics::{run_agent, run_serve_config, run_stats};
pub use natdemo::run_nat_test;
pub use rendezvous::run_rendezvous;
pub use sidecar::run_sidecar;
pub use sim::{run_crowd, run_relay_loss, run_relay_test, run_sim};
pub use stars::{run_stars, run_stars_race};
