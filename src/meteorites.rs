//! Le petit JEU de l'île : des MÉTÉORITES tombent du ciel (de loin, avec une traînée),
//! atterrissent sur l'île, et on va les RAMASSER. Un compteur affiche le score.
//!
//! Volontairement simple et sans dépendance : un xorshift maison pour le hasard, des
//! sphères émissives pour les météores/traînées, et une détection de ramassage par
//! distance. Tout est tagué `WorldGeometry` → ça se dé-spawn en quittant l'île.
//!
//! Ce module ne suppose RIEN de l'île (il vise une zone autour de l'origine) → il
//! marchera tel quel quand l'île placeholder sera remplacée par un vrai modèle .glb.

use crate::player::Player;
use crate::scenes::WorldGeometry;
use bevy::prelude::*;

const ISLAND_RADIUS: f32 = 8.0; // rayon de la zone où les météorites tombent
const FALL_SPEED: f32 = 22.0; // vitesse de chute (m/s)
const GROUND_HIT: f32 = 0.35; // hauteur (y) à laquelle un météore « atterrit »
const PICKUP_RADIUS: f32 = 1.4; // distance (m) pour ramasser un météore posé
const TRAIL_EVERY: f32 = 0.03; // période de dépôt d'un segment de traînée (s)

/// Compteur de météorites ramassées (remis à 0 à chaque entrée sur l'île).
#[derive(Resource, Default)]
pub struct Score(pub u32);

/// Cadence d'apparition + graine de hasard maison (xorshift).
#[derive(Resource)]
pub struct MeteorClock {
    timer: f32,
    rng: u32,
}

/// Maillages/matériaux partagés des météores (créés une fois à l'entrée sur l'île).
#[derive(Resource)]
pub struct MeteorAssets {
    ball: Handle<Mesh>,
    hot: Handle<StandardMaterial>,   // météore en vol (orange ardent)
    trail: Handle<StandardMaterial>, // segment de traînée
    gem: Handle<StandardMaterial>,   // météore posé, à ramasser (cyan-violet)
}

/// Un météore EN VOL (vitesse + accumulateur pour semer la traînée).
#[derive(Component)]
pub struct Meteor {
    vel: Vec3,
    trail_acc: f32,
}

/// Un météore POSÉ au sol, prêt à être ramassé.
#[derive(Component)]
pub struct Collectible;

/// Un segment de traînée qui rétrécit puis disparaît.
#[derive(Component)]
pub struct TrailPuff {
    life: f32,
}

/// Marqueur du texte UI du compteur.
#[derive(Component)]
pub struct ScoreText;

fn next_rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32 // [0,1)
}

/// OnEnter(Île) : (ré)initialise le score, l'horloge, les matériaux, et le compteur UI.
pub fn setup_island_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Score(0));
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1)
        | 1;
    commands.insert_resource(MeteorClock { timer: 0.5, rng: seed });
    commands.insert_resource(MeteorAssets {
        ball: meshes.add(Sphere::new(1.0)),
        hot: materials.add(emissive(7.0, 2.2, 0.3)),
        trail: materials.add(emissive(5.0, 1.6, 0.25)),
        gem: materials.add(emissive(1.2, 0.5, 4.5)),
    });

    // Compteur, en haut à gauche.
    commands.spawn((
        WorldGeometry,
        ScoreText,
        Text::new("Météorites : 0"),
        TextFont { font_size: 24.0, ..default() },
        TextColor(Color::srgb(1.0, 0.85, 0.4)),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(14.0),
            top: Val::Px(12.0),
            ..default()
        },
    ));
}

