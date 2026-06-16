//! La salle : sol, murs, plafond, grille néon au sol, arêtes lumineuses,
//! plafonnier visible et éclairage. Ambiance « arcade synthwave ».

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
    let neon_cyan = materials.add(emissive(0.0, 1.0, 1.3)); // grille du sol (cyan adouci)
    let neon_magenta = materials.add(emissive(1.3, 0.0, 0.95)); // arêtes
    let neon_wall = materials.add(emissive(1.3, 0.12, 0.5)); // grille des murs (rose chaud, adouci)
    let neon_ceil = materials.add(emissive(0.0, 0.7, 0.95)); // grille du plafond (cyan plus doux)
    let fixture_mat = materials.add(emissive(2.0, 1.6, 1.35)); // plafonnier (blanc chaud)

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

/// Spawn un pavé : le cube unitaire, translaté puis mis à l'échelle voulue.
fn spawn_box(
    commands: &mut Commands,
    cube: &Handle<Mesh>,
    material: &Handle<StandardMaterial>,
    position: Vec3,
    size: Vec3,
) {
    commands.spawn((
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
