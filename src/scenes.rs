//! Le HUB et l'aiguillage de SCÈNES — l'esprit VRChat sans serveur : on spawn dans un
//! petit monde de base (le hub) avec des PORTAILS, et on ENTRE dans un portail pour
//! choisir l'instance (la salle arcade, ou l'île). On peut revenir au hub avec H.
//!
//! Trois scènes (`Scene`) : `Hub`, `Arcade`, `Island`. Une seule est « montée » à la fois.
//! La géométrie de chaque scène porte le marqueur `WorldGeometry` → on la dé-spawn d'un
//! coup en quittant la scène. Le JOUEUR, lui, n'est PAS marqué : il survit et on le
//! TÉLÉPORTE au point d'apparition de la nouvelle scène.
//!
//! Automatisation : la variable d'env `SCENE=arcade|ile` fait démarrer DIRECTEMENT dans
//! une scène (saute le hub) → `tools/foule-3d.sh` et les tests 3D ne sont pas gênés. Les
//! simulations headless (`sim`/`coopsim`/`crowd`/`bot`) ne lancent pas la 3D : elles
//! n'ont pas de hub, donc rien à changer pour elles.

use crate::player::{Player, Vertical, GROUND_Y};
use bevy::prelude::*;

/// L'instance où l'on se trouve. `Hub` au démarrage (sauf `SCENE=…`).
#[derive(States, Default, Clone, PartialEq, Eq, Hash, Debug)]
pub enum Scene {
    #[default]
    Hub,
    Arcade,
    Island,
}

/// Scène de départ : `Hub` par défaut, ou forcée par `SCENE=arcade|ile` (auto-test).
pub fn initial_scene() -> Scene {
    match std::env::var("SCENE").as_deref() {
        Ok("arcade") => Scene::Arcade,
        Ok("ile") | Ok("island") => Scene::Island,
        _ => Scene::Hub,
    }
}

/// Marqueur posé sur toute la géométrie d'une scène : on dé-spawn tout ça en SORTANT
/// (le joueur et l'orbe ne le portent pas → ils survivent au changement de monde).
#[derive(Component)]
pub struct WorldGeometry;

/// Un portail du hub : marcher dedans bascule vers `target`.
#[derive(Component)]
pub struct Portal {
    target: Scene,
}

const PORTAL_RADIUS: f32 = 1.1; // rayon (m) pour « entrer » dans un portail

/// Dé-spawn toute la géométrie de la scène qu'on quitte (branché sur chaque `OnExit`).
pub fn despawn_world(mut commands: Commands, q: Query<Entity, With<WorldGeometry>>) {
    for e in &q {
        commands.entity(e).despawn();
    }
}

