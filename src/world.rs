//! La salle : sol, murs, plafond, grille néon au sol, arêtes lumineuses,
//! plafonnier visible et éclairage. Ambiance « arcade synthwave ».

use crate::scenes::WorldGeometry;
use bevy::prelude::*;

// Dimensions de la salle (en mètres), partagées avec le module joueur.
pub const ROOM_SIZE: f32 = 12.0; // largeur et profondeur
pub const ROOM_HEIGHT: f32 = 4.0; // hauteur sol -> plafond

/// Construit toute la salle au démarrage.
pub fn setup_room(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let half = ROOM_SIZE / 2.0;

    // Un seul cube unitaire, mis à l'échelle pour chaque élément (sol, mur, barre…).
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    // --- Matériaux des surfaces (volontairement sombres : le néon ressort mieux) ---
    let floor_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.03, 0.03, 0.06),
        perceptual_roughness: 0.85,
        ..default()
    });
    let wall_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.03, 0.09),
        emissive: LinearRgba::rgb(0.02, 0.0, 0.04), // halo très discret : surface sombre, comme le sol
        perceptual_roughness: 0.9,
        ..default()
    });
    let ceil_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.04, 0.05, 0.09),
        emissive: LinearRgba::rgb(0.0, 0.03, 0.06), // léger halo cyan
        perceptual_roughness: 0.9,
        ..default()
    });

    // --- Matériaux néon (émissif > 1 -> « glow » avec le bloom de la caméra) ---
    // Palette TIRÉE AU HASARD à chaque lancement : une teinte de base + deux
    // teintes contrastées. Ça donne de la diversité, et ça distingue chaque
    // fenêtre/instance d'un coup d'œil (chacune a sa couleur de salle).
    let (grid, grid_soft, wall_c, edge_c) = random_neon_palette();
    let neon_cyan = materials.add(emissive(grid.0, grid.1, grid.2)); // grille du sol
    let neon_magenta = materials.add(emissive(edge_c.0, edge_c.1, edge_c.2)); // arêtes (accent)
    let neon_wall = materials.add(emissive(wall_c.0, wall_c.1, wall_c.2)); // grille des murs
    let neon_ceil = materials.add(emissive(grid_soft.0, grid_soft.1, grid_soft.2)); // grille du plafond
    let fixture_mat = materials.add(emissive(2.0, 1.6, 1.35)); // plafonnier (blanc chaud, fixe)

    // On garde les poignées des matériaux néon : si on est connecté à un serveur,
    // `apply_world_color` les recolore avec la teinte PARTAGÉE par tous les joueurs.
    commands.insert_resource(WorldNeon {
        grid: neon_cyan.clone(),
        grid_soft: neon_ceil.clone(),
        wall: neon_wall.clone(),
        edge: neon_magenta.clone(),
    });

    let t = 0.1; // épaisseur des parois

    // --- Sol et plafond ---
    spawn_box(&mut commands, &cube, &floor_mat,
        Vec3::new(0.0, -t / 2.0, 0.0), Vec3::new(ROOM_SIZE, t, ROOM_SIZE));
    spawn_box(&mut commands, &cube, &ceil_mat,
        Vec3::new(0.0, ROOM_HEIGHT + t / 2.0, 0.0), Vec3::new(ROOM_SIZE, t, ROOM_SIZE));

    // --- 4 murs ---
    for z in [-half, half] {
        spawn_box(&mut commands, &cube, &wall_mat,
            Vec3::new(0.0, ROOM_HEIGHT / 2.0, z), Vec3::new(ROOM_SIZE, ROOM_HEIGHT, t));
    }
    for x in [-half, half] {
        spawn_box(&mut commands, &cube, &wall_mat,
            Vec3::new(x, ROOM_HEIGHT / 2.0, 0.0), Vec3::new(t, ROOM_HEIGHT, ROOM_SIZE));
    }

    // --- Grille néon au sol (cyan), une ligne tous les mètres ---
    let line_w = 0.03;
    let y_grid = 0.012; // juste au-dessus du sol (évite le z-fighting)
    let mut g = -half;
    while g <= half + 0.001 {
        // lignes parallèles à X (réparties le long de Z)
        spawn_box(&mut commands, &cube, &neon_cyan,
            Vec3::new(0.0, y_grid, g), Vec3::new(ROOM_SIZE, 0.012, line_w));
        // lignes parallèles à Z (réparties le long de X)
        spawn_box(&mut commands, &cube, &neon_cyan,
            Vec3::new(g, y_grid, 0.0), Vec3::new(line_w, 0.012, ROOM_SIZE));
        g += 1.0;
    }

    // --- Grille néon sur les 4 murs (rose/magenta chaud) ---
    // Les lignes se posent juste DEVANT la face intérieure du mur. Sinon elles
    // sont englouties dans l'épaisseur du mur (donc invisibles) — c'était le bug.
    let face = half - t / 2.0 - 0.02;
    spawn_wall_grid(&mut commands, &cube, &neon_wall, true, -face, half);
    spawn_wall_grid(&mut commands, &cube, &neon_wall, true, face, half);
    spawn_wall_grid(&mut commands, &cube, &neon_wall, false, -face, half);
    spawn_wall_grid(&mut commands, &cube, &neon_wall, false, face, half);

    // --- Grille néon au plafond (cyan doux) ---
    let y_ceil = ROOM_HEIGHT - 0.02;
    let mut c = -half + 1.0;
    while c < half - 0.001 {
        spawn_box(&mut commands, &cube, &neon_ceil,
            Vec3::new(0.0, y_ceil, c), Vec3::new(ROOM_SIZE, 0.02, 0.025));
        spawn_box(&mut commands, &cube, &neon_ceil,
            Vec3::new(c, y_ceil, 0.0), Vec3::new(0.025, 0.02, ROOM_SIZE));
        c += 1.0;
    }

    // --- Arêtes lumineuses (magenta) : les 12 arêtes du cube ---
    let e = 0.06; // épaisseur des arêtes
    // 4 arêtes verticales (les coins)
    for x in [-half, half] {
        for z in [-half, half] {
            spawn_box(&mut commands, &cube, &neon_magenta,
                Vec3::new(x, ROOM_HEIGHT / 2.0, z), Vec3::new(e, ROOM_HEIGHT, e));
        }
    }
    // 8 arêtes horizontales (périmètres bas et haut)
    for y_edge in [0.0, ROOM_HEIGHT] {
        for z in [-half, half] {
            spawn_box(&mut commands, &cube, &neon_magenta,
                Vec3::new(0.0, y_edge, z), Vec3::new(ROOM_SIZE, e, e));
        }
        for x in [-half, half] {
            spawn_box(&mut commands, &cube, &neon_magenta,
                Vec3::new(x, y_edge, 0.0), Vec3::new(e, e, ROOM_SIZE));
        }
    }

    // --- Plafonnier visible (la source réelle de la lumière) ---
    spawn_box(&mut commands, &cube, &fixture_mat,
        Vec3::new(0.0, ROOM_HEIGHT - 0.06, 0.0), Vec3::new(1.2, 0.08, 1.2));

    // --- Lumière ponctuelle (avec ombres) sous le plafonnier ---
    commands.spawn((
        WorldGeometry,
        PointLight {
            color: Color::srgb(1.0, 0.85, 0.82), // blanc chaud (ambiance plus accueillante)
            intensity: 1_000_000.0,
            range: 50.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, ROOM_HEIGHT - 0.3, 0.0),
    ));
}

