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

/// Nombre de pairs au FOCUS (chap. 8.2) : lien plein débit (jusqu'à `SEND_HZ`), prédiction,
/// avatar 3D détaillé. C'est le sous-ensemble servi en PRIORITÉ ; tout le reste de la table
/// est la CONSCIENCE (basse fidélité). **Pourquoi 8 et pas 16 :** le budget total est
/// `SEND_BUDGET_HZ = 240` ; à `SEND_HZ = 20`, 16 pairs en focus mangeraient `16×20 = 320 > 240`
/// → ils videraient TOUT le budget et la conscience tomberait à 0 (on ne verrait plus la foule).
/// 8 focus = `8×20 = 160`, ce qui laisse **80 Hz garantis** à la conscience. (Réglage assumé, à
/// recalibrer si besoin ; cf. registre des dettes — réglages AoI.)
pub(crate) const K_FOCUS: usize = 8;

/// Débit MAX d'un pair en CONSCIENCE (Hz, chap. 8.2) : basse fidélité — « il existe / il est
/// là », pas de prédiction fine, rendu LOD/imposteur. Plafond bas exprès : la conscience ne
/// prend que des miettes, le gros du budget va au focus.
pub(crate) const CONSCIENCE_HZ: f32 = 2.0;

/// Marge anti-oscillation du focus COLLANT (chap. 8.2a-bis) : un pair hors focus ne DÉLOGE
/// un membre du focus que s'il est au moins ce facteur PLUS pertinent. Sans cette marge, en
/// foule dense des dizaines de pairs à pertinence quasi égale s'échangeraient la place à chaque
/// tick (le « churn » mesuré au 8.2b) → aucun lien plein débit soutenu → retour au « tout flou ».
pub(crate) const FOCUS_SWAP_MARGIN: f32 = 1.5;

/// AoI À DEUX TIERS (chap. 8.2 / 8.2a-bis) — répartit le budget entre un FOCUS (lien plein
/// débit) et une CONSCIENCE (basse fidélité). **Le focus est DONNÉ** (`is_focus`, parallèle à
/// `weights`), pas recalculé ici : il est choisi de façon COLLANTE par `NetLink::refresh_focus`
/// (hystérésis), ce qui évite le « churn » de la foule dense (mesuré au 8.2b : recomposer le
/// top-8 à chaque tick → aucun lien 20 Hz soutenu → tout le monde flou). Les pairs `is_focus`
/// sont water-fillés sur tout le budget (bornés à `r_max` chacun) ; le reste (conscience) prend
/// le budget RÉSIDUEL avec un plafond bas (`CONSCIENCE_HZ`). Renvoie un débit (Hz) par pair,
/// dans l'ORDRE D'ENTRÉE.
///
/// **Invariant préservé :** un nœud est dans le FOCUS de ses ~`K_FOCUS` voisins proches
/// (≈ `K_FOCUS × r_max` Hz reçus) + dans la CONSCIENCE de tous les autres (chaque émetteur lui
/// donne `budget_conscience / (ses conscients)` ; sommé sur ~N émetteurs ≈ le budget conscience
/// d'UN émetteur, **indépendant de N**) → réception ≈ `K_FOCUS × r_max + budget_conscience`,
/// PLATE quand N grandit.
pub(crate) fn allocate_tiers(weights: &[f32], is_focus: &[bool], budget: f32, r_max: f32) -> Vec<f32> {
    let n = weights.len();
    let focus_idx: Vec<usize> = (0..n).filter(|&i| is_focus.get(i).copied().unwrap_or(false)).collect();
    let consc_idx: Vec<usize> = (0..n).filter(|&i| !is_focus.get(i).copied().unwrap_or(false)).collect();

    // FOCUS : tout le budget leur est offert, mais bornés à `r_max` chacun (et ils ne sont que
    // `K_FOCUS`) → ils en utilisent au plus `K_FOCUS × r_max`.
    let focus_w: Vec<f32> = focus_idx.iter().map(|&i| weights[i]).collect();
    let focus_rates = allocate_rates(&focus_w, budget, r_max);
    let focus_used: f32 = focus_rates.iter().sum();

    // CONSCIENCE : ce qui RESTE du budget, plafond bas. Le budget non dépensé par le focus
    // (peu de pairs) profite ainsi à la conscience, sans jamais dépasser le total.
    let consc_budget = (budget - focus_used).max(0.0);
    let consc_w: Vec<f32> = consc_idx.iter().map(|&i| weights[i]).collect();
    let consc_rates = allocate_rates(&consc_w, consc_budget, CONSCIENCE_HZ);

    // Remappe vers l'ordre d'entrée.
    let mut rates = vec![0.0f32; n];
    for (k, &i) in focus_idx.iter().enumerate() {
        rates[i] = focus_rates[k];
    }
    for (k, &i) in consc_idx.iter().enumerate() {
        rates[i] = consc_rates[k];
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

    /// 8.2 — FOULE DENSE : les pairs au focus (masque) sortent au PLEIN débit, le reste en
    /// CONSCIENCE à bas débit mais JAMAIS zéro, et le budget total est respecté.
    #[test]
    fn deux_tiers_focus_plein_conscience_miettes() {
        let weights = vec![1.0f32; 50]; // 50 pairs à pertinence identique (co-localisés)
        let mut is_focus = vec![false; 50];
        for f in is_focus.iter_mut().take(K_FOCUS) {
            *f = true; // K_FOCUS pairs au focus (choisis ailleurs, ici on teste l'alloc)
        }
        let r = allocate_tiers(&weights, &is_focus, 240.0, 20.0);
        let focus = r.iter().filter(|&&x| (x - 20.0).abs() < 0.01).count();
        assert_eq!(focus, K_FOCUS); // les K_FOCUS au plein débit
        assert!(r.iter().all(|&x| x > 0.0)); // personne n'est muet (la foule reste perçue)
        let max_consc = r[K_FOCUS..].iter().cloned().fold(0.0, f32::max);
        assert!(max_consc <= CONSCIENCE_HZ + 0.01); // conscience plafonnée bas
        assert!(r.iter().sum::<f32>() <= 240.0 + 0.01); // budget respecté
    }

    /// 8.2 — peu de pairs tous au focus : tout le monde au plein débit.
    #[test]
    fn deux_tiers_petit_groupe_tout_au_plein_debit() {
        let r = allocate_tiers(&[1.0, 1.0, 1.0], &[true, true, true], 240.0, 20.0);
        assert!(r.iter().all(|&x| (x - 20.0).abs() < 0.01));
    }

    /// 8.2 — c'est l'APPARTENANCE au focus (pas le poids brut) qui décide du tier : un pair
    /// à gros poids mais HORS focus reste plafonné en conscience.
    #[test]
    fn deux_tiers_le_focus_decide_pas_le_poids() {
        let weights = vec![10.0, 0.05, 0.05, 0.05]; // le 1er a un gros poids…
        let is_focus = vec![false, true, false, false]; // …mais n'est PAS au focus
        let r = allocate_tiers(&weights, &is_focus, 240.0, 20.0);
        assert!(r[0] <= CONSCIENCE_HZ + 0.01); // gros poids mais conscience → plafonné bas
        assert!((r[1] - 20.0).abs() < 0.01); // le focus (même petit poids) est servi plein
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
