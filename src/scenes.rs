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

/// A-t-on déjà posé le joueur au marqueur `spawn` du glb POUR CETTE entrée dans le hub ?
/// Remis à `false` à chaque `OnEnter(Hub)` (la scène glb se charge sur quelques frames).
#[derive(Resource, Default)]
pub struct HubSpawnDone(pub bool);

/// Marqueur posé sur le terrain de l'île une fois qu'il a été coloré procéduralement
/// (sert AUSSI de cible au raycast de collision : on ne marche QUE sur ce terrain).
#[derive(Component)]
pub struct IslandTextured;

/// Point d'apparition de l'île (lu du marqueur `spawn` du glb) + niveau de « chute » sous
/// lequel on renvoie le joueur au spawn (sortie de l'île / eau). `done` : déjà placé ?
#[derive(Resource)]
pub struct IslandSpawn {
    pub pos: Vec3,
    pub water_y: f32,
    pub done: bool,
}

impl Default for IslandSpawn {
    fn default() -> Self {
        // Au-dessus de l'île tant que le marqueur n'est pas lu ; `water_y = 0` = sous le
        // plus bas terrain marchable (centre du corps ≥ 0,7 dessus) → pas de faux renvoi.
        Self { pos: Vec3::new(0.0, 30.0, 0.0), water_y: 0.0, done: false }
    }
}

/// Un portail du hub : marcher dedans bascule vers `target`.
#[derive(Component)]
pub struct Portal {
    target: Scene,
}

/// Étiquette UI (texte) d'un portail : suit à l'écran la position 3D de l'entité-portail
/// `portal` (même technique que les pseudos des avatars).
#[derive(Component)]
pub struct PortalLabel {
    portal: Entity,
}

const PORTAL_RADIUS: f32 = 1.6; // rayon (m) pour « entrer » dans un portail

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
/// Monte le hub depuis le MODÈLE Blender `asset/HUB.glb` : on instancie la scène glTF
/// (la plateforme, le pont, les portails, le panneau…) + une lumière. Les portails et le
/// point d'apparition sont LIÉS ensuite par leur NOM (`bind_gltf_hub`).
pub fn setup_hub(mut commands: Commands, assets: Res<AssetServer>) {
    // `HUB.glb#Scene0` : le dossier d'assets est configuré sur `asset/` (cf. main.rs).
    commands.spawn((
        WorldGeometry,
        SceneRoot(assets.load("HUB.glb#Scene0")),
        Transform::IDENTITY,
    ));
    // Lumière VOLONTAIREMENT BASSE : surfaces sombres → les néons émissifs (strength 4)
    // RESSORTENT (look synthwave). Juste de quoi ne pas être dans le noir total.
    commands.spawn((
        WorldGeometry,
        PointLight { color: Color::srgb(0.7, 0.75, 0.95), intensity: 220_000.0, range: 50.0, shadows_enabled: false, ..default() },
        Transform::from_xyz(0.0, 6.0, 8.0),
    ));
}

/// LIAISON par NOM des objets du glb (la scène glTF se peuple sur quelques frames, donc on
/// réessaie chaque frame tant que ce n'est pas fait) :
///   - `portal arcade` / `portal ile`  → on y greffe un `Portal` (+ une étiquette UI).
///   - `spawn` (l'Empty)                → on y TÉLÉPORTE le joueur (une fois), face aux portails.
/// Tolérant : on compare en minuscules par `contains` (« portal arcade », « Portal_Arcade »…).
pub fn bind_gltf_hub(
    mut commands: Commands,
    mut spawn_done: ResMut<HubSpawnDone>,
    // `Without<Portal>` : on ne (re)lie que les portails PAS ENCORE liés → marche aussi au
    // RETOUR au hub (la scène glb est ré-instanciée avec de NOUVELLES entités).
    named: Query<(Entity, &Name, &Transform), Without<Portal>>,
    mut player: Query<(&mut Transform, &mut Vertical), (With<Player>, Without<Name>)>,
) {
    for (e, name, tf) in &named {
        let n = name.as_str().to_lowercase();
        if n.contains("portal") && n.contains("arcade") {
            commands.entity(e).insert(Portal { target: Scene::Arcade });
            spawn_portal_label(&mut commands, e, "ARCADE");
        } else if n.contains("portal") && (n.contains("ile") || n.contains("island")) {
            commands.entity(e).insert(Portal { target: Scene::Island });
            spawn_portal_label(&mut commands, e, "ÎLE");
        } else if !spawn_done.0 && n == "spawn" {
            // L'Empty `SPAWN` est un nœud de 1er niveau → son `Transform` local = sa position.
            // On garde X/Z, on pose le joueur au sol (`GROUND_Y`), face aux portails (+Z).
            if let Ok((mut pt, mut v)) = player.single_mut() {
                pt.translation = Vec3::new(tf.translation.x, GROUND_Y, tf.translation.z);
                pt.rotation = Quat::from_rotation_y(std::f32::consts::PI); // regarde vers +Z
                v.vy = 0.0;
                spawn_done.0 = true;
            }
        }
    }
}

