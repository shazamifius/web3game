//! LISSAGE : le ressort amorti (« SmoothDamp ») et les petits helpers d'angles.
//!
//! Le ressort fait rejoindre une cible de façon « critique » : vite, mais sans
//! jamais dépasser. C'est lui qui absorbe en douceur une correction de prédiction.

use super::state::SMOOTH_TIME;
use bevy::prelude::Vec3;
use std::f32::consts::{PI, TAU};

/// RESSORT AMORTI sur un nombre : fait avancer `current` vers `target` en gardant
/// une vitesse interne `vel` (mise à jour au passage). `SMOOTH_TIME` ≈ le temps
/// mis pour ~rejoindre une cible immobile.
/// (Formule de référence, celle de Unity — dérivée d'un ressort critique.)
pub(super) fn smooth_damp(current: f32, target: f32, vel: &mut f32, dt: f32) -> f32 {
    let omega = 2.0 / SMOOTH_TIME.max(1e-4);
    let x = omega * dt;
    // Approximation rationnelle de e^-x (rapide et stable pour tout dt).
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = current - target;
    let temp = (*vel + omega * change) * dt;
    *vel = (*vel - omega * temp) * exp;
    target + (change + temp) * exp
}

/// Le ressort amorti appliqué composante par composante à un vecteur 3D.
pub(super) fn smooth_damp_vec3(current: Vec3, target: Vec3, vel: &mut Vec3, dt: f32) -> Vec3 {
    Vec3::new(
        smooth_damp(current.x, target.x, &mut vel.x, dt),
        smooth_damp(current.y, target.y, &mut vel.y, dt),
        smooth_damp(current.z, target.z, &mut vel.z, dt),
    )
}

/// Le ressort amorti sur un angle : on « déplie » d'abord la cible vers le plus
/// court chemin pour éviter le saut à ±180°, puis on lisse normalement.
pub(super) fn smooth_damp_angle(current: f32, target: f32, vel: &mut f32, dt: f32) -> f32 {
    let unwrapped_target = current + shortest_diff(current, target);
    smooth_damp(current, unwrapped_target, vel, dt)
}

/// Écart le plus court entre deux angles, dans [−π, π] (gère le passage par
/// ±180° : sinon, en tournant, le corps ferait brièvement un tour à l'envers).
pub(super) fn shortest_diff(a: f32, b: f32) -> f32 {
    let mut diff = (b - a) % TAU;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    diff
}

/// Interpole entre deux angles par le plus court chemin.
pub(super) fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    a + shortest_diff(a, b) * t
}
