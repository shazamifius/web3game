//! Une simple salle cubique et un personnage en vue à la première personne.
//!
//! Contrôles :
//!   - ZQSD            : se déplacer (clavier AZERTY)
//!   - Souris          : regarder autour
//!   - Échap           : libérer la souris
//!   - Clic gauche     : recapturer la souris

use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::prelude::*;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::f32::consts::FRAC_PI_2;

// --- Dimensions de la salle (en mètres) ---
const ROOM_SIZE: f32 = 10.0; // largeur et profondeur
const ROOM_HEIGHT: f32 = 4.0; // hauteur sol -> plafond

// --- Réglages du joueur ---
const MOVE_SPEED: f32 = 4.0; // mètres par seconde
const MOUSE_SENSITIVITY: f32 = 0.0015;
const BODY_RADIUS: f32 = 0.3; // rayon du corps (capsule)

/// Marqueur du joueur : porte la position et la rotation gauche/droite (lacet).
#[derive(Component)]
struct Player;

/// Marqueur de la tête/caméra : porte la rotation haut/bas (tangage).
#[derive(Component)]
struct PlayerCamera;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Salle — vue première personne".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_room, setup_player, grab_cursor))
        .add_systems(Update, (move_player, look_around, toggle_cursor))
        .run();
}

/// Construit la salle : sol, plafond, 4 murs, et une lumière au centre.
fn setup_room(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let half = ROOM_SIZE / 2.0;
    let thickness = 0.1; // épaisseur des parois

    // Couleurs légèrement différentes pour bien percevoir le volume.
    let floor_mat = materials.add(Color::srgb(0.35, 0.35, 0.40));
    let ceil_mat = materials.add(Color::srgb(0.55, 0.55, 0.60));
    let wall_mat = materials.add(Color::srgb(0.70, 0.68, 0.65));

    // Sol (juste sous y = 0).
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(ROOM_SIZE, thickness, ROOM_SIZE))),
        MeshMaterial3d(floor_mat),
        Transform::from_xyz(0.0, -thickness / 2.0, 0.0),
    ));

    // Plafond.
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(ROOM_SIZE, thickness, ROOM_SIZE))),
        MeshMaterial3d(ceil_mat),
        Transform::from_xyz(0.0, ROOM_HEIGHT + thickness / 2.0, 0.0),
    ));

    // Murs avant/arrière (étendus selon X, à z = ±half).
    for z in [-half, half] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(ROOM_SIZE, ROOM_HEIGHT, thickness))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(0.0, ROOM_HEIGHT / 2.0, z),
        ));
    }

    // Murs gauche/droite (étendus selon Z, à x = ±half).
    for x in [-half, half] {
        commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(thickness, ROOM_HEIGHT, ROOM_SIZE))),
            MeshMaterial3d(wall_mat.clone()),
            Transform::from_xyz(x, ROOM_HEIGHT / 2.0, 0.0),
        ));
    }

    // Lumière ponctuelle suspendue au centre, juste sous le plafond.
    commands.spawn((
        PointLight {
            intensity: 2_000_000.0,
            range: 40.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, ROOM_HEIGHT - 0.5, 0.0),
    ));
}

/// Crée le joueur : un corps en capsule, avec la caméra (la tête) au-dessus.
fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Corps : capsule classique d'environ 1,4 m (rayon 0,3 + cylindre 0,8).
    let body = meshes.add(Capsule3d::new(BODY_RADIUS, 0.8));
    let body_mat = materials.add(Color::srgb(0.20, 0.50, 0.90));

    commands
        .spawn((
            Player,
            Mesh3d(body),
            MeshMaterial3d(body_mat),
            // Centre du corps à 0,7 m : la capsule va donc de 0 (pieds) à 1,4 m.
            Transform::from_xyz(0.0, 0.7, 0.0),
        ))
        .with_children(|parent| {
            // La caméra (les yeux) à ~1,6 m, au-dessus du corps -> on voit
            // son propre corps en baissant le regard.
            parent.spawn((
                PlayerCamera,
                Camera3d::default(),
                Transform::from_xyz(0.0, 0.9, 0.0),
                // Lumière ambiante (portée par la caméra en Bevy 0.18) pour que
                // les murs ne soient pas tout noirs ; complète la lumière du plafond.
                AmbientLight {
                    brightness: 400.0,
                    ..default()
                },
            ));
        });
}

/// Déplacement horizontal du joueur avec ZQSD (positions physiques W/A/S/D).
fn move_player(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<&mut Transform, With<Player>>,
) {
    let Ok(mut transform) = player.single_mut() else {
        return;
    };

    // Avant/droite « à plat » : on ignore l'inclinaison verticale.
    let forward: Vec3 = transform.forward().into();
    let right: Vec3 = transform.right().into();
    let forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    let right = Vec3::new(right.x, 0.0, right.z).normalize_or_zero();

    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction += forward; // Z : avancer
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction -= forward; // S : reculer
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction -= right; // Q : gauche
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction += right; // D : droite
    }

    let direction = direction.normalize_or_zero();
    transform.translation += direction * MOVE_SPEED * time.delta_secs();

    // Bornage simple pour rester dans la salle (pas de vraie collision).
    let limit = ROOM_SIZE / 2.0 - BODY_RADIUS;
    transform.translation.x = transform.translation.x.clamp(-limit, limit);
    transform.translation.z = transform.translation.z.clamp(-limit, limit);
}

/// Rotation de la vue à la souris : lacet sur le corps, tangage sur la tête.
fn look_around(
    mouse_motion: Res<AccumulatedMouseMotion>,
    cursor: Query<&CursorOptions>,
    mut player: Query<&mut Transform, (With<Player>, Without<PlayerCamera>)>,
    mut camera: Query<&mut Transform, (With<PlayerCamera>, Without<Player>)>,
) {
    // On ne tourne que si la souris est capturée.
    let Ok(cursor) = cursor.single() else {
        return;
    };
    if cursor.grab_mode == CursorGrabMode::None {
        return;
    }

    let delta = mouse_motion.delta;
    if delta == Vec2::ZERO {
        return;
    }

    let (Ok(mut player_transform), Ok(mut camera_transform)) =
        (player.single_mut(), camera.single_mut())
    else {
        return;
    };

    // Gauche/droite : on tourne tout le corps autour de l'axe vertical.
    player_transform.rotate_y(-delta.x * MOUSE_SENSITIVITY);

    // Haut/bas : on incline seulement la tête, en bloquant aux quasi 90°.
    let (pitch, _, _) = camera_transform.rotation.to_euler(EulerRot::XYZ);
    let new_pitch =
        (pitch - delta.y * MOUSE_SENSITIVITY).clamp(-FRAC_PI_2 + 0.01, FRAC_PI_2 - 0.01);
    camera_transform.rotation = Quat::from_rotation_x(new_pitch);
}

/// Capture la souris au démarrage (curseur verrouillé et invisible).
fn grab_cursor(mut cursor: Query<&mut CursorOptions>) {
    if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Échap libère la souris ; un clic gauche la recapture.
fn toggle_cursor(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut cursor: Query<&mut CursorOptions>,
) {
    let Ok(mut cursor) = cursor.single_mut() else {
        return;
    };
    if keyboard.just_pressed(KeyCode::Escape) {
        cursor.grab_mode = CursorGrabMode::None;
        cursor.visible = true;
    }
    if mouse.just_pressed(MouseButton::Left) {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}
