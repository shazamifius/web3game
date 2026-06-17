//! AREA OF INTEREST (« zone d'intérêt ») : on ne se synchronise qu'avec les
//! joueurs PROCHES, pas avec tout le monde.
//!
//! On découpe le monde en cases carrées. Un joueur ne « voit » sur le réseau que
//! les joueurs de sa case et des 8 cases adjacentes (un bloc 3×3 autour de lui).
//! Résultat : chacun ne parle qu'à ~une poignée de voisins au lieu de TOUS →
//! on passe d'un coût en O(N²) (chacun parle à chacun) à ~O(N).
//!
//! Ici la salle est petite (12 m), donc les cases sont volontairement petites
//! pour que l'effet soit VISIBLE : un joueur à un coin ne voit pas celui du coin
//! opposé. Dans un vrai grand monde, on choisirait des cases à la bonne échelle.

/// Côté d'une case, en mètres.
pub(crate) const CELL_SIZE: f32 = 4.0;

/// La case (colonne, ligne) qui contient la position (x, z).
pub(crate) fn cell_of(x: f32, z: f32) -> (i8, i8) {
    ((x / CELL_SIZE).floor() as i8, (z / CELL_SIZE).floor() as i8)
}

/// Deux cases sont-elles voisines (la même, ou adjacentes — bloc 3×3) ?
pub(crate) fn is_neighbor(a: (i8, i8), b: (i8, i8)) -> bool {
    (a.0 - b.0).abs() <= 1 && (a.1 - b.1).abs() <= 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cases_et_voisinage() {
        // Centre de la salle, puis deux coins opposés (salle de 12 m, cases de 4 m).
        assert_eq!(cell_of(0.0, 0.0), (0, 0));
        assert_eq!(cell_of(-6.0, 5.9), (-2, 1));
        assert_eq!(cell_of(5.9, -6.0), (1, -2));

        // Même case et cases adjacentes = voisins.
        assert!(is_neighbor((0, 0), (0, 0)));
        assert!(is_neighbor((0, 0), (1, 1)));
        assert!(is_neighbor((0, 0), (-1, 1)));

        // Deux cases d'écart (coins opposés de la salle) = PAS voisins → filtrés.
        assert!(!is_neighbor((-2, -2), (1, 1)));
        assert!(!is_neighbor((0, 0), (2, 0)));
    }
}
