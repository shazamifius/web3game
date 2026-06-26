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

use super::crypto::PeerId;

/// Borne GROSSIÈRE de candidats (m). À l'échelle planétaire, on bornerait ici
/// l'ensemble des joueurs connus (via un index spatial). Réglée très grand →
/// dans une instance normale, personne n'est jamais exclu par cette borne ;
/// c'est le water-filling, pas un rayon, qui décide des débits.
pub(crate) const CANDIDATE_RADIUS: f32 = 500.0;

/// Budget d'émission total (mises à jour/seconde) qu'on s'autorise vers TOUS les
/// pairs réunis. Plus tard : fonction de la qualité du lien (bon wifi = grand B).
pub(crate) const SEND_BUDGET_HZ: f32 = 240.0;

/// Côté d'une CELLULE spatiale (m), chap. 8.3 : une cellule = une RÉGION qu'un hôte élu
/// RÉSUME. Plus grand que la portée de focus (~`COMFORT_DIST`) : ce qui est dans une cellule
/// LOINTAINE est perçu via UN résumé basse fréquence, pas N flux individuels → la fraîcheur
/// des lointains ne s'effondre plus en 1/N. Réglage à calibrer (8.3d) : trop grand = résumé
/// grossier ; trop petit = trop de cellules. (Index aussi réutilisé pour le focus en O(K).)
pub(crate) const CELL_SIZE: f32 = 16.0;

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

/// La CELLULE (colonne, rangée) qui contient la position (x, z), chap. 8.3. Grille infinie
/// ancrée à l'origine. `floor` (pas un cast brut) pour que les coordonnées NÉGATIVES tombent
/// dans la bonne case : −0,1 → cellule −1, jamais 0 (un cast `as i32` tronquerait vers 0 et
/// collerait deux régions distinctes dans la même cellule autour de l'origine).
pub(crate) fn cell_of(x: f32, z: f32) -> (i32, i32) {
    ((x / CELL_SIZE).floor() as i32, (z / CELL_SIZE).floor() as i32)
}

/// Poids de pertinence d'un pair, à partir de la distance² qui nous sépare.
/// Descente douce `1 / (1 + (d/d0)²)` (vaut ~1 tout près, tend vers 0 au loin)
/// + un socle pour ne jamais atteindre zéro. (À enrichir : champ de vision,
/// interaction, attention récente — il suffira d'ajouter des termes ici.)
pub(crate) fn relevance_weight(d2: f32) -> f32 {
    WEIGHT_FLOOR + 1.0 / (1.0 + d2 / (COMFORT_DIST * COMFORT_DIST))
}

/// Fraction de pertinence qu'un pair PROCHE transmet à un pair avec qui il se déclare
/// ENGAGÉ (D29, T1). Strictement < 1 : le tiers « présenté » compte, mais JAMAIS plus
/// que l'ami proche qui le présente (on ne s'intéresse pas plus à l'inconnu qu'à son hôte).
#[allow(dead_code)] // EN ATTENTE : prouvé par son test ; câblé à l'étape 2 (le wire `engaged`).
pub(crate) const TRANSITIVE_FRACTION: f32 = 0.5;

