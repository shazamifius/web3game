//! Le petit JEU de l'île : des MÉTÉORITES de 4 RARETÉS tombent du ciel (de loin, avec
//! une traînée à LEUR couleur), atterrissent en cristaux, et on les RAMASSE avec E.
//! Un compteur PAR couleur s'affiche.
//!
//!   🟠 orange   = commun        (petite traînée)
//!   🟢 vert     = peu commun    (traînée moyenne)
//!   🟡 jaune    = rare          (grande traînée)
//!   ⚪ blanc    = extrêmement rare (traînée MAGISTRALE)
//!
//! Volontairement simple et sans dépendance : un xorshift maison pour le hasard, des
//! sphères émissives, et un ramassage par distance + touche E. Tout est tagué
//! `WorldGeometry` → ça se dé-spawn en quittant l'île. Ne suppose RIEN de l'île (vise
//! une zone autour de l'origine) → marchera tel quel avec une vraie île .glb.

use crate::player::Player;
use crate::scenes::{IslandTextured, WorldGeometry};
use bevy::picking::mesh_picking::ray_cast::{MeshRayCast, MeshRayCastSettings, RayCastVisibility};
use bevy::prelude::*;

const ISLAND_RADIUS: f32 = 95.0; // rayon de la zone d'impact (île géante ×30, demi-étendue ~138)
const FALL_SPEED: f32 = 11.0; // vitesse de chute (m/s) — lente : on les voit venir de loin
const GROUND_HIT: f32 = 0.30; // hauteur (y) à laquelle un météore « atterrit »
const PICKUP_RADIUS: f32 = 1.6; // distance (m) pour ramasser un cristal posé
const TRAIL_EVERY: f32 = 0.02; // période de dépôt d'un segment de traînée (s) — traînée fine
const TWINKLE_FREQ: f32 = 13.0; // vitesse du scintillement
const TWINKLE_AMP: f32 = 0.40; // amplitude du scintillement (±40 % de taille)

/// Les 4 raretés, du plus commun au plus rare.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Rarity {
    Orange,
    Green,
    Yellow,
    White,
}

impl Rarity {
    const ALL: [Rarity; 4] = [Rarity::Orange, Rarity::Green, Rarity::Yellow, Rarity::White];

    fn idx(self) -> usize {
        match self {
            Rarity::Orange => 0,
            Rarity::Green => 1,
            Rarity::Yellow => 2,
            Rarity::White => 3,
        }
    }

    /// Tirage pondéré : orange très commun → blanc extrêmement rare.
    fn roll(r: f32) -> Rarity {
        match r {
            x if x < 0.68 => Rarity::Orange,    // 68 %
            x if x < 0.92 => Rarity::Green,     // 24 %
            x if x < 0.985 => Rarity::Yellow,   // 6,5 %
            _ => Rarity::White,                 // 1,5 %
        }
    }

    /// Couleur ÉMISSIVE du cristal (sert au cristal, au météore ET à sa traînée). Très
    /// lumineuse → avec le bloom, les petits météores deviennent de vifs points scintillants.
    fn emissive(self) -> (f32, f32, f32) {
        match self {
            Rarity::Orange => (10.0, 3.0, 0.3),
            Rarity::Green => (0.6, 9.5, 1.4),
            Rarity::Yellow => (9.5, 7.8, 0.6),
            Rarity::White => (8.5, 8.8, 10.0),
        }
    }

    /// Taille de base d'un segment de traînée : la plus longue/large pour le blanc, dégressive.
    fn trail_size(self) -> f32 {
        match self {
            Rarity::Orange => 0.12,
            Rarity::Green => 0.16,
            Rarity::Yellow => 0.24,
            Rarity::White => 0.38,
        }
    }

    /// Taille du météore/cristal lui-même — PETIT (les rares à peine plus gros).
    fn body_size(self) -> f32 {
        match self {
            Rarity::Orange => 0.16,
            Rarity::Green => 0.20,
            Rarity::Yellow => 0.26,
            Rarity::White => 0.38,
        }
    }

