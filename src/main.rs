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

mod player;
mod world;

use bevy::prelude::*;

fn main() {
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
