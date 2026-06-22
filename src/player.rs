//! Le joueur : un corps articulé simple (torse, tête, bras, jambes), une caméra
//! à hauteur des yeux, le déplacement ZQSD, la vue à la souris et un léger head-bob.

use crate::world::ROOM_SIZE;
use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::input::mouse::AccumulatedMouseMotion;
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy::render::view::Hdr;
use bevy::window::{CursorGrabMode, CursorOptions};
use std::f32::consts::FRAC_PI_2;

const MOVE_SPEED: f32 = 4.0; // mètres par seconde
const MOUSE_SENSITIVITY: f32 = 0.0015;
const BODY_RADIUS: f32 = 0.25; // demi-largeur du corps (pour rester dans la salle)
const EYE_HEIGHT: f32 = 0.7; // hauteur des yeux au-dessus du centre du corps (~niveau de la tête)

// --- Saut & gravité (petit jeu île) ---
/// Hauteur du CENTRE du corps quand les pieds touchent le sol (`pub` : l'aiguillage
/// de scènes repositionne le joueur à cette hauteur en changeant de monde).
pub const GROUND_Y: f32 = 0.7;
const GRAVITY: f32 = 18.0; // m/s² (un peu plus que la vraie : plus « jeu », moins flottant)
const JUMP_SPEED: f32 = 6.0; // impulsion verticale au décollage (m/s) → saut ~1 m de haut

/// Joueur : porte la position et la rotation gauche/droite (lacet).
#[derive(Component)]
pub struct Player;

/// Vitesse VERTICALE du joueur (m/s) — saut + chute. À 0 quand on est au sol.
/// Séparée du déplacement horizontal (ZQSD), qui reste « à plat ». `pub(crate)` pour
/// que l'aiguillage de scènes puisse la remettre à 0 en téléportant le joueur.
#[derive(Component)]
pub struct Vertical {
    pub(crate) vy: f32,
}

/// Tête/caméra : porte la rotation haut/bas (tangage) et le head-bob.
#[derive(Component)]
pub struct PlayerCamera;