/// Crée l'étiquette UI (texte) d'un portail, liée à son entité (suit sa position 3D).
fn spawn_portal_label(commands: &mut Commands, portal: Entity, name: &str) {
    commands.spawn((
        WorldGeometry,
        PortalLabel { portal },
        Text::new(name),
        TextFont { font_size: 26.0, ..default() },
        TextColor(Color::WHITE),
        Node { position_type: PositionType::Absolute, ..default() },
    ));
}

/// Projette chaque étiquette de portail (au-dessus de l'entité liée) sur l'écran.
pub fn update_portal_labels(
    camera: Query<(&Camera, &GlobalTransform)>,
    portals: Query<&GlobalTransform, With<Portal>>,
    mut labels: Query<(&PortalLabel, &mut Node, &mut Visibility)>,
) {
    let Ok((cam, cam_tf)) = camera.single() else {
        return;
    };
    for (label, mut node, mut vis) in &mut labels {
        let world = match portals.get(label.portal) {
            Ok(gt) => gt.translation() + Vec3::Y * 1.3,
            Err(_) => {
                *vis = Visibility::Hidden;
                continue;
            }
        };
        match cam.world_to_viewport(cam_tf, world) {
            Ok(screen) => {
                *vis = Visibility::Visible;
                node.left = Val::Px(screen.x);
                node.top = Val::Px(screen.y);
            }
            Err(_) => *vis = Visibility::Hidden,
        }
    }
}

/// Dans le hub uniquement : si le joueur s'approche assez d'un portail (en X/Z), on bascule.
pub fn portal_enter(
    player: Query<&Transform, With<Player>>,
    portals: Query<(&GlobalTransform, &Portal)>,
    mut next: ResMut<NextState<Scene>>,
) {
    let Ok(p) = player.single() else {
        return;
    };
    for (pt, portal) in &portals {
        if p.translation.xz().distance(pt.translation().xz()) < PORTAL_RADIUS {
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
/// Facteur d'agrandissement de l'île .glb (le mesh exporté est petit).
pub const ISLAND_SCALE: f32 = 12.0;

pub fn setup_island(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
) {
    let ball = meshes.add(Sphere::new(1.0));

    // --- L'ÎLE : le modèle Blender `asset/ile.glb`, agrandi. Il n'a NI matériau NI texture
    // → `texture_island` (système) le colore PROCÉDURALEMENT par hauteur + pente. ---
    commands.spawn((
        WorldGeometry,
        SceneRoot(assets.load("ile.glb#Scene0")),
        Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(ISLAND_SCALE)),
    ));

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

/// TEXTURING PROCÉDURAL du terrain de l'île (le .glb n'a aucune texture). Quand le gros
/// mesh « Landscape » est chargé, on calcule UNE couleur par sommet selon sa HAUTEUR et sa
/// PENTE (sable en bas, herbe, roche sur les pentes raides, neige sur les sommets), on
/// l'écrit en couleurs de sommets, et on pose un matériau blanc qui les laisse ressortir.
/// → un terrain « clean » sans aucun travail de texture côté Blender. Fait UNE fois (marqueur).
pub fn texture_island(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q: Query<(Entity, &Mesh3d), Without<IslandTextured>>,
) {
    use bevy::mesh::VertexAttributeValues as V;
    for (e, m3d) in &q {
        let Some(mesh) = meshes.get_mut(&m3d.0) else {
            continue; // pas encore chargé
        };
        // On ne vise QUE le terrain (très dense) — pas les météores/étoiles (petits meshes).
        if mesh.count_vertices() < 50_000 {
            continue;
        }
        if mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_none() {
            let positions: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                Some(V::Float32x3(p)) => p.clone(),
                _ => continue,
            };
            let normals: Vec<[f32; 3]> = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
                Some(V::Float32x3(n)) => n.clone(),
                _ => vec![[0.0, 1.0, 0.0]; positions.len()],
            };
            let (mut ymin, mut ymax) = (f32::MAX, f32::MIN);
            for p in &positions {
                ymin = ymin.min(p[1]);
                ymax = ymax.max(p[1]);
            }
            let span = (ymax - ymin).max(1e-4);
            let colors: Vec<[f32; 4]> = positions
                .iter()
                .zip(&normals)
                .map(|(p, n)| terrain_color((p[1] - ymin) / span, n[1].abs()))
                .collect();
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
        }
        // Matériau blanc mat → les couleurs de sommets portent toute la teinte du terrain.
        let mat = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.95,
            ..default()
        });
        commands.entity(e).insert((MeshMaterial3d(mat), IslandTextured));
    }
}

