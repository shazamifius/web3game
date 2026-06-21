//! Monde P2P en vue première personne (Rust/Bevy) — esprit « VRChat sans serveur ».
//! On spawn dans un HUB avec 2 portails ; on entre dans un portail pour choisir
//! l'instance : la salle arcade, ou l'île. (H : revenir au hub.)
//!
//! Contrôles :
//!   - ZQSD            : se déplacer (clavier AZERTY)
//!   - Espace          : sauter
//!   - H               : revenir au hub (depuis une instance)
//!   - Souris          : regarder autour
//!   - Échap           : libérer la souris
//!   - Clic gauche     : recapturer la souris
//!
//! Organisation :
//!   - `scenes` : le hub, les portails, l'aiguillage Hub/Arcade/Île
//!   - `world`  : la salle arcade (sol, murs, plafond, néon, lumière)
//!   - `player` : le personnage, la caméra et les contrôles

mod net;
mod player;
mod scenes;
mod world;

use bevy::prelude::*;
use scenes::Scene;

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

    // `cargo run -- sim [bots] [attaquants] [secondes]`  lance la SIMULATION MASSIVE
    // (chap. 6.8) : N nœuds headless + M attaquants en threads, et un rapport agrégé.
    if mode == Some("sim") {
        let n_bots = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
        let n_attackers = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
        let secs = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(15);
        net::run_sim(n_bots, n_attackers, secs);
        return;
    }

    // `cargo run -- crowd <N> [secondes]`  lance une FOULE DENSE de N nœuds au même
    // endroit (chap. 8.0) et mesure la COUVERTURE DE PERCEPTION — le mur D22 : au-delà
    // de 32 voisins, on est AVEUGLE. Sert à CHIFFRER le problème avant de le résoudre.
    if mode == Some("crowd") {
        let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
        let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
        net::run_crowd(n, secs);
        return;
    }

    // `cargo run -- coopsim <N> [secondes]`  lance N nœuds dans UN thread coopératif
    // (banc léger D25 : pas d'OS-thread/bot) pour mesurer la foule au-delà de ~1500.
    if mode == Some("coopsim") {
        let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
        let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
        net::run_coopsim(n, secs);
        return;
    }

    // `cargo run -- coopsim-bus <N> [secondes]`  banc BUS MÉMOIRE (dette D25) : N nœuds reliés
    // par un bus synchrone, dt fixe sans sleep → temps-sim découplé du mural, pour viser 5k-50k.
    if mode == Some("coopsim-bus") {
        let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
        let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
        net::run_coopsim_bus(n, secs);
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
    // Aiguillage de scènes (hub / arcade / île). On démarre dans le hub, sauf si
    // `SCENE=arcade|ile` (auto-test 3D / foule-3d qui veut sauter le hub).
    .insert_state(scenes::initial_scene())
    // Le joueur est créé UNE fois et survit aux changements de scène ; chaque scène
    // est montée/démontée par OnEnter/OnExit et le joueur est téléporté à son spawn.
    .add_systems(Startup, (player::setup_player, player::grab_cursor))
    .add_systems(OnEnter(Scene::Hub), (scenes::setup_hub, scenes::enter_hub_player))
    .add_systems(OnExit(Scene::Hub), scenes::despawn_world)
    .add_systems(OnEnter(Scene::Arcade), (world::setup_room, scenes::enter_arcade_player))
    .add_systems(OnExit(Scene::Arcade), scenes::despawn_world)
    .add_systems(OnEnter(Scene::Island), (scenes::setup_island, scenes::enter_island_player))
    .add_systems(OnExit(Scene::Island), scenes::despawn_world)
    .add_systems(
        Update,
        (
            player::move_player,
            player::jump_and_gravity,
            player::look_around,
            player::head_bob,
            player::toggle_cursor,
            // Portails : actifs dans le hub ; retour au hub (H) : actif ailleurs.
            scenes::portal_enter.run_if(in_state(Scene::Hub)),
            scenes::return_to_hub.run_if(not(in_state(Scene::Hub))),
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
        // Identité PERSISTANTE (chap. 10.1) : le profil = le mode (`a`/`b`/`play`…) → deux fenêtres
        // sur un même PC gardent des identités DISTINCTES *et* stables entre sessions (a.key ≠ b.key).
        let profile = mode.unwrap_or("player");
        match net::NetLink::new_persistent(my_color, is_weak, profile) {
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
                            // La recoloration n'a de sens qu'en arcade (où `WorldNeon` existe) :
                            // garde anti-panique quand on est au hub / sur l'île.
                            world::apply_world_color.run_if(resource_exists::<world::WorldNeon>),
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
