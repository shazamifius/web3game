//! AREA OF INTEREST par ALLOCATION DE BUDGET (water-filling).
//!
//! On ne supprime JAMAIS un pair par règle. On RÉPARTIT un budget d'émission fini
//! entre tous les pairs, proportionnellement à leur PERTINENCE, en plafonnant
//! chaque débit. La dégradation n'apparaît que si le budget est saturé ; sinon
//! tout le monde a le plein débit, peu importe la distance.
//!
//! Deux briques :
//!   1) `relevance_weight` : un poids `w` par pair (distance douce + un socle pour
//!      ne jamais tomber à zéro ; on y ajoutera champ de vision / interaction).
//!   2) `allocate_rates` : le water-filling. On monte un niveau commun `λ`
//!      (débit_i = λ·w_i), on plafonne ceux qui dépassent `R_max` et on redonne
//!      leur surplus aux autres, jusqu'à dépenser le budget.

/// Borne GROSSIÈRE de candidats (m). À l'échelle planétaire, on bornerait ici
/// l'ensemble des joueurs connus (via un index spatial). Réglée très grand →
/// dans une instance normale, personne n'est jamais exclu par cette borne ;
/// c'est le water-filling, pas un rayon, qui décide des débits.
pub(crate) const CANDIDATE_RADIUS: f32 = 500.0;

/// Budget d'émission total (mises à jour/seconde) qu'on s'autorise vers TOUS les
/// pairs réunis. Plus tard : fonction de la qualité du lien (bon wifi = grand B).
pub(crate) const SEND_BUDGET_HZ: f32 = 240.0;

/// Nombre MAXIMAL de voisins qu'un joueur suit / à qui il parle (chap. 6.6). C'est
/// LA borne d'échelle : sans elle, chacun parlerait à tous → O(N²) de connexions et
/// un WELCOME qui déborde le tampon. Le rendez-vous ne renvoie donc que les
/// `MAX_NEIGHBORS` pairs les PLUS PROCHES ; au-delà, on n'existe pas pour vous.
/// 32 × 38 octets par fiche + en-tête tient largement dans un paquet (< 2 Ko).
pub(crate) const MAX_NEIGHBORS: usize = 32;

/// Garde les `k` éléments de plus petite « distance » (le 2e membre de chaque
/// couple), triés du plus proche au plus loin. Sert au rendez-vous à ne présenter
/// que le voisinage le plus proche (borne d'échelle, chap. 6.6).
pub(crate) fn keep_nearest<T>(mut items: Vec<(T, f32)>, k: usize) -> Vec<T> {
    items.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    items.truncate(k);
    items.into_iter().map(|(t, _)| t).collect()
}

/// Distance « de confort » (m) : en deçà, un pair est très pertinent.
const COMFORT_DIST: f32 = 6.0;
/// Socle de pertinence : même très loin, un pair garde un filet (jamais 0).
const WEIGHT_FLOOR: f32 = 0.05;

/// Distance au carré entre deux positions (x, z). On évite la racine carrée :
/// pour comparer/pondérer, le carré suffit (et c'est moins cher).
pub(crate) fn dist2(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = a.0 - b.0;
    let dz = a.1 - b.1;
    dx * dx + dz * dz
}

/// Deux positions sont-elles dans la borne grossière de candidats ?
pub(crate) fn within_radius(a: (f32, f32), b: (f32, f32)) -> bool {
    dist2(a, b) <= CANDIDATE_RADIUS * CANDIDATE_RADIUS
}

/// Poids de pertinence d'un pair, à partir de la distance² qui nous sépare.
/// Descente douce `1 / (1 + (d/d0)²)` (vaut ~1 tout près, tend vers 0 au loin)
/// + un socle pour ne jamais atteindre zéro. (À enrichir : champ de vision,
/// interaction, attention récente — il suffira d'ajouter des termes ici.)
pub(crate) fn relevance_weight(d2: f32) -> f32 {
    WEIGHT_FLOOR + 1.0 / (1.0 + d2 / (COMFORT_DIST * COMFORT_DIST))
}

/// WATER-FILLING : répartit `budget` (maj/s) entre des `weights`, chaque débit
/// plafonné à `r_max`. Renvoie un débit (Hz) par poids, dans le même ordre.
///
/// Principe (cf. le cours) : on monte un niveau commun `λ` tel que débit_i = λ·w_i.
/// Ceux qui dépasseraient `r_max` sont fixés à `r_max` et leur surplus est rendu
/// au reste ; on recommence avec le budget restant. Converge en quelques passes.
pub(crate) fn allocate_rates(weights: &[f32], budget: f32, r_max: f32) -> Vec<f32> {
    let n = weights.len();
    let mut rates = vec![0.0f32; n];
    let mut capped = vec![false; n];
    let mut remaining = budget;

    loop {
        // Somme des poids encore non plafonnés.
        let sum_w: f32 = (0..n).filter(|&i| !capped[i]).map(|i| weights[i]).sum();
        if sum_w <= 0.0 || remaining <= 0.0 {
            break;
        }
        let lambda = remaining / sum_w;

        // Qui dépasse le plafond à ce niveau ? On le plafonne et on rend son surplus.
        let mut newly_capped = false;
        for i in 0..n {
            if !capped[i] && lambda * weights[i] > r_max {
                capped[i] = true;
                rates[i] = r_max;
                remaining -= r_max;
                newly_capped = true;
            }
        }

        // Personne ne dépasse : les non-plafonnés prennent λ·w et c'est fini.
        if !newly_capped {
            for i in 0..n {
                if !capped[i] {
                    rates[i] = lambda * weights[i];
                }
            }
            break;
        }
    }
    rates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poids_decroit_avec_distance_sans_jamais_zero() {
        assert!(relevance_weight(0.0) > relevance_weight(100.0));
        assert!(relevance_weight(1.0e9) >= WEIGHT_FLOOR); // jamais nul, même très loin
    }

    #[test]
    fn water_filling_exemple_du_cours() {
        // B = 50, R_max = 20, poids [8, 3, 2, 1] → débits [20, 15, 10, 5].
        let r = allocate_rates(&[8.0, 3.0, 2.0, 1.0], 50.0, 20.0);
        let approx = |a: f32, b: f32| (a - b).abs() < 0.01;
        assert!(approx(r[0], 20.0));
        assert!(approx(r[1], 15.0));
        assert!(approx(r[2], 10.0));
        assert!(approx(r[3], 5.0));
        assert!((r.iter().sum::<f32>() - 50.0).abs() < 0.01); // budget exactement utilisé
    }

    #[test]
    fn water_filling_non_sature_tout_le_monde_au_plafond() {
        // Budget énorme (2 joueurs) → plein débit pour tous, peu importe le reste.
        let r = allocate_rates(&[1.0, 1.0], 1000.0, 20.0);
        assert!(r.iter().all(|&x| (x - 20.0).abs() < 0.01));
    }

    #[test]
    fn keep_nearest_garde_les_plus_proches_dans_l_ordre() {
        let v = vec![("loin", 100.0), ("pres", 1.0), ("moyen", 10.0)];
        assert_eq!(keep_nearest(v, 2), vec!["pres", "moyen"]);
        // k plus grand que la liste : on renvoie tout, trié.
        let v = vec![("b", 2.0), ("a", 1.0)];
        assert_eq!(keep_nearest(v, 9), vec!["a", "b"]);
    }
}
