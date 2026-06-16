//! PRÉDICTION : à partir de l'historique, donner l'état du joueur à un instant `t`.
//!
//! Trois cas :
//!   - avant le 1er instantané connu → on tient sa position (rien à deviner) ;
//!   - entre deux instantanés → INTERPOLATION (on glisse de l'un à l'autre) ;
//!   - au-delà du dernier → PRÉDICTION par extrapolation de la vraie vitesse.

use super::smooth::{lerp_angle, shortest_diff};
use super::state::{Snapshot, MAX_EXTRAPOLATION};
use bevy::prelude::Vec3;
use std::collections::VecDeque;

pub(super) fn sample(buf: &VecDeque<Snapshot>, t: f32) -> (Vec3, f32, f32) {
    // Avant le plus ancien instantané connu : on tient sa position (rien à deviner).
    let first = buf.front().unwrap();
    if t <= first.t {
        return (first.pos, first.yaw, first.pitch);
    }
    // Cas normal : on cherche la paire (a, b) qui encadre `t` et on glisse de a vers b.
    for i in 0..buf.len() - 1 {
        let a = buf[i];
        let b = buf[i + 1];
        if t <= b.t {
            // `alpha` = où se trouve `t` entre a.t et b.t, ramené dans [0, 1].
            let span = (b.t - a.t).max(1e-6); // évite la division par zéro
            let alpha = ((t - a.t) / span).clamp(0.0, 1.0);
            let pos = a.pos.lerp(b.pos, alpha);
            let yaw = lerp_angle(a.yaw, b.yaw, alpha);
            let pitch = a.pitch + (b.pitch - a.pitch) * alpha;
            return (pos, yaw, pitch);
        }
    }
    // Au-delà du dernier instantané : la file est épuisée (paquet en retard ou
    // perdu). PRÉDICTION : on prolonge le mouvement, le temps que le vrai paquet
    // arrive. On borne à MAX_EXTRAPOLATION pour ne pas partir n'importe où si le
    // joueur s'est déconnecté d'un coup.
    let last = buf.back().unwrap();
    let ahead = (t - last.t).min(MAX_EXTRAPOLATION);
    // Position : on prolonge avec la VRAIE vitesse reçue (Axe 1) — bien plus stable
    // qu'une vitesse estimée sur deux points bruités.
    let pos = last.pos + last.vel * ahead;
    // Orientation : on n'envoie pas (encore) la vitesse de rotation, donc on
    // l'estime sur les deux derniers points si on les a, sinon on tient l'angle.
    let yaw = if buf.len() >= 2 {
        let prev = buf[buf.len() - 2];
        let dt = (last.t - prev.t).max(1e-6);
        last.yaw + shortest_diff(prev.yaw, last.yaw) / dt * ahead
    } else {
        last.yaw
    };
    (pos, yaw, last.pitch) // le tangage de tête, lui, on ne le prédit pas
}