/// Crée un matériau émissif : base noire, couleur émissive (qui « glow »).
fn emissive(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(r, g, b),
        ..default()
    }
}

/// Type d'une palette néon : 4 couleurs (r,g,b) — sol, plafond (plus doux),
/// murs, arêtes (accent).
type Palette = ((f32, f32, f32), (f32, f32, f32), (f32, f32, f32), (f32, f32, f32));

/// Les poignées des matériaux néon recolorables : on les garde pour pouvoir
/// changer la couleur de la salle quand le serveur nous donne sa teinte partagée.
#[derive(Resource)]
pub struct WorldNeon {
    grid: Handle<StandardMaterial>,      // grille du sol
    grid_soft: Handle<StandardMaterial>, // grille du plafond
    wall: Handle<StandardMaterial>,      // grille des murs
    edge: Handle<StandardMaterial>,      // arêtes
}

/// Construit une palette harmonieuse à partir d'une teinte de base (0–360).
/// Sol/plafond suivent la teinte ; murs et arêtes prennent des teintes décalées
/// (≈ +140° et +210°) pour un contraste agréable.
fn palette_from_hue(base: f32) -> Palette {
    let grid = hsv(base, 1.0, 1.3);
    let grid_soft = hsv(base, 0.85, 0.95);
    let wall_c = hsv((base + 140.0) % 360.0, 1.0, 1.2);
    let edge_c = hsv((base + 210.0) % 360.0, 1.0, 1.25);
    (grid, grid_soft, wall_c, edge_c)
}

