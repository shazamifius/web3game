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

/// Étiquette UI (texte) d'un portail : suit la position 3D du portail à l'écran
/// (même technique que les pseudos des avatars). `world` = point monde à projeter.
#[derive(Component)]
pub struct PortalLabel {
    world: Vec3,
}

const PORTAL_RADIUS: f32 = 1.1; // rayon (m) pour « entrer » dans un portail

// Couleurs de fond (ciel) par scène : sombre en hub/arcade, NUIT bleutée sur l'île.
const SKY_DARK: Color = Color::srgb(0.02, 0.01, 0.05);
const SKY_NIGHT: Color = Color::srgb(0.015, 0.02, 0.06);

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
    spawn_portal(&mut commands, &cube, &mut materials, Vec3::new(-2.6, 1.3, -3.5), (2.5, 0.3, 0.9), Scene::Arcade, "ARCADE");
    spawn_portal(&mut commands, &cube, &mut materials, Vec3::new(2.6, 1.3, -3.5), (0.2, 1.0, 0.4), Scene::Island, "ÎLE");

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

/// Un portail = un panneau émissif vertical (couleur distincte) + son volume d'entrée,
/// + une étiquette UI flottante (son nom) au-dessus.
#[allow(clippy::too_many_arguments)]
fn spawn_portal(
    commands: &mut Commands,
    cube: &Handle<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pos: Vec3,
    color: (f32, f32, f32),
    target: Scene,
    name: &str,
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
    // Le nom, en texte UI flottant juste au-dessus du panneau.
    commands.spawn((
        WorldGeometry,
        PortalLabel { world: pos + Vec3::Y * 1.7 },
        Text::new(name),
        TextFont { font_size: 26.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, ..default() },
    ));
}