/// Couleur d'un sommet de terrain selon sa hauteur `t` (0 = bas, 1 = sommet) et la planéité
/// `flat` (1 = horizontal, 0 = vertical/falaise). Bandes douces : sable → herbe → roche → neige,
/// et les pentes raides virent à la roche (falaises) quelle que soit la hauteur.
fn terrain_color(t: f32, flat: f32) -> [f32; 4] {
    let grass = [0.16, 0.42, 0.15];
    let rock = [0.34, 0.31, 0.28];
    let snow = [0.92, 0.94, 0.98];
    let sand = [0.78, 0.71, 0.46];
    let lerp = |a: [f32; 3], b: [f32; 3], k: f32| {
        let k = k.clamp(0.0, 1.0);
        [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k]
    };
    let base = if t < 0.06 {
        sand
    } else if t > 0.72 {
        snow
    } else if t > 0.5 {
        lerp(grass, rock, (t - 0.5) / 0.22)
    } else {
        grass
    };
    // Pentes raides → roche (falaises) : d'autant plus que c'est vertical.
    let cliff = ((1.0 - flat) * 1.6).clamp(0.0, 0.85);
    let c = lerp(base, rock, cliff);
    [c[0], c[1], c[2], 1.0]
}

/// LIAISON du marqueur `spawn` de l'île (Empty du glb, nœud de 1er niveau → `Transform`
/// local × `ISLAND_SCALE` = position monde). On y pose le joueur UNE fois (à `OnEnter`,
/// `done` est remis à `false`). Sert aussi de point de renvoi quand on tombe de l'île.
pub fn bind_island_spawn(
    mut spawn: ResMut<IslandSpawn>,
    named: Query<(&Name, &Transform)>,
    mut player: Query<(&mut Transform, &mut Vertical), (With<Player>, Without<Name>)>,
) {
    if spawn.done {
        return;
    }
    for (name, tf) in &named {
        if name.as_str().eq_ignore_ascii_case("spawn") {
            // Centre du corps légèrement au-dessus du marqueur (la collision le posera au sol).
            spawn.pos = tf.translation * ISLAND_SCALE + Vec3::Y * GROUND_Y;
            if let Ok((mut pt, mut v)) = player.single_mut() {
                pt.translation = spawn.pos;
                pt.rotation = Quat::from_rotation_y(std::f32::consts::PI);
                v.vy = 0.0;
            }
            spawn.done = true;
            return;
        }
    }
}

// ----------------------------------------------------------------------------
// Téléportation du joueur au point d'apparition de chaque scène
// ----------------------------------------------------------------------------
pub fn enter_hub_player(
    mut clear: ResMut<ClearColor>,
    mut spawn_done: ResMut<HubSpawnDone>,
    q: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    clear.0 = SKY_DARK;
    spawn_done.0 = false; // le marqueur `spawn` du glb (re)placera le joueur
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
    mut spawn: ResMut<IslandSpawn>,
    mut q: Query<(&mut Transform, &mut Vertical), With<Player>>,
) {
    clear.0 = SKY_NIGHT;
    spawn.done = false; // le marqueur `spawn` du glb (re)placera le joueur
    // Placement temporaire EN HAUTEUR le temps que le terrain charge (la collision tient le
    // joueur immobile tant que le terrain n'est pas là, puis `bind_island_spawn` le pose).
    if let Ok((mut t, mut v)) = q.single_mut() {
        t.translation = Vec3::new(0.0, 30.0, 0.0);
        t.rotation = Quat::from_rotation_y(std::f32::consts::PI);
        v.vy = 0.0;
    }
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
