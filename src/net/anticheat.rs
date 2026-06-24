//! ANTICHEAT — le « Shield » local : des règles de PLAUSIBILITÉ physique appliquées
//! à ce qu'on reçoit. La signature (chap. 5/6.1) prouve QUI a envoyé un état ; elle
//! ne dit RIEN sur le fait que cet état soit physiquement possible. Ici on ajoute
//! cette deuxième barrière : un mouvement signé mais impossible (téléport, speed-hack)
//! est refusé ET compté comme faute ATTRIBUABLE (c'est bien le détenteur de la clé
//! qui l'a signé). C'est le pendant « monde » des bornes déjà posées sur l'orbe.

use crate::math::Vec3;

/// Vitesse maximale plausible d'un joueur (m/s). Très généreuse : un humain sprinte
/// à ~10 m/s ; on laisse de la marge pour un dash, une chute, un saut. Au-delà, c'est
/// un téléport ou un speed-hack. Réglage à affiner avec la vraie vitesse du jeu.
pub(crate) const MAX_SPEED: f32 = 30.0;

/// Marge fixe (m) tolérée en plus, pour absorber le jitter de spawn et les arrondis.
const SLACK: f32 = 1.0;

/// Un déplacement de `prev` vers `now_pos` en `dt` secondes est-il plausible ?
///   - `dt` quasi nul (même image / paquets en rafale) → on accepte (rien à juger) ;
///   - sinon, on accepte si la distance parcourue tient sous `MAX_SPEED · dt` (+ marge).
/// On compare au CARRÉ pour éviter une racine carrée (moins cher, même verdict).
pub(crate) fn move_plausible(prev: Vec3, now_pos: Vec3, dt: f32) -> bool {
    if dt <= 1e-3 {
        return true; // pas d'intervalle exploitable : on ne juge pas
    }
    let max_dist = MAX_SPEED * dt + SLACK;
    prev.distance_squared(now_pos) <= max_dist * max_dist
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marche_normale_est_plausible() {
        // 1,5 m en 0,1 s = 15 m/s : sous la borne → accepté.
        assert!(move_plausible(Vec3::ZERO, Vec3::new(1.5, 0.0, 0.0), 0.1));
    }

    #[test]
    fn teleport_est_refuse() {
        // 50 m en 0,3 s ≈ 166 m/s : très au-dessus → refusé.
        assert!(!move_plausible(Vec3::ZERO, Vec3::new(50.0, 0.0, 0.0), 0.3));
        // 1000 m d'un coup : refusé quelle que soit la marge.
        assert!(!move_plausible(Vec3::ZERO, Vec3::new(1000.0, 0.0, 0.0), 0.3));
    }

    #[test]
    fn intervalle_nul_est_tolere() {
        // dt ~ 0 (rafale) : on ne peut rien juger → accepté.
        assert!(move_plausible(Vec3::ZERO, Vec3::new(500.0, 0.0, 0.0), 0.0));
    }

    #[test]
    fn longue_absence_autorise_un_grand_pas() {
        // Silencieux 5 s (perte de paquets) puis 100 m : 20 m/s → plausible.
        assert!(move_plausible(Vec3::ZERO, Vec3::new(100.0, 0.0, 0.0), 5.0));
    }
}