/// Tire une palette néon au hasard, pour le démarrage (état « pas encore
/// connecté » : chaque fenêtre a sa propre couleur, donc deux fenêtres non
/// connectées au même serveur se distinguent).
fn random_neon_palette() -> Palette {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut x = nanos ^ std::process::id().wrapping_mul(2_654_435_761);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    palette_from_hue((x % 360) as f32)
}

/// Système (mode réseau) : dès que le serveur nous a donné sa teinte de salle,
/// on recolore les néons avec. Résultat : TOUS les joueurs d'un même serveur ont
/// la même salle ; une fenêtre non connectée garde sa couleur aléatoire → on voit
/// d'un coup d'œil si on est bien connecté.
pub fn apply_world_color(
    link: Res<crate::net::NetLink>,
    neon: Res<WorldNeon>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut applied: Local<Option<u16>>,
) {
    let Some(hue) = link.world_hue else {
        return;
    };
    if *applied == Some(hue) {
        return; // déjà appliquée : rien à refaire
    }
    *applied = Some(hue);

    let (grid, grid_soft, wall_c, edge_c) = palette_from_hue(hue as f32);
    recolor(&mut materials, &neon.grid, grid);
    recolor(&mut materials, &neon.grid_soft, grid_soft);
    recolor(&mut materials, &neon.wall, wall_c);
    recolor(&mut materials, &neon.edge, edge_c);
    println!("Salle recolorée à la teinte du serveur : {hue}° (connecté).");
}

/// Change la couleur émissive d'un matériau déjà créé (via sa poignée).
fn recolor(materials: &mut Assets<StandardMaterial>, handle: &Handle<StandardMaterial>, c: (f32, f32, f32)) {
    if let Some(mat) = materials.get_mut(handle) {
        mat.emissive = LinearRgba::rgb(c.0, c.1, c.2);
    }
}

/// Convertit Teinte/Saturation/Valeur en Rouge/Vert/Bleu (valeur > 1 autorisée
/// pour le « glow » néon).
fn hsv(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - ((h2 % 2.0) - 1.0).abs());
    let (r, g, b) = match h2 as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r + m, g + m, b + m)
}

/// Spawn un pavé : le cube unitaire, translaté puis mis à l'échelle voulue.
fn spawn_box(
    commands: &mut Commands,
    cube: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    commands.spawn((
        WorldGeometry, // scène-scopée : dé-spawn quand on quitte l'arcade
        Mesh3d(cube.clone()),
        MeshMaterial3d(material.clone()),
        Transform::from_translation(position).with_scale(size),
    ));
}

/// Trace une grille de lignes néon sur un mur vertical.
/// - `along_x = true`  : mur perpendiculaire à Z (s'étend selon X), placé à z = `fixed`.
/// - `along_x = false` : mur perpendiculaire à X (s'étend selon Z), placé à x = `fixed`.
fn spawn_wall_grid(
    commands: &mut Commands,
    cube: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    along_x: bool,
    fixed: f32,
    half: f32,
) {
    let line = 0.025; // épaisseur des lignes
    let depth = 0.02; // épaisseur vers l'intérieur de la salle
    let step = 1.0; // même pas que le sol -> grille cohérente sur les 6 faces

    // Lignes horizontales, à différentes hauteurs.
    let mut y = step;
    while y < ROOM_HEIGHT - 0.001 {
        let (pos, size) = if along_x {
            (Vec3::new(0.0, y, fixed), Vec3::new(ROOM_SIZE, line, depth))
        } else {
            (Vec3::new(fixed, y, 0.0), Vec3::new(depth, line, ROOM_SIZE))
        };
        spawn_box(commands, cube, mat, pos, size);
        y += step;
    }

    // Lignes verticales, réparties le long du mur.
    let mut u = -half + step;
    while u < half - 0.001 {
        let (pos, size) = if along_x {
            (Vec3::new(u, ROOM_HEIGHT / 2.0, fixed), Vec3::new(line, ROOM_HEIGHT, depth))
        } else {
            (Vec3::new(fixed, ROOM_HEIGHT / 2.0, u), Vec3::new(depth, ROOM_HEIGHT, line))
        };
        spawn_box(commands, cube, mat, pos, size);
        u += step;
    }
}
