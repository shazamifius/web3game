//! STATISTIQUES DE LIEN — la mesure PURE « ce que l'œil dirait », chiffrée.
//!
//! Extrait de l'agent (`metrics.rs`) : ici vit le CŒUR de mesure, sans aucune plomberie (ni réseau, ni
//! config, ni 3D). Tout part d'un fait : chaque état porte un `seq` MONOTONE (l'anti-rejeu). Du point de vue
//! d'un observateur, la suite des `(recv_ms, seq)` reçus suffit à TOUT déduire d'un lien :
//!   - **perte** = trous dans les `seq` (apparente vs réelle, cf. le bridage AoI) ;
//!   - **ré-ordonnancement** = un `seq` qui recule ;
//!   - **gigue (jitter)** = irrégularité des intervalles d'arrivée ;
//!   - **fraîcheur** = l'ÂGE du dernier état connu échantillonné dans le temps (la grandeur reine : ≤ 500 ms = jouable).
//!
//! Fonctions PURES et déterministes → testées en isolation (l'agent, lui, ne fait que les ALIMENTER).

/// Un événement d'ARRIVÉE d'état distant, vu par un observateur : QUAND on l'a reçu
/// (ms depuis le début de la mesure) et le `seq` de l'émetteur (monotone).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arrival {
    pub recv_ms: f64,
    pub seq: u64,
}

/// Les statistiques de LIEN d'une paire (observateur ← émetteur), indépendantes du moteur 3D.
/// C'est « ce que l'œil dirait », chiffré.
#[derive(Clone, Debug, Default)]
pub(crate) struct LinkStats {
    pub received: usize,   // nombre de paquets reçus
    pub expected: u64,     // attendus sur la plage de seq (max − min + 1) — RÉFÉRENCE PLEIN DÉBIT
    pub loss_pct: f64,     // perte APPARENTE : 1 − reçus / attendus (inclut le bridage AoI !)
    pub real_loss_pct: f64, // perte RÉELLE : relative à la cadence INFÉRÉE (hors bridage volontaire)
    pub cadence_step: u64, // pas de seq inféré entre deux envois reçus (1 = plein débit ; 10 ≈ bridé 2 Hz)
    pub reorder_pct: f64,  // fraction d'arrivées dont le seq recule
    pub jitter_ms: f64,    // écart absolu moyen des intervalles inter-arrivées
    pub fresh_p50_ms: f64, // FRAÎCHEUR (âge du dernier état connu) — médiane
    pub fresh_p95_ms: f64, // p95
    pub fresh_max_ms: f64, // pire cas
}

