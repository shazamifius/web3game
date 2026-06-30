//! BANC AoI — PERTINENCE vs PROXIMITÉ (headless, déterministe). « Les 32 qui COMPTENT, pas les plus proches » (D29/D30).
//!
//! La sélection du focus actuelle (`NetLink::refresh_focus`) choisit par DISTANCE seule. La thèse OASIS (D29) est que
//! la PERTINENCE ≠ la proximité : les gens avec qui j'INTERAGIS doivent rester NETS (plein débit) même s'ils se sont
//! éloignés, là où la seule distance les relègue à la « conscience » (basse fidélité, ~2 Hz). La logique
//! `relevance_transitive` (un pair LOIN mais ENGAGÉ avec un de mes proches est rehaussé) est déjà prouvée en unité ;
//! ce banc la met à l'épreuve dans la SÉLECTION + l'ALLOCATION, et CHIFFRE le gain — comme `jeu vivant`/`jeu voix`.
//!
//! Scénario déterministe : moi à l'origine, 1 ami PROCHE engagé avec P PARTENAIRES placés LOIN (40 m), au milieu
//! d'une foule dense (anneau 9–11 m). On compare deux sélections de focus (proximité vs pertinence) et on mesure le
//! débit livré AUX PARTENAIRES, quand la foule grossit. Verdict attendu : sous proximité on PERD ses partenaires
//! (ils tombent en conscience) ; sous pertinence ils restent au focus (×10 de fidélité), quel que soit N.

use super::aoi::{allocate_tiers, dist2, relevance_transitive, relevance_weight, K_FOCUS, SEND_BUDGET_HZ};
use super::crypto::PeerId;

const ME: (f32, f32) = (0.0, 0.0);
const SEND_HZ: f32 = 20.0; // débit plein (focus) = la cadence d'émission

fn pid(i: usize) -> PeerId {
    let mut b = [0u8; 32];
    b[0] = (i & 0xff) as u8;
    b[1] = ((i >> 8) & 0xff) as u8;
    PeerId::from_bytes(b)
}

struct Scenario {
    ids: Vec<PeerId>,
    pos: Vec<(f32, f32)>,
    engaged: Vec<Vec<PeerId>>,
    partenaires: Vec<usize>, // indices des partenaires LOINTAINS (ceux avec qui j'interagis)
}

/// Foule de `n_foule` figurants (anneau 9–11 m) + 1 ami proche (1 m) engagé avec `p_part` partenaires LOIN (40 m).
fn scenario(n_foule: usize, p_part: usize) -> Scenario {
    let mut ids = Vec::new();
    let mut pos = Vec::new();

    // 0 : ami PROCHE (haute pertinence spatiale), c'est lui qui « présente » les partenaires lointains.
    ids.push(pid(0));
    pos.push((1.0, 0.0));

    // partenaires LOIN (40 m), regroupés — pertinence spatiale ≈ socle, mais ENGAGÉS avec l'ami proche.
    let mut partenaires = Vec::new();
    for j in 0..p_part {
        partenaires.push(ids.len());
        ids.push(pid(100 + j));
        let a = j as f32 * 1.3;
        pos.push((40.0 + a.cos() * 2.0, a.sin() * 2.0));
    }

    // foule dense : anneau 9–11 m, angles en « nombre d'or » → répartition déterministe et régulière.
    for c in 0..n_foule {
        ids.push(pid(1000 + c));
        let a = c as f32 * 2.399_963_2; // angle d'or (rad)
        let r = 9.0 + (c % 3) as f32; // 9, 10, 11 m
        pos.push((r * a.cos(), r * a.sin()));
    }

    // L'ami proche (idx 0) est ENGAGÉ avec tous les partenaires → ils héritent d'une part de SA pertinence.
    let mut engaged = vec![Vec::new(); ids.len()];
    engaged[0] = partenaires.iter().map(|&i| ids[i]).collect();

    Scenario { ids, pos, engaged, partenaires }
}

/// Les `k` indices de plus fort poids → masque `is_focus` (sélection simple top-K pour le banc ; en prod c'est la
/// version COLLANTE `refresh_focus`, hors-sujet ici : on isole l'effet PERTINENCE vs PROXIMITÉ).
fn top_k_focus(weights: &[f32], k: usize) -> Vec<bool> {
    let mut idx: Vec<usize> = (0..weights.len()).collect();
    idx.sort_by(|&a, &b| weights[b].partial_cmp(&weights[a]).unwrap_or(std::cmp::Ordering::Equal));
    let mut f = vec![false; weights.len()];
    for &i in idx.iter().take(k) {
        f[i] = true;
    }
    f
}

fn moyenne(it: impl Iterator<Item = f32>) -> f32 {
    let (mut s, mut n) = (0.0, 0u32);
    for x in it {
        s += x;
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        s / n as f32
    }
}