/// Dans le hub : projette chaque étiquette de portail (sa position monde) sur l'écran
/// et l'y positionne (cachée si hors champ / derrière la caméra). Même procédé que les
/// pseudos d'avatars.
pub fn update_portal_labels(
    camera: Query<(&Camera, &GlobalTransform)>,
    mut labels: Query<(&PortalLabel, &mut Node, &mut Visibility)>,
) {
    let Ok((cam, cam_tf)) = camera.single() else {
        return;
    };
    for (label, mut node, mut vis) in &mut labels {
        match cam.world_to_viewport(cam_tf, label.world) {
            Ok(screen) => {
                *vis = Visibility::Visible;
                node.left = Val::Px(screen.x);
                node.top = Val::Px(screen.y);
            }
            Err(_) => *vis = Visibility::Hidden,
        }
    }
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
/// Monte une petite île ACCUEILLANTE (plus de « backrooms ») : une île ronde d'herbe
/// posée sur une grande MER bleue, sous un ciel ouvert, avec un soleil, du RELIEF (une
/// colline + des rochers) et quelques NUAGES qui flottent. Pas encore de météorites —
/// c'est le pas suivant ; ici on rend juste l'endroit agréable.
pub fn setup_island(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let disc = meshes.add(Cylinder::new(1.0, 1.0)); // rayon 1, hauteur 1 (mis à l'échelle)
    let ball = meshes.add(Sphere::new(1.0));

    // --- La MER : un grand disque bleu translucide, autour et sous l'île. ---
    let sea = materials.add(StandardMaterial {
        base_color: Color::srgb(0.10, 0.45, 0.75),
        perceptual_roughness: 0.2, // un peu brillante (eau)
        ..default()
    });
    commands.spawn((
        WorldGeometry,
        Mesh3d(disc.clone()),
        MeshMaterial3d(sea),
        Transform::from_xyz(0.0, -0.6, 0.0).with_scale(Vec3::new(120.0, 0.2, 120.0)),
    ));

    // --- L'ÎLE : un disque d'herbe (rayon ~6), bordé de sable clair. ---
    let sand = materials.add(StandardMaterial {
        base_color: Color::srgb(0.80, 0.74, 0.50),
        perceptual_roughness: 1.0,
        ..default()
    });
    commands.spawn((
        WorldGeometry,
        Mesh3d(disc.clone()),
        MeshMaterial3d(sand),
        Transform::from_xyz(0.0, -0.2, 0.0).with_scale(Vec3::new(13.0, 0.5, 13.0)), // plage
    ));
    let grass = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.50, 0.20),
        perceptual_roughness: 0.95,
        ..default()
    });
    commands.spawn((
        WorldGeometry,
        Mesh3d(disc.clone()),
        MeshMaterial3d(grass.clone()),
        Transform::from_xyz(0.0, -0.1, 0.0).with_scale(Vec3::new(11.0, 0.5, 11.0)), // herbe
    ));

    // --- RELIEF : une colline douce (sphère aplatie) au fond, + quelques rochers. ---
    commands.spawn((
        WorldGeometry,
        Mesh3d(ball.clone()),
        MeshMaterial3d(grass.clone()),
        Transform::from_xyz(0.0, 0.0, -3.5).with_scale(Vec3::new(4.0, 1.6, 4.0)),
    ));
    let rock = materials.add(StandardMaterial {
        base_color: Color::srgb(0.35, 0.35, 0.38),
        perceptual_roughness: 1.0,
        ..default()
    });
    for (x, z, s) in [(3.2_f32, 1.5_f32, 0.5_f32), (-3.0, 2.2, 0.7), (2.0, -2.8, 0.4)] {
        commands.spawn((
            WorldGeometry,
            Mesh3d(ball.clone()),
            MeshMaterial3d(rock.clone()),
            Transform::from_xyz(x, 0.0, z).with_scale(Vec3::splat(s)),
        ));
    }

    // --- ÉTOILES : une multitude de petits points blancs émissifs, semés sur un dôme
    // lointain et haut (positions déterministes via un petit xorshift → même ciel à
    // chaque visite, et zéro dépendance). Plus beau de NUIT, comme demandé. ---
    let star = materials.add(emissive(2.2, 2.3, 2.6)); // blanc bleuté qui « glow »
    let mut s: u32 = 0x51ED_3F17;
    let mut rnd = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        s as f32 / u32::MAX as f32 // [0,1)
    };
    for _ in 0..160 {
        // Point sur un grand dôme : angle autour + hauteur, rayon ~70.
        let ang = rnd() * std::f32::consts::TAU;
        let up = 0.15 + rnd() * 0.85; // surtout en hauteur
        let r = 70.0;
        let pos = Vec3::new(
            ang.cos() * r * (1.0 - up * 0.5),
            8.0 + up * 55.0,
            ang.sin() * r * (1.0 - up * 0.5),
        );
        let sz = 0.15 + rnd() * 0.22;
        commands.spawn((
            WorldGeometry,
            Mesh3d(ball.clone()),
            MeshMaterial3d(star.clone()),
            Transform::from_translation(pos).with_scale(Vec3::splat(sz)),
        ));
    }

    // --- CLAIR DE LUNE : lumière directionnelle douce et bleutée, avec ombres. ---
    commands.spawn((
        WorldGeometry,
        DirectionalLight {
            color: Color::srgb(0.70, 0.78, 1.0),
            illuminance: 3_500.0, // tamisé : c'est la nuit
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(8.0, 14.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    // Une LUNE visible (boule claire émissive, haut dans le ciel).
    let moon = materials.add(emissive(1.4, 1.5, 1.8));
    commands.spawn((
        WorldGeometry,
        Mesh3d(ball.clone()),
        MeshMaterial3d(moon),
        Transform::from_xyz(-22.0, 34.0, -28.0).with_scale(Vec3::splat(4.0)),
    ));
}

// ----------------------------------------------------------------------------
// Téléportation du joueur au point d'apparition de chaque scène
// ----------------------------------------------------------------------------
pub fn enter_hub_player(
    mut clear: ResMut<ClearColor>,
    q: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    clear.0 = SKY_DARK;
    place_player(q, Vec3::new(0.0, GROUND_Y, 2.5));
}
pub fn enter_arcade_player(
    mut clear: ResMut<ClearColor>,
    q: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    clear.0 = SKY_DARK;
    place_player(q, Vec3::new(0.0, GROUND_Y, 0.0));
}
pub fn enter_island_player(
    mut clear: ResMut<ClearColor>,
    q: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    clear.0 = SKY_NIGHT;
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