/// Le p-ième centile d'un tableau DÉJÀ TRIÉ (rang le plus proche). `p` ∈ [0, 100].
pub(crate) fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Les ÂGES de fraîcheur : on balaie le temps par pas de `tick_ms` (la cadence à laquelle
/// l'observateur « regarde »), et à chaque instant on note l'âge du DERNIER état reçu. Une
/// dent de scie : 0 juste après une arrivée, qui monte jusqu'à la suivante. `arrivals` doit
/// être trié par `recv_ms` croissant.
fn freshness_ages(arrivals: &[Arrival], tick_ms: f64) -> Vec<f64> {
    if arrivals.len() < 2 || tick_ms <= 0.0 {
        return vec![0.0];
    }
    let t0 = arrivals[0].recv_ms;
    let t_end = arrivals[arrivals.len() - 1].recv_ms;
    let mut ages = Vec::new();
    let mut idx = 0usize;
    let mut t = t0;
    while t <= t_end + 1e-9 {
        while idx + 1 < arrivals.len() && arrivals[idx + 1].recv_ms <= t {
            idx += 1;
        }
        ages.push(t - arrivals[idx].recv_ms);
        t += tick_ms;
    }
    ages
}

/// Calcule les stats de lien à partir des arrivées brutes et de la cadence d'observation.
/// `tick_ms` = tous les combien l'observateur « regarde » (typiquement le pas de rendu, ~16 ms).
pub(crate) fn link_stats(arrivals: &[Arrival], tick_ms: f64) -> LinkStats {
    if arrivals.is_empty() {
        return LinkStats::default();
    }
    // On travaille sur une copie triée par instant de réception (robuste si non trié).
    let mut by_time = arrivals.to_vec();
    by_time.sort_by(|a, b| a.recv_ms.partial_cmp(&b.recv_ms).unwrap_or(std::cmp::Ordering::Equal));

    let received = by_time.len();
    let min_seq = by_time.iter().map(|a| a.seq).min().unwrap();
    let max_seq = by_time.iter().map(|a| a.seq).max().unwrap();
    let expected = max_seq - min_seq + 1;
    let loss_pct = (1.0 - received as f64 / expected as f64).max(0.0);

    // VRAIE PERTE vs BRIDAGE VOLONTAIRE (l'enquête « inspecteur Eve », 28 juin).
    // L'émetteur incrémente son seq à plein débit (SEND_HZ, 20/s) pour TOUS, mais n'émet vers un
    // pair LOINTAIN qu'à CONSCIENCE_HZ (2/s) par l'AoI : ce pair voit seq 1,11,21… → `loss_pct`
    // (vs seq global) le compte « perdu » alors que RIEN ne l'est. On INFÈRE le pas de cadence
    // (médiane des sauts de seq consécutifs, robuste : une vraie perte fait un saut ~double) et on
    // mesure la perte RELATIVE à cette cadence : saut ≈ 1 pas = normal ; ≈ 2 pas = 1 envoi vraiment perdu.
    let mut by_seq: Vec<u64> = by_time.iter().map(|a| a.seq).collect();
    by_seq.sort_unstable();
    by_seq.dedup();
    let step_gaps: Vec<u64> = by_seq.windows(2).map(|w| w[1] - w[0]).collect();
    let (cadence_step, real_loss_pct) = if step_gaps.len() < 2 {
        (1, 0.0) // pas assez d'arrivées pour inférer une cadence → on ne prétend rien
    } else {
        let mut sorted_gaps = step_gaps.clone();
        sorted_gaps.sort_unstable();
        let base = sorted_gaps[sorted_gaps.len() / 2].max(1); // médiane (≥1) = le pas de cadence
        let mut slots = 0u64; // nb de créneaux d'émission attendus À CETTE CADENCE
        let mut missing = 0u64; // créneaux manquants = vraies pertes
        for &g in &step_gaps {
            let k = ((g as f64 / base as f64).round() as u64).max(1);
            slots += k;
            missing += k - 1;
        }
        let rl = if slots > 0 { missing as f64 / slots as f64 } else { 0.0 };
        (base, rl)
    };

    // Ré-ordonnancement : un seq qui recule par rapport à l'arrivée précédente (en temps).
    let reorders = by_time.windows(2).filter(|w| w[1].seq < w[0].seq).count();
    let reorder_pct = if received > 1 {
        reorders as f64 / (received - 1) as f64
    } else {
        0.0
    };

    // Gigue : écart absolu moyen des intervalles inter-arrivées autour de leur moyenne.
    let gaps: Vec<f64> = by_time.windows(2).map(|w| w[1].recv_ms - w[0].recv_ms).collect();
    let jitter_ms = if gaps.is_empty() {
        0.0
    } else {
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        gaps.iter().map(|g| (g - mean).abs()).sum::<f64>() / gaps.len() as f64
    };

    // Fraîcheur : distribution des âges.
    let mut ages = freshness_ages(&by_time, tick_ms);
    ages.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    LinkStats {
        received,
        expected,
        loss_pct,
        real_loss_pct,
        cadence_step,
        reorder_pct,
        jitter_ms,
        fresh_p50_ms: percentile(&ages, 50.0),
        fresh_p95_ms: percentile(&ages, 95.0),
        fresh_max_ms: ages.last().copied().unwrap_or(0.0),
    }
}

/// Le rapport d'une paire en JSON (à la main : on n'ajoute pas de dépendance pour ça).
/// C'est le format que les agents enverront au collecteur (un objet par paire observée).
pub(crate) fn report_json(observer: &str, target: &str, s: &LinkStats) -> String {
    format!(
        "{{\"observer\":\"{observer}\",\"target\":\"{target}\",\"received\":{},\"expected\":{},\
         \"loss_pct\":{:.2},\"real_loss_pct\":{:.2},\"cadence_step\":{},\"reorder_pct\":{:.2},\"jitter_ms\":{:.1},\
         \"fresh_p50_ms\":{:.1},\"fresh_p95_ms\":{:.1},\"fresh_max_ms\":{:.1}}}",
        s.received,
        s.expected,
        s.loss_pct * 100.0,
        s.real_loss_pct * 100.0,
        s.cadence_step,
        s.reorder_pct * 100.0,
        s.jitter_ms,
        s.fresh_p50_ms,
        s.fresh_p95_ms,
        s.fresh_max_ms,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La perte se lit EXACTEMENT dans les trous de seq, et un seq qui recule = ré-ordre.
    #[test]
    fn perte_et_reordre_se_lisent_dans_les_seq() {
        // seq [0,1,2,4,5] : le 3 manque → 1 perdu sur 6 attendus (16,7 %). Aucun recul.
        let a: Vec<Arrival> = [0u64, 1, 2, 4, 5]
            .iter()
            .enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 50.0, seq })
            .collect();
        let s = link_stats(&a, 50.0);
        assert_eq!(s.received, 5);
        assert_eq!(s.expected, 6);
        assert!((s.loss_pct - 1.0 / 6.0).abs() < 1e-9, "perte = 1/6");
        assert!(s.reorder_pct.abs() < 1e-9, "aucun ré-ordre");

        // seq [0,1,3,2,4] : le 2 arrive APRÈS le 3 → 1 recul sur 4 transitions (25 %).
        let b: Vec<Arrival> = [0u64, 1, 3, 2, 4]
            .iter()
            .enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 50.0, seq })
            .collect();
        let s = link_stats(&b, 50.0);
        assert!((s.reorder_pct - 0.25).abs() < 1e-9, "1 ré-ordre sur 4");
        assert!(s.loss_pct.abs() < 1e-9, "aucune perte ici (0..4 complet)");
    }

    /// ENQUÊTE « inspecteur Eve » (28 juin) : un pair BRIDÉ par l'AoI (2 Hz sur un seq global à
    /// 20 Hz) affiche une `loss_pct` énorme (FAUX : rien n'est perdu, l'émetteur n'a pas envoyé
    /// exprès) mais une `real_loss_pct` ~nulle. Et une VRAIE perte par-dessus se lit, elle, dans
    /// `real_loss_pct`. C'est la correction qui sépare « pas envoyé » de « envoyé puis perdu ».
    #[test]
    fn vraie_perte_distinguee_du_bridage_aoi() {
        // BRIDÉ SANS PERTE : seq 1,11,21,31,41 (cadence 10 = 2 Hz sur 20 Hz). Plein débit aurait
        // « attendu » 41 paquets → loss_pct ~88 % (faux positif). Cadence régulière → real_loss = 0.
        let bride: Vec<Arrival> = [1u64, 11, 21, 31, 41]
            .iter().enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 500.0, seq })
            .collect();
        let s = link_stats(&bride, 50.0);
        assert_eq!(s.cadence_step, 10, "cadence inférée = 10 (le bridage 2 Hz)");
        assert!(s.loss_pct > 0.85, "perte APPARENTE énorme (vs plein débit) : {}", s.loss_pct);
        assert!(s.real_loss_pct.abs() < 1e-9, "AUCUNE vraie perte (rien n'a été perdu) : {}", s.real_loss_pct);

        // BRIDÉ + 1 VRAIE PERTE : seq 1,11,31,41 (le 21 manque). Cadence toujours 10 (médiane) ;
        // le saut de 20 = 2 créneaux → 1 manquant sur 4 créneaux = 25 % de VRAIE perte.
        let perdu: Vec<Arrival> = [1u64, 11, 31, 41]
            .iter().enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 500.0, seq })
            .collect();
        let s = link_stats(&perdu, 50.0);
        assert_eq!(s.cadence_step, 10, "cadence inférée toujours 10");
        assert!((s.real_loss_pct - 0.25).abs() < 1e-9, "1 vraie perte sur 4 créneaux = 25 % : {}", s.real_loss_pct);
    }

    /// La FRAÎCHEUR grandit quand les paquets s'espacent : un lien à 1 paquet/seconde donne
    /// un âge bien pire qu'un lien à 20 Hz — c'est la grandeur « est-ce vivant ».
    #[test]
    fn fraicheur_pire_quand_les_paquets_s_espacent() {
        let serre: Vec<Arrival> = (0..50).map(|i| Arrival { recv_ms: i as f64 * 50.0, seq: i }).collect();
        let lache: Vec<Arrival> = (0..50).map(|i| Arrival { recv_ms: i as f64 * 1000.0, seq: i }).collect();
        let f_serre = link_stats(&serre, 16.0).fresh_p95_ms;
        let f_lache = link_stats(&lache, 16.0).fresh_p95_ms;
        assert!(f_lache > f_serre * 5.0, "un lien lâche est bien moins frais qu'un lien serré");
        assert!(f_serre < 500.0, "20 Hz reste sous le seuil de vivacité (≤ 500 ms)");
    }
}