/// Crée le joueur (corps articulé) et sa caméra première personne.
/// Position de départ pseudo-aléatoire (x, z) dans la salle (chap. 8.2c). Même générateur
/// maison que la couleur de skin (xorshift, graine = nanosecondes ^ identifiant de processus
/// → deux fenêtres lancées quasi en même temps tombent à des endroits différents). Aucune
/// dépendance externe. On garde une marge d'1 m depuis les murs.
fn random_spawn_xz() -> (f32, f32) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut s = (nanos ^ std::process::id().wrapping_mul(2_654_435_761)) | 1;
    let mut next = || {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        (s as f32 / u32::MAX as f32) * 2.0 - 1.0 // ramené dans [-1, 1)
    };
    let half = crate::world::ROOM_SIZE / 2.0 - 1.0; // marge depuis les murs
    (next() * half, next() * half)
}

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

    // Point de départ. Au HUB (et sur l'île), spawn FIXE et déterministe — sinon on
    // apparaît parfois DANS un portail (qui nous téléporte aussitôt). On n'éparpille (8.2c)
    // que si on démarre DIRECTEMENT en arcade (`SCENE=arcade`, cas `tools/foule-3d.sh`) :
    // là, éparpiller distingue le focus (proches détaillés) de la conscience (lointains LOD).
    let (sx, sz) = match crate::scenes::initial_scene() {
        crate::scenes::Scene::Arcade => random_spawn_xz(),
        _ => (0.0, 2.5), // face aux 2 portails du hub, à bonne distance
    };

    commands
        .spawn((
            Player,
            Vertical { vy: 0.0 }, // au sol au départ
            // Centre du corps à 0,7 m : les pieds arrivent ~au sol.
            Transform::from_xyz(sx, GROUND_Y, sz),
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
                // Far plane large : l'île géante + son ciel (lune/étoiles lointaines) doivent
                // tenir dans le frustum sans être coupés.
                Projection::from(PerspectiveProjection {
                    far: 4000.0,
                    ..default()
                }),
                Transform::from_xyz(0.0, EYE_HEIGHT, 0.0),
                // HDR + bloom : le néon « glow ». Tonemapping pour de belles couleurs.
                Hdr,
                // Bloom À SEUIL (OLD_SCHOOL = additif + prefilter) : SEULS les pixels au-dessus
                // du seuil (le néon émissif ≫ 1) « bavent », et fort. Le terrain/les murs sombres
                // restent NETS → plus de voile gris sur l'île, et un glow néon franc comme Blender.
                Bloom {
                    intensity: 0.30,
                    ..Bloom::OLD_SCHOOL
                },
                Tonemapping::TonyMcMapface,
                // Ambiance basse et violacée : surfaces SOMBRES → le néon émissif RESSORT.
                AmbientLight {
                    color: Color::srgb(0.45, 0.35, 0.55),
                    brightness: 45.0,
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
    scene: Res<State<crate::scenes::Scene>>,
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

    // Bornage simple UNIQUEMENT dans la salle arcade (boîte fermée). Le hub (glb) et l'île
    // sont ouverts et plus grands → pas de mur invisible (sinon on ne pourrait PAS atteindre
    // les portails du hub, à z≈16). La vraie collision (raycast sur mesh) viendra avec l'île .glb.
    if *scene.get() == crate::scenes::Scene::Arcade {
        let limit = ROOM_SIZE / 2.0 - BODY_RADIUS;
        transform.translation.x = transform.translation.x.clamp(-limit, limit);
        transform.translation.z = transform.translation.z.clamp(-limit, limit);
    }
}

/// Saut + gravité (Espace pour sauter). On ne peut sauter QUE si on touche le sol
/// (pas de double-saut). Hors saut, on reste collé au sol → comportement inchangé.
pub fn jump_and_gravity(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    let Ok((mut transform, mut v)) = player.single_mut() else {
        return;
    };

    let grounded = transform.translation.y <= GROUND_Y + 0.001 && v.vy <= 0.0;

    // Décollage : seulement si on est au sol (anti double-saut).
    if grounded && keyboard.just_pressed(KeyCode::Space) {
        v.vy = JUMP_SPEED;
    }

    // Intégration verticale : la gravité tire vers le bas en continu.
    let dt = time.delta_secs();
    v.vy -= GRAVITY * dt;
    transform.translation.y += v.vy * dt;

    // Atterrissage : on ne passe pas sous le sol, et la vitesse retombe à 0.
    if transform.translation.y <= GROUND_Y {
        transform.translation.y = GROUND_Y;
        v.vy = 0.0;
    }
}

/// COLLISION de l'île (raycast sur le terrain) : le joueur MARCHE sur le relief, et s'il
/// QUITTE l'île (chute hors du terrain / sous l'eau), il revient INSTANTANÉMENT au spawn.
/// On lance un rayon vers le BAS sous le joueur, filtré sur le seul terrain (`Terrain`) :
///   - touche → sa hauteur devient le sol (gravité + saut par-dessus, comme ailleurs) ;
///   - rien sous nous OU on est passé sous le niveau de l'eau → retour au spawn.
/// Tant que le terrain n'est pas chargé, on tient le joueur immobile (pas de chute parasite).
pub fn island_collision(
    mut ray: MeshRayCast,
    terrain: Query<(), With<crate::scenes::IslandTextured>>,
    spawn: Res<crate::scenes::IslandSpawn>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut player: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    let Ok((mut t, mut v)) = player.single_mut() else {
        return;
    };
    if terrain.is_empty() {
        v.vy = 0.0; // terrain pas encore chargé → on ne chute pas
        return;
    }
    // Hors carte / sous l'eau → retour spawn instantané.
    if t.translation.y < spawn.water_y {
        t.translation = spawn.pos;
        t.rotation = Quat::from_rotation_y(std::f32::consts::PI);
        v.vy = 0.0;
        return;
    }
    // Sol = terrain sous le joueur (rayon vers le bas, depuis un peu au-dessus).
    let origin = t.translation + Vec3::Y * 4.0;
    let filter = |e: Entity| terrain.contains(e);
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_visibility(RayCastVisibility::Any);
    let ground = ray
        .cast_ray(Ray3d::new(origin, Dir3::NEG_Y), &settings)
        .first()
        .map(|(_, hit)| hit.point.y + GROUND_Y); // centre du corps au-dessus du sol

    let dt = time.delta_secs();
    v.vy -= GRAVITY * dt;
    match ground {
        Some(g) => {
            let grounded = t.translation.y <= g + 0.02 && v.vy <= 0.0;
            if grounded && keyboard.just_pressed(KeyCode::Space) {
                v.vy = JUMP_SPEED;
            }
            t.translation.y += v.vy * dt;
            if t.translation.y <= g {
                t.translation.y = g;
                v.vy = 0.0;
            }
        }
        None => {
            // Pas de terrain sous nous (au-dessus de l'eau / dans le vide) → on chute,
            // puis le test « sous l'eau » ci-dessus renverra au spawn.
            t.translation.y += v.vy * dt;
        }
    }
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