/// Fait apparaître des météores RÉGULIÈREMENT (souvent : on en veut beaucoup), chacun
/// très haut et décalé, filant vers un point au hasard de l'île.
pub fn spawn_meteors(
    time: Res<Time>,
    mut clock: ResMut<MeteorClock>,
    assets: Res<MeteorAssets>,
    mut commands: Commands,
) {
    clock.timer -= time.delta_secs();
    if clock.timer > 0.0 {
        return;
    }
    // Prochaine apparition dans 0,5 à 1,5 s → un ciel bien actif.
    clock.timer = 0.5 + next_rand(&mut clock.rng) * 1.0;

    // Cible au sol (dans le disque de l'île) et point de départ haut + décalé.
    let ang = next_rand(&mut clock.rng) * std::f32::consts::TAU;
    let rad = next_rand(&mut clock.rng).sqrt() * ISLAND_RADIUS;
    let target = Vec3::new(ang.cos() * rad, GROUND_HIT, ang.sin() * rad);
    let off = Vec3::new(
        (next_rand(&mut clock.rng) - 0.5) * 50.0,
        55.0 + next_rand(&mut clock.rng) * 15.0,
        (next_rand(&mut clock.rng) - 0.5) * 50.0,
    );
    let start = target + off;
    let vel = (target - start).normalize() * FALL_SPEED;

    commands.spawn((
        WorldGeometry,
        Meteor { vel, trail_acc: 0.0 },
        Mesh3d(assets.ball.clone()),
        MeshMaterial3d(assets.hot.clone()),
        Transform::from_translation(start).with_scale(Vec3::splat(0.5)),
    ));
}

/// Déplace les météores, sème leur traînée, et les transforme en objet À RAMASSER
/// quand ils touchent le sol.
pub fn fall_meteors(
    time: Res<Time>,
    assets: Res<MeteorAssets>,
    mut commands: Commands,
    mut meteors: Query<(Entity, &mut Transform, &mut Meteor)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut m) in &mut meteors {
        tf.translation += m.vel * dt;

        // Semer des segments de traînée derrière le météore.
        m.trail_acc += dt;
        while m.trail_acc >= TRAIL_EVERY {
            m.trail_acc -= TRAIL_EVERY;
            commands.spawn((
                WorldGeometry,
                TrailPuff { life: 1.1 },
                Mesh3d(assets.ball.clone()),
                MeshMaterial3d(assets.trail.clone()),
                Transform::from_translation(tf.translation).with_scale(Vec3::splat(0.34)),
            ));
        }

        // Atterrissage : il se fige au sol et devient ramassable (gemme cyan-violet).
        if tf.translation.y <= GROUND_HIT {
            tf.translation.y = GROUND_HIT;
            tf.scale = Vec3::splat(0.45);
            commands
                .entity(e)
                .remove::<Meteor>()
                .insert(Collectible)
                .insert(MeshMaterial3d(assets.gem.clone()));
        }
    }
}

/// Rétrécit les segments de traînée puis les supprime → un joli sillon qui s'efface.
pub fn fade_trails(
    time: Res<Time>,
    mut commands: Commands,
    mut puffs: Query<(Entity, &mut Transform, &mut TrailPuff)>,
) {
    let dt = time.delta_secs();
    for (e, mut tf, mut p) in &mut puffs {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
        } else {
            tf.scale = Vec3::splat(0.34 * p.life);
        }
    }
}

/// Ramassage : si le joueur passe assez près d'un météore posé, il le prend (+1).
pub fn collect_meteors(
    mut commands: Commands,
    mut score: ResMut<Score>,
    player: Query<&Transform, With<Player>>,
    gems: Query<(Entity, &Transform), With<Collectible>>,
) {
    let Ok(p) = player.single() else {
        return;
    };
    for (e, gt) in &gems {
        if p.translation.xz().distance(gt.translation.xz()) < PICKUP_RADIUS {
            commands.entity(e).despawn();
            score.0 += 1;
        }
    }
}

/// Met à jour le texte du compteur.
pub fn update_score_text(score: Res<Score>, mut text: Query<&mut Text, With<ScoreText>>) {
    if !score.is_changed() {
        return;
    }
    if let Ok(mut t) = text.single_mut() {
        t.0 = format!("Météorites : {}", score.0);
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