    /// Nom + couleur (UI) du compteur.
    fn counter(self) -> (&'static str, Color) {
        match self {
            Rarity::Orange => ("Orange", Color::srgb(1.0, 0.55, 0.12)),
            Rarity::Green => ("Vert", Color::srgb(0.35, 1.0, 0.45)),
            Rarity::Yellow => ("Jaune", Color::srgb(1.0, 0.9, 0.25)),
            Rarity::White => ("Blanc", Color::srgb(0.95, 0.97, 1.0)),
        }
    }
}

/// Compteur PAR rareté. PERSISTANT : chargé au début de l'île, sauvé à chaque ramassage
/// (`~/.web3game/ile_score.txt`) → la collection survit aux changements de scène et de session.
#[derive(Resource, Default)]
pub struct Score(pub [u32; 4]);

/// Fichier de progression de l'île, à côté du coffre d'identité (`$WEB3GAME_DIR`, sinon
/// `~/.web3game` ; `%USERPROFILE%` sous Windows). Local par utilisateur, jamais réseau.
fn island_score_path() -> std::path::PathBuf {
    let dir = std::env::var_os("WEB3GAME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| std::path::PathBuf::from("."))
                .join(".web3game")
        });
    dir.join("ile_score.txt")
}

/// Charge la progression sauvegardée (4 compteurs), ou des zéros si rien/illisible.
fn load_island_score() -> [u32; 4] {
    let mut out = [0u32; 4];
    if let Ok(txt) = std::fs::read_to_string(island_score_path()) {
        for (slot, tok) in out.iter_mut().zip(txt.split_whitespace()) {
            if let Ok(v) = tok.parse() {
                *slot = v;
            }
        }
    }
    out
}

/// Sauvegarde la progression — best-effort : on ne casse JAMAIS le jeu si l'écriture échoue.
fn save_island_score(s: &[u32; 4]) {
    let path = island_score_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, format!("{} {} {} {}", s[0], s[1], s[2], s[3]));
}

/// Cadence d'apparition + graine de hasard maison (xorshift).
#[derive(Resource)]
pub struct MeteorClock {
    timer: f32,
    rng: u32,
}

/// Maillage + un matériau émissif par rareté (créés une fois à l'entrée sur l'île).
#[derive(Resource)]
pub struct MeteorAssets {
    ball: Handle<Mesh>,
    mats: [Handle<StandardMaterial>; 4], // indexés par Rarity::idx()
}

/// Un météore EN VOL.
#[derive(Component)]
pub struct Meteor {
    vel: Vec3,
    trail_acc: f32,
    rarity: Rarity,
}

/// Un cristal POSÉ au sol, prêt à être ramassé.
#[derive(Component)]
pub struct Collectible {
    rarity: Rarity,
}

/// Fait SCINTILLER une entité (météore en vol ou cristal posé) : sa taille pulse autour de
/// `base` à une phase propre (désynchronisée). Conservé à l'atterrissage → le cristal scintille.
#[derive(Component)]
pub struct Twinkle {
    base: f32,
    phase: f32,
}

/// Un segment de traînée qui rétrécit puis disparaît (`base` = sa taille de départ).
#[derive(Component)]
pub struct TrailPuff {
    life: f32,
    base: f32,
}

/// Marqueur d'un texte UI de compteur (porte sa rareté).
#[derive(Component)]
pub struct CounterText(Rarity);

/// Marqueur de l'invite « [E] Ramasser ».
#[derive(Component)]
pub struct PickupPrompt;

fn next_rand(state: &mut u32) -> f32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state as f32 / u32::MAX as f32 // [0,1)
}

