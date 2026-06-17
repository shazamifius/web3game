//! AREA OF INTEREST par PERTINENCE (pas par grille).
//!
//! On ne synchronise pas « les gens de ma case » mais « les gens que je peux
//! réellement percevoir », classés par **distance**. Deux niveaux :
//!
//!   1) RAYON (côté rendez-vous) : il ne te donne que les joueurs dans un cercle
//!      autour de toi — un seuil DOUX et symétrique, pas un carré arbitraire.
//!   2) BUDGET DE PRIORITÉ (côté client) : parmi ces voisins, tu donnes le plein
//!      débit aux plus PROCHES (jusqu'à FULL_BUDGET), et un débit réduit aux
//!      plus lointains. Résultat : une foule ne te coûte jamais plus qu'un budget
//!      fixe, et l'expérience se dégrade en douceur au lieu d'un mur dedans/dehors.
//!
//! La grille reste utile ailleurs, mais seulement comme INDEX rapide — jamais
//! comme la règle. Ici la règle, c'est la distance.

/// Rayon de perception (m) : au-delà, le rendez-vous ne te parle plus du joueur.
pub(crate) const AOI_RADIUS: f32 = 8.0;
/// Nombre de voisins servis à plein débit (les plus proches). Au-delà : réduit.
pub(crate) const FULL_BUDGET: usize = 8;
/// Les voisins « lointains » reçoivent 1 paquet sur REDUCE_FACTOR (débit réduit).
pub(crate) const REDUCE_FACTOR: u32 = 4;

/// Distance au carré entre deux positions (x, z). On évite la racine carrée :
/// pour comparer/trier des distances, le carré suffit (et c'est moins cher).
pub(crate) fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dz = a.1 - b.1;
    dx * dx + dz * dz
}

/// Deux positions sont-elles à portée de perception (dans le rayon) ?
pub(crate) fn within_radius(a: (f32, f32), b: (f32, f32)) -> bool {
    dist2(a, b) <= AOI_RADIUS * AOI_RADIUS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rayon_et_distance() {
        // Triangle 3-4-5 : distance 5 < 8 → à portée.
        assert!(within_radius((0.0, 0.0), (3.0, 4.0)));
        // Coins opposés de la salle (12 m) : ~17 m → hors portée.
        assert!(!within_radius((-6.0, -6.0), (6.0, 6.0)));
        // Le tri du budget de priorité s'appuie sur dist2 : plus proche = plus petit.
        assert!(dist2((0.0, 0.0), (1.0, 0.0)) < dist2((0.0, 0.0), (5.0, 0.0)));
    }
}
