//! Le joueur : un corps articulé simple (torse, tête, bras, jambes), une caméra
//! à hauteur des yeux, le déplacement ZQSD, la vue à la souris et un léger head-bob.

use crate::world::ROOM_SIZE;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::f32::consts::FRAC_PI_2;

const MOVE_SPEED: f32 = 4.0; // mètres par seconde
const MOUSE_SENSITIVITY: f32 = 0.0015;
const BODY_RADIUS: f32 = 0.25; // demi-largeur du corps (pour rester dans la salle)
const EYE_HEIGHT: f32 = 0.9; // hauteur des yeux au-dessus du centre du corps

/// Joueur : porte la position et la rotation gauche/droite (lacet).
#[derive(Component)]
pub struct Player;

/// Tête/caméra : porte la rotation haut/bas (tangage) et le head-bob.
#[derive(Component)]
pub struct PlayerCamera;

/// Crée le joueur (corps articulé) et sa caméra première personne.
pub fn setup_player(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    my_color: Res<crate::net::MyColor>,
) {
    let torso = meshes.add(Capsule3d::new(0.17, 0.45));
    let head = meshes.add(Sphere::new(0.14));
    let limb = meshes.add(Capsule3d::new(0.07, 0.40));

    // Couleur de skin aléatoire (la même qu'on envoie sur le réseau). On la
    // décline en 3 nuances pour garder un peu de relief : torse/bras à la
    // couleur de base, tête plus vive, jambes plus sombres.
    let (r, g, b) = (my_color.0, my_color.1, my_color.2);
    let cyan = materials.add(body_mat(r, g, b)); // torse + bras
    let magenta = materials.add(body_mat(r * 1.3, g * 1.3, b * 1.3)); // tête (plus vive)
    let violet = materials.add(body_mat(r * 0.55, g * 0.55, b * 0.55)); // jambes (plus sombres)

    commands
        .spawn((
            Player,
            // Centre du corps à 0,7 m : les pieds arrivent ~au sol.
            Transform::from_xyz(0.0, 0.7, 0.0),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Torse
            p.spawn((
                Mesh3d(torso),
                MeshMaterial3d(cyan.clone()),
                Transform::from_xyz(0.0, 0.10, 0.0),
            ));
            // Tête
            p.spawn((
                Mesh3d(head),
                MeshMaterial3d(magenta),
                Transform::from_xyz(0.0, 0.62, 0.0),
            ));
            // Bras (gauche / droit)
            for x in [-0.30, 0.30] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(cyan.clone()),
                    Transform::from_xyz(x, 0.08, 0.0),
                ));
            }
            // Jambes (gauche / droite)
            for x in [-0.11, 0.11] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(violet.clone()),
                    Transform::from_xyz(x, -0.45, 0.0),
                ));
            }
            // Caméra (les yeux), au-dessus du corps : on se voit en baissant les yeux.
            p.spawn((
                PlayerCamera,
                Camera3d::default(),
                Transform::from_xyz(0.0, EYE_HEIGHT, 0.0),
                // HDR + bloom : le néon « glow ». Tonemapping pour de belles couleurs.
                Hdr,
                Bloom {
                    intensity: 0.20,
                    ..Bloom::NATURAL
                },
                Tonemapping::TonyMcMapface,
                // Ambiance violacée tiède, basse pour que le néon ressorte (mais moins froide).
                AmbientLight {
                    color: Color::srgb(0.55, 0.42, 0.60),
                    brightness: 110.0,
                    ..default()
                },
            ));
        });
}

/// Matériau de corps : base sombre + émissif (glow néon).
fn body_mat(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.02, 0.02, 0.03),
        emissive: LinearRgba::rgb(r, g, b),
        perceptual_roughness: 0.5,
        ..default()
    }
}

/// Déplacement horizontal avec ZQSD (positions physiques W/A/S/D = ZQSD en AZERTY).
pub fn move_player(
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

/// Vue à la souris : lacet sur le corps, tangage sur la tête (bloqué aux ~90°).
pub fn look_around(
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

/// Léger balancement vertical de la caméra quand on marche (donne de la vie).
pub fn head_bob(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut phase: Local<f32>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
) {
    let Ok(mut cam) = camera.single_mut() else {
        return;
    };

    let moving = keyboard.any_pressed([
        KeyCode::KeyW,
        KeyCode::KeyS,
        KeyCode::KeyA,
        KeyCode::KeyD,
    ]);

    if moving {
        *phase += time.delta_secs() * 9.0;
        cam.translation.y = EYE_HEIGHT + (*phase).sin() * 0.045;
    } else {
        // Retour doux à la hauteur de repos.
        *phase = 0.0;
        let t = (time.delta_secs() * 8.0).min(1.0);
        cam.translation.y += (EYE_HEIGHT - cam.translation.y) * t;
    }
}

/// Capture la souris au démarrage (curseur verrouillé et invisible).
pub fn grab_cursor(mut cursor: Query<&mut CursorOptions>) {
    if let Ok(mut cursor) = cursor.single_mut() {
        cursor.grab_mode = CursorGrabMode::Locked;
        cursor.visible = false;
    }
}

/// Échap libère la souris ; un clic gauche la recapture.
pub fn toggle_cursor(
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
