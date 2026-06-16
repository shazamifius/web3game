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
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str);

    // `cargo run -- net-demo a`  lance la démo réseau en texte (sans la 3D).
    if mode == Some("net-demo") {
        let role = args.get(2).map(String::as_str).unwrap_or("a");
        net::run_demo(role);
        return;
    }

    // Couleur de skin aléatoire de CETTE session : le perso et le réseau
    // utiliseront la même. Choisie une fois, au démarrage.
    let my_color = net::random_color();

    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Salle arcade — vue première personne".into(),
            // app-id Wayland (et classe X11) : sert au compositeur (niri) à
            // reconnaître nos fenêtres, par ex. pour les ouvrir sur un bureau précis.
            name: Some("web3game".into()),
            ..default()
        }),
        ..default()
    }))
    // Fond sombre violacé (cohérent avec l'ambiance néon).
    .insert_resource(ClearColor(Color::srgb(0.02, 0.01, 0.05)))
    .insert_resource(net::MyColor(my_color.0, my_color.1, my_color.2))
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
    );

    // Mode multijoueur : `cargo run -- a` (ou `b`) sur le même PC.
    // Sans argument, le jeu reste solo (aucun réseau).
    if mode == Some("a") || mode == Some("b") {
        let role = mode.unwrap();
        match net::NetLink::new(role, my_color) {
            Ok(link) => {
                app.insert_resource(link)
                    .init_resource::<net::RemoteAvatars>()
                    .add_systems(
                        Update,
                        (net::net_send, net::net_receive, net::net_interpolate),
                    );
            }
            Err(e) => eprintln!("Réseau désactivé ({e}) — le jeu démarre en solo."),
        }
    }

    app.run();
}
