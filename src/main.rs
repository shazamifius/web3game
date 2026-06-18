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

    // `cargo run -- rendezvous`  lance le serveur d'annuaire (à lancer en premier).
    if mode == Some("rendezvous") {
        net::run_rendezvous();
        return;
    }

    // `cargo run -- attack <type>`  lance le PROGRAMME ATTAQUANT : de vrais paquets
    // malveillants sur de vraies sockets, pour prouver la robustesse.
    // Chap. 5 (neutralisées) : forge | replay | flood | orb-steal | orb-freeze.
    // Chap. 6 (encore RÉUSSIES — trous à fermer) : teleport | sybil | orb-creep | amplify.
    if mode == Some("attack") {
        let kind = args.get(2).map(String::as_str).unwrap_or("forge");
        net::run_attack(kind);
        return;
    }

    // `cargo run -- bot alice`  lance un CLIENT HEADLESS (le vrai protocole, sans
    // 3D) : sert à tester l'architecture à plusieurs sans GPU ni écran (chap. 6.0).
    // C'est la « victime » honnête face au programme attaquant.
    if mode == Some("bot") {
        let label = args.get(2).map(String::as_str).unwrap_or("1");
        net::run_bot(label);
        return;
    }

    // `cargo run -- nat-test alice`  rejoue le hole punching en texte (sans 3D),
    // pour le test NAT en namespaces réseau (voir tools/test-nat.sh).
    if mode == Some("nat-test") {
        let label = args.get(2).map(String::as_str).unwrap_or("client");
        net::run_nat_test(label);
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

    // Mode multijoueur : `cargo run -- a` (ou `b`, `play`…) sur le même PC.
    // Tous ces arguments lancent un CLIENT (l'identifiant vient du rendez-vous,
    // plus de rôle codé en dur). Sans argument, le jeu reste solo (aucun réseau).
    // `weak` = un client à faible upload : il émet son état via un parent (relais)
    // au lieu de le diffuser à tous (chapitre 4.1). Sert à tester le relais sans
    // avoir réellement une mauvaise connexion.
    let is_client = matches!(
        mode,
        Some("a") | Some("b") | Some("play") | Some("client") | Some("weak")
    );
    let is_weak = mode == Some("weak");
    if is_client {
        match net::NetLink::new(my_color, is_weak) {
            Ok(link) => {
                app.insert_resource(link)
                    .init_resource::<net::RemoteAvatars>()
                    .init_resource::<net::Holes>()
                    .init_resource::<net::Nameplates>()
                    // L'orbe partagée : sa sphère 3D et la ressource d'état (Startup),
                    // puis les systèmes qui la font vivre (Update). Client uniquement.
                    .add_systems(Startup, net::setup_orb)
                    .add_systems(
                        Update,
                        (
                            net::net_punch,
                            net::net_send,
                            net::net_receive,
                            net::net_interpolate,
                            world::apply_world_color,
                            // Les 4 systèmes de l'orbe s'enchaînent dans CET ordre
                            // (`.chain()`) : on saisit, puis on migre si besoin, puis
                            // on simule la physique, puis on émet — sinon Bevy les
                            // ordonnerait au hasard et on perdrait une frame entre eux.
                            (
                                net::orb_grab,
                                net::orb_migrate,
                                net::orb_simulate,
                                net::orb_send,
                            )
                                .chain(),
                            net::update_nameplates,
                        ),
                    );
            }
            Err(e) => eprintln!("Réseau désactivé ({e}) — le jeu démarre en solo."),
        }
    }

    app.run();
}