/// OnEnter(Île) : (ré)initialise score, horloge, matériaux, compteurs et l'invite E.
pub fn setup_island_game(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Score(load_island_score())); // reprend la collection sauvegardée
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(1)
        | 1;
    commands.insert_resource(MeteorClock { timer: 0.5, rng: seed });
    let mats = Rarity::ALL.map(|r| {
        let (er, eg, eb) = r.emissive();
        materials.add(StandardMaterial {
            base_color: Color::BLACK,
            emissive: LinearRgba::rgb(er, eg, eb),
            ..default()
        })
    });
    commands.insert_resource(MeteorAssets { ball: meshes.add(Sphere::new(1.0)), mats });

    // Un compteur par rareté, empilés en haut à gauche.
    for r in Rarity::ALL {
        let (name, col) = r.counter();
        commands.spawn((
            WorldGeometry,
            CounterText(r),
            Text::new(format!("{name} : 0")),
            TextFont { font_size: 22.0, ..default() },
            TextColor(col),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(14.0),
                top: Val::Px(12.0 + r.idx() as f32 * 26.0),
                ..default()
            },
        ));
    }

    // L'invite de ramassage (cachée par défaut), en bas au centre.
    commands.spawn((
        WorldGeometry,
        PickupPrompt,
        Text::new("[ E ] Ramasser"),
        TextFont { font_size: 28.0, ..default() },
        TextColor(Color::srgb(1.0, 0.95, 0.6)),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(70.0),
            left: Val::Percent(42.0),
            ..default()
        },
        Visibility::Hidden,
    ));
}

/// Fait apparaître des météores SOUVENT (on en veut beaucoup), de rareté tirée au sort,
/// très haut et décalés, filant vers un point au hasard de l'île.
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
    clock.timer = 20.0 + next_rand(&mut clock.rng) * 100.0; // prochaine : 20–120 s (rare)

    let rarity = Rarity::roll(next_rand(&mut clock.rng));
    let ang = next_rand(&mut clock.rng) * std::f32::consts::TAU;
    let rad = next_rand(&mut clock.rng).sqrt() * ISLAND_RADIUS;
    let target = Vec3::new(ang.cos() * rad, GROUND_HIT, ang.sin() * rad);
    // Départ ULTRA HAUT (320–390 m) et quasi vertical → ~30 s de chute, visible de très loin.
    let off = Vec3::new(
        (next_rand(&mut clock.rng) - 0.5) * 30.0,
        320.0 + next_rand(&mut clock.rng) * 70.0,
        (next_rand(&mut clock.rng) - 0.5) * 30.0,
    );
    let start = target + off;
    let vel = (target - start).normalize() * FALL_SPEED;
    let phase = next_rand(&mut clock.rng) * std::f32::consts::TAU;

    commands.spawn((
        WorldGeometry,
        Meteor { vel, trail_acc: 0.0, rarity },
        Twinkle { base: rarity.body_size(), phase },
        Mesh3d(assets.ball.clone()),
        MeshMaterial3d(assets.mats[rarity.idx()].clone()),
        Transform::from_translation(start).with_scale(Vec3::splat(rarity.body_size())),
    ));
}

/// Déplace les météores, sème leur traînée (à LEUR couleur, taille selon la rareté), et
/// les transforme en cristal À RAMASSER au contact du sol.
pub fn fall_meteors(
    time: Res<Time>,
    assets: Res<MeteorAssets>,
    mut ray: MeshRayCast,
    terrain: Query<(), With<IslandTextured>>,
    mut commands: Commands,
    mut meteors: Query<(Entity, &mut Transform, &mut Meteor)>,
) {
    let dt = time.delta_secs();
    let filter = |e: Entity| terrain.contains(e);
    let settings = MeshRayCastSettings::default()
        .with_filter(&filter)
        .with_visibility(RayCastVisibility::Any);
    for (e, mut tf, mut m) in &mut meteors {
        tf.translation += m.vel * dt;

        m.trail_acc += dt;
        let base = m.rarity.trail_size();
        while m.trail_acc >= TRAIL_EVERY {
            m.trail_acc -= TRAIL_EVERY;
            commands.spawn((
                WorldGeometry,
                TrailPuff { life: 1.1, base },
                Mesh3d(assets.ball.clone()),
                MeshMaterial3d(assets.mats[m.rarity.idx()].clone()),
                Transform::from_translation(tf.translation).with_scale(Vec3::splat(base)),
            ));
        }

        // Sol RÉEL sous le météore (raycast sur le terrain), pour qu'il se pose SUR le relief
        // (montagnes comprises) au lieu de plonger dans un plan plat. On lance depuis un peu
        // au-dessus pour ne pas rater le sol si on l'a déjà franchi cette frame.
        let origin = tf.translation + Vec3::Y * 2.0;
        let ground = ray
            .cast_ray(Ray3d::new(origin, Dir3::NEG_Y), &settings)
            .first()
            .map(|(_, hit)| hit.point.y);

        match ground {
            // Touché le terrain : on pose le cristal POSÉ dessus (centre = sol + demi-taille).
            Some(g) if tf.translation.y <= g + m.rarity.body_size() => {
                tf.translation.y = g + m.rarity.body_size() * 0.5;
                // On garde `Twinkle` → le cristal posé continue de scintiller (il gère la taille).
                commands.entity(e).remove::<Meteor>().insert(Collectible { rarity: m.rarity });
            }
            // Pas de terrain sous nous (tombé hors de l'île / au-dessus de l'eau) : on laisse
            // filer, puis on nettoie sous le niveau de référence pour ne pas accumuler.
            None if tf.translation.y < GROUND_HIT - 5.0 => {
                commands.entity(e).despawn();
            }
            _ => {}
        }
    }
}

