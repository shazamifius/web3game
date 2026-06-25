//! Champ d'ÉTOILES déterministe (Jalon 3 — « l'île aux étoiles »).
//!
//! # L'idée et le MUR (pensé sur le papier avant de coder)
//! Le jeu fait tomber des étoiles qu'on ramasse. Le piège serait de **réseau-ter
//! chaque étoile** (qui apparaît, où, quand) : à plusieurs ça inonde le réseau et
//! deux joueurs risquent de voir des champs différents. La parade = le **déterminisme** :
//! à partir d'une **graine partagée** (`seed`, propre au monde/à la session) et du
//! **temps écoulé**, CHAQUE pair calcule LE MÊME champ d'étoiles, tout seul, sans
//! échanger un octet. C'est la même philosophie que les météorites de l'ancien proto,
//! mais portée dans le CŒUR pur (engine-agnostique), donc réutilisable par Unreal.
//!
//! # Ce qui est déterministe (cheap) vs ce qui est autoritaire (le vrai netcode)
//! - **L'APPARITION d'une étoile** (position, instant) est déterministe et **tolérante** :
//!   si deux pairs sont désynchronisés d'une seconde, ils voient l'étoile tomber un poil
//!   décalée — c'est **cosmétique**, ça ne casse rien.
//! - **LE RAMASSAGE**, lui, est un **événement d'AUTORITÉ** : qui a eu l'étoile, une seule
//!   fois, sans double-ramassage. Ça NE vit PAS ici — ça réutilisera la logique ORBE+OWN
//!   prouvée (`orb.rs` : `supersedes`/`apply_incoming`), branchée au palier 4 du sidecar.
//!   *On garde donc ce module volontairement PUR et sans réseau.*
//!
//! # Le PRNG
//! Pas de dépendance externe (`rand`) — on reste sur des crates minimales (cf. `Cargo.toml`).
//! On utilise `splitmix64`, un générateur déterministe minuscule et de bonne qualité de
//! dispersion, suffisant pour disperser des positions (ce n'est PAS de la crypto).

use crate::math::Vec3;

/// Secondes entre deux apparitions d'étoile. **Volontairement lent** (cf. la DA : évolution
/// lente pour forcer le social). Une étoile toutes les 5 s.
pub const WAVE_PERIOD: f64 = 5.0;

/// Rayon de l'île jouable (mètres). Le sol placeholder fait ~200 m de côté → on reste dedans.
pub const ISLAND_RADIUS: f32 = 90.0;
/// Au-delà de ce rayon (mais dans l'île), l'étoile tombe dans l'EAU (cf. DA : terre OU eau).
pub const LAND_RADIUS: f32 = 60.0;
/// Hauteur du sol et de l'eau au point d'atterrissage.
pub const GROUND_Y: f32 = 0.0;
pub const WATER_Y: f32 = -1.0;

const TAU: f32 = std::f32::consts::TAU;

/// Une étoile : identité STABLE (dérivée de la graine + l'onde), où elle tombe, et quand.
/// `id` sert de clé d'autorité au ramassage (palier 4) — deux pairs s'accordent dessus.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Star {
    /// Identité déterministe et globalement unique pour (seed, onde). Clé du ramassage.
    pub id: u64,
    /// Instant d'apparition (secondes depuis l'origine de session partagée).
    pub spawn_t: f64,
    /// Point d'atterrissage dans le monde (x, y, z), y = sol ou eau.
    pub landing: Vec3,
    /// Vraie si elle tombe dans l'eau (au-delà de `LAND_RADIUS`).
    pub in_water: bool,
}

// NB : l'altitude d'apparition et l'animation de chute sont du COSMÉTIQUE côté client
// (Unreal) — le cœur ne décide que d'OÙ l'étoile atterrit, de QUAND, et de son `id`.

/// splitmix64 — mélangeur déterministe minuscule. Une passe = un `u64` bien dispersé.
#[inline]
fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Convertit 64 bits aléatoires en un `f32` dans [0, 1) (24 bits de mantisse).
#[inline]
fn unit_f32(bits: u64) -> f32 {
    ((bits >> 40) as u32) as f32 / 16_777_216.0 // 2^24
}

/// L'étoile de l'onde `wave` pour la graine `seed` — PURE et déterministe.
pub fn star_for_wave(seed: u64, wave: u64) -> Star {
    // On dérive un flux de hashs indépendants en chaînant splitmix64 à partir d'une graine
    // mêlant `seed` et `wave` (chacun passé au mélangeur pour casser les corrélations).
    let mut h = splitmix64(seed ^ splitmix64(wave.wrapping_add(0xA5A5_A5A5)));
    let mut next = || {
        h = splitmix64(h);
        h
    };

    // Échantillonnage UNIFORME dans un disque : angle libre, rayon en sqrt (sinon ça s'entasse au centre).
    let angle = unit_f32(next()) * TAU;
    let radius = ISLAND_RADIUS * unit_f32(next()).sqrt();
    let in_water = radius > LAND_RADIUS;
    let landing = Vec3::new(
        radius * angle.cos(),
        if in_water { WATER_Y } else { GROUND_Y },
        radius * angle.sin(),
    );

    // Instant : base de l'onde + une gigue dans la période (les étoiles ne tombent pas pile en cadence).
    let jitter = unit_f32(next()) as f64 * WAVE_PERIOD;
    let spawn_t = wave as f64 * WAVE_PERIOD + jitter;

    // Identité stable : un hash de plus, indépendant des positions.
    let id = next();

    Star { id, spawn_t, landing, in_water }
}