/// PERTINENCE PAR TRANSITIVITÉ (D29, T1 — « pertinence ≠ proximité »). Un pair LOIN
/// mais ENGAGÉ avec un de mes pairs PROCHES devient pertinent (« mon voisin lui parle »),
/// là où la seule distance l'aurait laissé au socle. On part de la pertinence SPATIALE
/// (`base[i]`, issue de `relevance_weight`) et on REHAUSSE chaque pair désigné comme
/// partenaire : il hérite de `TRANSITIVE_FRACTION × base[présentateur]`.
///
/// - `ids[i]` = identité du pair i · `base[i]` = sa pertinence spatiale · `engaged[i]` =
///   les ids (quelques-uns) avec qui i se déclare engagé (porté dans son état signé —
///   le wire est branché à l'étape 2 ; ici on prouve la LOGIQUE de sélection, pure).
/// - **Un seul saut** (mes voisins T0 → leurs partenaires) : pas de chaîne transitive,
///   donc ni explosion ni cycle. Renvoie la pertinence effective, dans l'ordre d'entrée.
///
/// Invariant voulu : un pair tiré ne dépasse JAMAIS celui qui le tire (fraction < 1) → la
/// hiérarchie « proches d'abord » est préservée ; on ne fait qu'AJOUTER les pertinents cachés.
#[allow(dead_code)] // EN ATTENTE : logique prouvée (test) ; branchée dans `refresh_focus` à l'étape 3.
pub(crate) fn relevance_transitive(ids: &[PeerId], base: &[f32], engaged: &[Vec<PeerId>]) -> Vec<f32> {
    let mut eff = base.to_vec();
    let index: std::collections::HashMap<PeerId, usize> =
        ids.iter().enumerate().map(|(i, id)| (*id, i)).collect();
    for (i, partners) in engaged.iter().enumerate() {
        let boost = TRANSITIVE_FRACTION * base.get(i).copied().unwrap_or(0.0);
        for partner in partners {
            if let Some(&j) = index.get(partner) {
                if boost > eff[j] {
                    eff[j] = boost;
                }
            }
        }
    }
    eff
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

    /// D29 (T1) — CRITÈRE PRÉ-ENREGISTRÉ : « PERTINENCE ≠ PROXIMITÉ ». Un pair LOIN mais
    /// ENGAGÉ avec un de mes pairs PROCHES doit devenir pertinent (tiré par transitivité),
    /// là où un pair tout aussi loin mais engagé avec PERSONNE reste au simple socle de
    /// distance. C'est le cœur de la thèse : on perçoit « les plus PERTINENTS », pas « les
    /// plus PROCHES ». (Ici la logique pure ; le wire `engaged` est branché à l'étape 2.)
    #[test]
    fn transitivite_tire_un_pair_loin_mais_engage_avec_un_proche() {
        let a = PeerId::from_bytes([1u8; 32]); // PROCHE de moi (mon voisin / focus)
        let b = PeerId::from_bytes([2u8; 32]); // LOIN, mais A se déclare engagé avec lui
        let c = PeerId::from_bytes([3u8; 32]); // LOIN, engagé avec personne
        let ids = vec![a, b, c];
        let base = vec![
            relevance_weight(0.25),  // A : tout proche → pertinence ~1
            relevance_weight(1.0e6), // B : très loin → socle
            relevance_weight(1.0e6), // C : très loin → socle
        ];
        let engaged = vec![vec![b], vec![], vec![]]; // A est engagé avec B
        let eff = relevance_transitive(&ids, &base, &engaged);

        // B est TIRÉ nettement au-dessus de C (la pertinence suit le SOCIAL, pas la distance).
        assert!(eff[1] > eff[2] * 5.0, "B (engagé avec un proche) doit dépasser nettement C (juste loin)");
        // …mais jamais au-dessus de A qui le présente (fraction < 1 → hiérarchie préservée).
        assert!(eff[1] <= eff[0], "un tiers présenté ne dépasse pas son hôte");
        // C, engagé avec personne, reste EXACTEMENT à sa pertinence spatiale (aucun rehaussement).
        assert!((eff[2] - base[2]).abs() < 1.0e-6, "un pair non engagé n'est pas rehaussé");
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

    /// 8.3a — la grille de cellules : origine, frontières, et coordonnées NÉGATIVES (le piège
    /// du cast brut). `cell_of` doit utiliser `floor`, donc −0,1 tombe dans la cellule −1.
    #[test]
    fn cell_of_gere_origine_frontieres_et_negatifs() {
        assert_eq!(cell_of(0.0, 0.0), (0, 0));
        assert_eq!(cell_of(CELL_SIZE - 0.01, 0.0), (0, 0)); // juste avant la frontière
        assert_eq!(cell_of(CELL_SIZE, CELL_SIZE), (1, 1)); // sur la frontière → cellule suivante
        assert_eq!(cell_of(-0.1, -0.1), (-1, -1)); // négatif proche de 0 → −1 (pas 0 !)
        assert_eq!(cell_of(-CELL_SIZE, -CELL_SIZE), (-1, -1));
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