/// Fait SCINTILLER météores en vol et cristaux posés : la taille pulse autour de `base`.
pub fn twinkle(time: Res<Time>, mut q: Query<(&Twinkle, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (tw, mut tf) in &mut q {
        let f = 1.0 + TWINKLE_AMP * (t * TWINKLE_FREQ + tw.phase).sin();
        tf.scale = Vec3::splat(tw.base * f);
    }
}

/// Rétrécit les segments de traînée puis les supprime → un sillon coloré qui s'efface.
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
            tf.scale = Vec3::splat(p.base * p.life);
        }
    }
}

/// Invite : montre « [E] Ramasser » si un cristal est à portée (et garde le plus proche).
/// Renvoie l'entité la plus proche dans le rayon, le cas échéant.
fn nearest_collectible(
    player: &Transform,
    gems: &Query<(Entity, &Transform, &Collectible)>,
) -> Option<(Entity, Rarity)> {
    let mut best: Option<(Entity, Rarity, f32)> = None;
    for (e, gt, c) in gems.iter() {
        let d = player.translation.xz().distance(gt.translation.xz());
        if d < PICKUP_RADIUS && best.map_or(true, |(_, _, bd)| d < bd) {
            best = Some((e, c.rarity, d));
        }
    }
    best.map(|(e, r, _)| (e, r))
}

/// Affiche/masque l'invite « [E] Ramasser » selon la proximité d'un cristal.
pub fn pickup_prompt(
    player: Query<&Transform, With<Player>>,
    gems: Query<(Entity, &Transform, &Collectible)>,
    mut prompt: Query<&mut Visibility, With<PickupPrompt>>,
) {
    let (Ok(p), Ok(mut vis)) = (player.single(), prompt.single_mut()) else {
        return;
    };
    *vis = if nearest_collectible(p, &gems).is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
}

/// Ramassage : la touche E prend le cristal le PLUS PROCHE à portée (+1 à sa couleur).
pub fn collect_meteors(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
    mut score: ResMut<Score>,
    player: Query<&Transform, With<Player>>,
    gems: Query<(Entity, &Transform, &Collectible)>,
) {
    if !keyboard.just_pressed(KeyCode::KeyE) {
        return;
    }
    let Ok(p) = player.single() else {
        return;
    };
    if let Some((e, rarity)) = nearest_collectible(p, &gems) {
        commands.entity(e).despawn();
        score.0[rarity.idx()] += 1;
        save_island_score(&score.0); // persiste la collection à chaque ramassage
    }
}

/// Met à jour les 4 compteurs colorés.
pub fn update_counters(score: Res<Score>, mut texts: Query<(&CounterText, &mut Text)>) {
    if !score.is_changed() {
        return;
    }
    for (c, mut t) in &mut texts {
        let (name, _) = c.0.counter();
        t.0 = format!("{name} : {}", score.0[c.0.idx()]);
    }
}