/// Toutes les étoiles dont l'apparition tombe dans la fenêtre [t0, t1).
/// Déterministe : mêmes (seed, t0, t1) → même `Vec` exact, sur n'importe quelle machine.
pub fn field_window(seed: u64, t0: f64, t1: f64) -> Vec<Star> {
    debug_assert!(t1 >= t0);
    // Une étoile de l'onde `w` peut apparaître jusqu'à WAVE_PERIOD après `w*WAVE_PERIOD` (gigue).
    // On élargit donc la plage d'ondes scannées d'un cran de chaque côté, puis on filtre sur spawn_t.
    let first = ((t0 / WAVE_PERIOD).floor() as i64 - 1).max(0) as u64;
    let last = (t1 / WAVE_PERIOD).ceil() as i64 as u64;
    let mut out = Vec::new();
    for w in first..=last {
        let s = star_for_wave(seed, w);
        if s.spawn_t >= t0 && s.spawn_t < t1 {
            out.push(s);
        }
    }
    out
}

/// Sous-commande `stars <seed> <secs>` : imprime le champ déterministe sur [0, secs).
/// **Preuve reproductible** : la lancer DEUX fois avec les mêmes arguments donne une sortie
/// IDENTIQUE (c'est tout l'intérêt). Sert de juge neutre du déterminisme, sans GPU ni réseau.
pub fn run_stars(seed_arg: &str, secs_arg: &str) {
    let seed: u64 = seed_arg.parse().unwrap_or(1);
    let secs: f64 = secs_arg.parse().unwrap_or(30.0);
    let champ = field_window(seed, 0.0, secs);
    println!(
        "Champ d'étoiles — seed={seed}, fenêtre=[0, {secs}s), période={}s → {} étoiles",
        WAVE_PERIOD,
        champ.len()
    );
    for s in &champ {
        let lieu = if s.in_water { "eau  " } else { "terre" };
        println!(
            "  #{:016x}  t={:6.2}s  {lieu}  pos=({:7.2}, {:7.2})",
            s.id, s.spawn_t, s.landing.x, s.landing.z
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le déterminisme PUR : même graine → champ identique au bit près (rejouable partout).
    #[test]
    fn meme_graine_meme_champ() {
        let a = field_window(42, 0.0, 120.0);
        let b = field_window(42, 0.0, 120.0);
        assert_eq!(a, b, "même seed/fenêtre doit donner exactement le même champ");
        // Une étoile isolée est stable elle aussi.
        assert_eq!(star_for_wave(42, 7), star_for_wave(42, 7));
    }

    /// Deux graines différentes divergent (sinon le « monde partagé » serait toujours le même).
    #[test]
    fn graines_differentes_divergent() {
        let s1 = star_for_wave(1, 5);
        let s2 = star_for_wave(2, 5);
        assert_ne!(s1.id, s2.id, "des graines différentes doivent donner des id différents");
        assert!(
            s1.landing != s2.landing,
            "des graines différentes doivent placer l'étoile ailleurs"
        );
    }

    /// Compte BORNÉ et reproductible : sur [0, 60s) à une étoile / 5 s → exactement 12.
    /// (Chiffre figé : si la cadence change, ce test DOIT être mis à jour sciemment.)
    #[test]
    fn compte_reproductible() {
        let champ = field_window(7, 0.0, 60.0);
        assert_eq!(champ.len(), 12, "60 s / 5 s = 12 étoiles attendues");
    }

    /// Invariants géométriques : dans l'île, à la bonne altitude, dans la fenêtre.
    #[test]
    fn etoiles_bien_formees() {
        for s in field_window(123, 0.0, 300.0) {
            let r = (s.landing.x * s.landing.x + s.landing.z * s.landing.z).sqrt();
            assert!(r <= ISLAND_RADIUS + 0.01, "étoile hors de l'île (r={r})");
            let attendu_y = if s.in_water { WATER_Y } else { GROUND_Y };
            assert_eq!(s.landing.y, attendu_y, "y incohérent avec terre/eau");
            assert!(s.in_water == (r > LAND_RADIUS), "drapeau eau incohérent avec le rayon");
            assert!(s.spawn_t >= 0.0 && s.spawn_t < 300.0, "spawn hors fenêtre");
        }
    }

    /// Les fenêtres se recollent sans trou ni doublon : [0,60) ∪ [60,120) == [0,120).
    #[test]
    fn fenetres_contigues_sans_trou_ni_doublon() {
        let mut a = field_window(9, 0.0, 60.0);
        let b = field_window(9, 60.0, 120.0);
        let tout = field_window(9, 0.0, 120.0);
        a.extend(b);
        assert_eq!(a, tout, "la concaténation de deux fenêtres doit égaler la fenêtre globale");
    }

    /// Le mélange terre/eau existe (sinon le `LAND_RADIUS` ne servirait à rien).
    #[test]
    fn melange_terre_et_eau() {
        let champ = field_window(2024, 0.0, 2000.0); // ~400 étoiles, échantillon large
        let eau = champ.iter().filter(|s| s.in_water).count();
        let terre = champ.len() - eau;
        assert!(eau > 0 && terre > 0, "on veut des étoiles sur la terre ET dans l'eau (eau={eau}, terre={terre})");
    }
}
