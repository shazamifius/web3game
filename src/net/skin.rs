//! LA COULEUR DE SKIN : une couleur vive aléatoire, choisie une fois au lancement.
//!
//! Le perso local ET le réseau utilisent la même (rangée dans une ressource Bevy).

use bevy::prelude::Resource;

/// La couleur de skin de CETTE session, choisie au démarrage.
#[derive(Resource, Clone, Copy)]
pub struct MyColor(pub f32, pub f32, pub f32);

/// Tire une couleur vive aléatoire (rouge/vert/bleu, valeurs faites pour « glow »).
/// On évite toute dépendance externe : petit générateur pseudo-aléatoire maison.
pub fn random_color() -> (f32, f32, f32) {
    hsv_to_rgb(random_hue() as f32, 1.0, 1.2) // saturation max, valeur > 1 pour le néon
}

/// Tire une teinte au hasard (0–359) sur le cercle des couleurs. Sert au skin du
/// joueur ET à la couleur de salle choisie par le serveur de rendez-vous.
pub(crate) fn random_hue() -> u16 {
    // Graine = nanosecondes actuelles, mélangées à l'identifiant du processus
    // (pour que deux fenêtres lancées au même instant aient des couleurs différentes).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut x = nanos ^ std::process::id().wrapping_mul(2_654_435_761);
    // « xorshift » : on brasse les bits pour obtenir un nombre bien mélangé.
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    (x % 360) as u16
}

/// Convertit une couleur Teinte/Saturation/Valeur en Rouge/Vert/Bleu.
/// (La teinte donne « quelle couleur » ; on s'en sert pour tirer au hasard.)
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
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