// ----------------------------------------------------------------------------
// HUB
// ----------------------------------------------------------------------------
/// Monte le hub : une plateforme sombre + 2 portails néon (magenta = arcade, vert = île)
/// + un peu de lumière. Volontairement minimaliste (esthétique d'abord).
pub fn setup_hub(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Plateforme du hub (disque sombre carré, 10×10).
    let floor = materials.add(StandardMaterial {
        base_color: Color::srgb(0.03, 0.03, 0.06),
        perceptual_roughness: 0.9,
        ..default()
    });
    commands.spawn((
        WorldGeometry,
        Mesh3d(cube.clone()),
        MeshMaterial3d(floor),
        Transform::from_xyz(0.0, -0.05, 0.0).with_scale(Vec3::new(10.0, 0.1, 10.0)),
    ));

    // Grille néon douce au sol (cyan) pour l'ambiance.
    let neon = materials.add(emissive(0.0, 0.9, 1.0));
    let mut g = -5.0;
    while g <= 5.001 {
        commands.spawn((
            WorldGeometry,
            Mesh3d(cube.clone()),
            MeshMaterial3d(neon.clone()),
            Transform::from_xyz(0.0, 0.012, g).with_scale(Vec3::new(10.0, 0.012, 0.03)),
        ));
        commands.spawn((
            WorldGeometry,
            Mesh3d(cube.clone()),
            MeshMaterial3d(neon.clone()),
            Transform::from_xyz(g, 0.012, 0.0).with_scale(Vec3::new(0.03, 0.012, 10.0)),
        ));
        g += 1.0;
    }

    // Les 2 portails, devant le point d'apparition (le joueur regarde vers -Z).
    spawn_portal(&mut commands, &cube, &mut materials, Vec3::new(-2.6, 1.3, -3.5), (2.5, 0.3, 0.9), Scene::Arcade);
    spawn_portal(&mut commands, &cube, &mut materials, Vec3::new(2.6, 1.3, -3.5), (0.2, 1.0, 0.4), Scene::Island);

    // Lumière douce du hub.
    commands.spawn((
        WorldGeometry,
        PointLight {
            color: Color::srgb(0.9, 0.9, 1.0),
            intensity: 800_000.0,
            range: 40.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(0.0, 4.0, 0.0),
    ));
}

/// Un portail = un panneau émissif vertical (couleur distincte) + son volume d'entrée.
fn spawn_portal(
    commands: &mut Commands,
    cube: &Handle<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    color: (f32, f32, f32),
    target: Scene,
) {
    let glow = materials.add(emissive(color.0 * 2.0, color.1 * 2.0, color.2 * 2.0));
    // Le panneau lumineux (large, fin), qu'on traverse.
    commands.spawn((
        WorldGeometry,
        Portal { target },
        Mesh3d(cube.clone()),
        MeshMaterial3d(glow.clone()),
        Transform::from_translation(pos).with_scale(Vec3::new(1.8, 2.6, 0.12)),
    ));
    // Un petit socle au sol, même couleur, pour viser le portail.
    commands.spawn((
        WorldGeometry,
        Mesh3d(cube.clone()),
        MeshMaterial3d(glow),
        Transform::from_xyz(pos.x, 0.02, pos.z).with_scale(Vec3::new(2.0, 0.02, 1.4)),
    ));
}

/// Dans le hub uniquement : si le joueur entre dans le volume d'un portail, on bascule.
pub fn portal_enter(
    player: Query<&Transform, With<Player>>,
    portals: Query<(&Transform, &Portal)>,
    mut next: ResMut<NextState<Scene>>,
) {
    let Ok(p) = player.single() else {
        return;
    };
    for (pt, portal) in &portals {
        if p.translation.xz().distance(pt.translation.xz()) < PORTAL_RADIUS {
            next.set(portal.target.clone());
            return;
        }
    }
}

/// Hors hub : la touche H ramène au hub (on « sort » de l'instance).
pub fn return_to_hub(keyboard: Res<ButtonInput<KeyCode>>, mut next: ResMut<NextState<Scene>>) {
    if keyboard.just_pressed(KeyCode::KeyH) {
        next.set(Scene::Hub);
    }
}

// ----------------------------------------------------------------------------
// ÎLE (minimale pour l'instant — météorites & ramassage = pas suivants)
// ----------------------------------------------------------------------------
/// Monte une petite île : un sol d'herbe rond-ish (carré pour l'instant) sous un ciel
/// ouvert, avec un « soleil ». Pas encore de météorites — juste de quoi voir qu'on est
/// AILLEURS qu'en arcade quand on passe le portail vert.
pub fn setup_island(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // Sol de l'île (vert sombre), 11×11 pour rester dans le bornage actuel du joueur.
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.28, 0.12),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        WorldGeometry,
        Mesh3d(cube.clone()),
        MeshMaterial3d(grass),
        Transform::from_xyz(0.0, -0.25, 0.0).with_scale(Vec3::new(11.0, 0.5, 11.0)),
    ));

    // Une bordure de plage claire autour (juste un anneau de 4 barres sableuses).
    let sand = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.5, 0.32),
        perceptual_roughness: 1.0,
        ..default()
    });
    for (pos, size) in [
        (Vec3::new(0.0, -0.2, 5.5), Vec3::new(12.0, 0.45, 1.0)),
        (Vec3::new(0.0, -0.2, -5.5), Vec3::new(12.0, 0.45, 1.0)),
        (Vec3::new(5.5, -0.2, 0.0), Vec3::new(1.0, 0.45, 12.0)),
        (Vec3::new(-5.5, -0.2, 0.0), Vec3::new(1.0, 0.45, 12.0)),
    ] {
        commands.spawn((
            WorldGeometry,
            Mesh3d(cube.clone()),
            MeshMaterial3d(sand.clone()),
            Transform::from_translation(pos).with_scale(size),
        ));
    }

    // Le « soleil » : une lumière directionnelle chaude qui projette des ombres.
    commands.spawn((
        WorldGeometry,
        DirectionalLight {
            color: Color::srgb(1.0, 0.95, 0.85),
            illuminance: 12_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(6.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

// ----------------------------------------------------------------------------
// Téléportation du joueur au point d'apparition de chaque scène
// ----------------------------------------------------------------------------
pub fn enter_hub_player(q: Query<(&mut Transform, &mut Vertical), With<Player>>) {
    place_player(q, Vec3::new(0.0, GROUND_Y, 2.5));
}
pub fn enter_arcade_player(q: Query<(&mut Transform, &mut Vertical), With<Player>>) {
    place_player(q, Vec3::new(0.0, GROUND_Y, 0.0));
}
pub fn enter_island_player(q: Query<(&mut Transform, &mut Vertical), With<Player>>) {
    place_player(q, Vec3::new(0.0, GROUND_Y, 0.0));
}

/// Pose le joueur à `pos`, face à -Z, vitesse verticale remise à 0.
fn place_player(mut q: Query<(&mut Transform, &mut Vertical), With<Player>>, pos: Vec3) {
    if let Ok((mut t, mut v)) = q.single_mut() {
        t.translation = pos;
        t.rotation = Quat::IDENTITY;
        v.vy = 0.0;
    }
}

/// Matériau émissif (base noire + couleur qui « glow » avec le bloom).
fn emissive(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(r, g, b),
        ..default()
    }
}