/// Mesure une foule de taille `n_foule` : (partenaires en focus prox, partenaires en focus pert, Hz partenaire prox,
/// Hz partenaire pert). Base = pertinence SPATIALE ; eff = avec la transitivité (pertinence sociale).
fn mesurer(n_foule: usize, p_part: usize) -> (usize, usize, f32, f32) {
    let sc = scenario(n_foule, p_part);
    let base: Vec<f32> = sc.pos.iter().map(|&p| relevance_weight(dist2(ME, p))).collect();
    let eff = relevance_transitive(&sc.ids, &base, &sc.engaged);

    let focus_prox = top_k_focus(&base, K_FOCUS);
    let focus_pert = top_k_focus(&eff, K_FOCUS);

    let rates_prox = allocate_tiers(&base, &focus_prox, SEND_BUDGET_HZ, SEND_HZ);
    let rates_pert = allocate_tiers(&eff, &focus_pert, SEND_BUDGET_HZ, SEND_HZ);

    let in_focus = |f: &[bool]| sc.partenaires.iter().filter(|&&i| f[i]).count();
    let hz = |r: &[f32]| moyenne(sc.partenaires.iter().map(|&i| r[i]));
    (in_focus(&focus_prox), in_focus(&focus_pert), hz(&rates_prox), hz(&rates_pert))
}

/// Point d'entrée `jeu aoi`.
pub fn run_aoi(_arg: &str) {
    let p_part = 3;
    println!("🧭  BANC AoI — PERTINENCE vs PROXIMITÉ (les « {} qui COMPTENT », pas les plus proches) — D29/D30", K_FOCUS);
    println!(
        "    moi à l'origine · {} partenaires LOIN (40 m) engagés via 1 ami proche (1 m) · foule dense (anneau 9–11 m)",
        p_part
    );
    println!("    K_FOCUS={} · budget {} Hz · plein débit {} Hz · conscience ≤ 2 Hz\n", K_FOCUS, SEND_BUDGET_HZ as u32, SEND_HZ as u32);
    println!(
        "   {:>9} │ {:^25} │ {:^25}",
        "foule N", "partenaires EN FOCUS", "débit livré au partenaire"
    );
    println!("   {:>9} │ {:>11} {:>13} │ {:>11} {:>13}", "", "proximité", "PERTINENCE", "proximité", "PERTINENCE");
    for &n in &[10usize, 30, 60, 120] {
        let (fp, fr, hp, hr) = mesurer(n, p_part);
        println!(
            "   {:>9} │ {:>8}/{} {:>10}/{} │ {:>9.1} Hz {:>10.1} Hz",
            n, fp, p_part, fr, p_part, hp, hr
        );
    }
    println!("\n📌 Lecture : sous PROXIMITÉ, mes partenaires d'interaction (partis à 40 m) tombent en CONSCIENCE (~2 Hz,");
    println!("   flous) — la foule proche leur vole le focus. Sous PERTINENCE, l'engagement de mon ami proche les");
    println!("   REHAUSSE → ils restent au plein débit (~20 Hz, nets), quel que soit N. « On perçoit les plus PERTINENTS,");
    println!("   pas les plus PROCHES. » Reste à câbler : porter `engaged` sur le wire d'état, puis brancher dans refresh_focus.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pertinence_garde_les_partenaires_au_focus_la_proximite_les_perd() {
        // D29/D30 — le cœur de la thèse, à l'échelle de la SÉLECTION + ALLOCATION (pas juste la logique pure).
        let p_part = 3;
        for n in [10usize, 60, 120] {
            let (fp, fr, hp, hr) = mesurer(n, p_part);
            assert_eq!(fr, p_part, "PERTINENCE : tous les partenaires au focus (N={}) — {}/{}", n, fr, p_part);
            assert_eq!(fp, 0, "PROXIMITÉ : les partenaires lointains ne sont PAS au focus (N={}) — {}", n, fp);
            assert!(hr > 10.0, "partenaire au plein débit sous pertinence (N={}) : {} Hz", n, hr);
            assert!(hr > hp * 3.0, "pertinence livre ≥3× le débit de la proximité aux partenaires (N={}) : {} vs {}", n, hr, hp);
        }
    }

    #[test]
    fn le_gain_tient_quand_la_foule_grossit() {
        // Invariant SCALE : le débit livré aux partenaires sous pertinence reste élevé même à grande foule.
        let (_, _, _, hr_petit) = mesurer(10, 3);
        let (_, _, _, hr_grand) = mesurer(200, 3);
        assert!(hr_grand > 10.0 && (hr_petit - hr_grand).abs() < 5.0, "partenaires nets stables : {} → {}", hr_petit, hr_grand);
    }
}
