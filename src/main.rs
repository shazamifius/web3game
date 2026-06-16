//! Salle arcade néon, en vue à la première personne — base minimaliste en Rust/Bevy.
//!
//! Contrôles :
//!   - ZQSD            : se déplacer (clavier AZERTY)
//!   - Souris          : regarder autour
//!   - Échap           : libérer la souris
//!   - Clic gauche     : recapturer la souris
//!
//! Organisation :
//!   - `world`  : la salle (sol, murs, plafond, néon, lumière)
//!   - `player` : le personnage, la caméra et les contrôles

mod net;
mod player;
mod world;

use bevy::prelude::*;

fn main() {
    // Aiguillage en fonction des arguments de la ligne de commande.
    // `cargo run -- net-demo a`  lance la démo réseau (échange de paquets UDP)
    // au lieu du jeu. Pratique pour observer le réseau seul, sans la 3D.
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("net-demo") {
        // Le rôle ('a' ou 'b') décide des ports utilisés ; 'a' par défaut.
        let role = args.get(2).map(String::as_str).unwrap_or("a");
        net::run_demo(role);
        return; // on ne démarre PAS le jeu dans ce mode
    }

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Salle arcade — vue première personne".into(),
                ..default()
            }),
            ..default()
        }))
        // Fond sombre violacé (cohérent avec l'ambiance néon).
        .insert_resource(ClearColor(Color::srgb(0.02, 0.01, 0.05)))
        .add_systems(
            Startup,
            (world::setup_room, player::setup_player, player::grab_cursor),
        )
        .add_systems(
            Update,
            (
                player::move_player,
                player::look_around,
                player::head_bob,
                player::toggle_cursor,
            ),
        )
        .run();
}
