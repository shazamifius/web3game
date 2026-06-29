//! PHASE 1 (D36) — banc DÉTERMINISTE de la redondance sous perte CONNUE.
//!
//! Le test live 4G (29 juin) a échoué méthodologiquement (sessions séquentielles sur un lien fluctuant)
//! et a suggéré que la redondance EMPIRE un lien saturé. Le banc UDP en-process (`relay-loss`) est trop
//! flaky (perçage qui churn). Ici on ISOLE le mécanisme : une simulation en mémoire, à graine fixe
//! (chiffre reproductible — « une preuve = un chiffre »), qui modélise FIDÈLEMENT notre redondance réelle.
//!
//! NOTRE mécanisme (`KIND_STATE_BUNDLE`) : à chaque tick on émet UN paquet portant les `k` DERNIERS états.
//! Donc l'état `i` voyage dans les bundles `i, i+1, …, i+k-1` → il est reçu si AU MOINS un de ces bundles
//! arrive. Le coût de la redondance n'est PAS plus de paquets, mais des paquets `k×` plus GROS (en octets)
//! → c'est ce qui sature un lien à débit borné. On mesure la perte RÉELLE = états jamais reçus.
//!
//! Trois régimes :
//!  • ALÉATOIRE (indépendant) : chaque bundle perdu avec proba `p`, débit illimité → prédiction `p^k`.
//!  • RAFALE (Gilbert-Elliott) : pertes CORRÉLÉES (le lien part en vrille par paquets) → gain < `p^k`.
//!  • CONGESTION (débit plafonné, 0 perte aléatoire) : les bundles `k×` plus gros débordent le budget
//!    d'octets → la redondance n'aide plus, voire NUIT (le cas du 4G de ce soir).

/// RNG déterministe (xorshift64*), std-only — pas de dépendance.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1) // jamais 0
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    /// Flottant dans [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Le modèle de perte appliqué à un bundle ADMIS par le débit.
#[derive(Clone, Copy)]
pub enum Loss {
    /// Perte indépendante de proba `p` par bundle (le cas « idéal » pour la redondance).
    Random(f64),
    /// Gilbert-Elliott : deux états (bon/mauvais). En « mauvais », tout tombe. Pertes EN RAFALE.
    /// `g2b`/`b2g` = probas de transition ; la fraction moyenne de perte ≈ g2b/(g2b+b2g).
    Burst { g2b: f64, b2g: f64 },
    /// Aucune perte aléatoire (pour isoler la CONGESTION).
    None,
}

/// Paramètres d'un run.
pub struct Params {
    pub n: usize,            // nombre d'états émis
    pub rate_hz: f64,        // cadence d'émission (paquets/s)
    pub state_bytes: f64,    // taille d'UN état (octets) — réel : 182
    pub k: usize,            // redondance : états par bundle
    pub bandwidth_bps: Option<f64>, // débit du goulot (octets/s) ; None = illimité
    pub loss: Loss,
    pub seed: u64,
}

/// Simule un run et renvoie la PERTE RÉELLE (fraction d'états jamais reçus, dans [0,1]).
pub fn simulate(p: &Params) -> f64 {
    let dt = 1.0 / p.rate_hz;
    let mut rng = Rng::new(p.seed);
    let mut received = vec![false; p.n];

    // Seau à jetons (octets) pour le débit : capacité = ~0,25 s de débit (petite rafale tolérée).
    let cap = p.bandwidth_bps.map(|b| b * 0.25).unwrap_or(f64::INFINITY);
    let mut tokens = cap;
    // État Gilbert-Elliott (false = bon, true = mauvais).
    let mut bad = false;

    for t in 0..p.n {
        // Le bundle au tick t porte les états {t-k+1 .. t} (clampé à 0).
        let first = t.saturating_sub(p.k - 1);
        let n_in_bundle = t - first + 1;
        let size = n_in_bundle as f64 * p.state_bytes;

        // 1) Débit : recharge puis admission (sinon = drop de CONGESTION).
        let admitted = match p.bandwidth_bps {
            None => true,
            Some(b) => {
                tokens = (tokens + b * dt).min(cap);
                if tokens >= size {
                    tokens -= size;
                    true
                } else {
                    false // le bundle ne tient pas dans le budget → congestion
                }
            }
        };
        if !admitted {
            continue;
        }

        // 2) Perte aléatoire/rafale sur le bundle admis.
        let lost = match p.loss {
            Loss::None => false,
            Loss::Random(pr) => rng.next_f64() < pr,
            Loss::Burst { g2b, b2g } => {
                // Transition d'état AVANT la décision (le mauvais état « colle »).
                bad = if bad { rng.next_f64() >= b2g } else { rng.next_f64() < g2b };
                bad // en mauvais état, le bundle tombe
            }
        };
        if lost {
            continue;
        }

        // 3) Bundle délivré → tous ses états sont reçus.
        received[first..=t].fill(true);
    }

    let got = received.iter().filter(|&&r| r).count();
    1.0 - got as f64 / p.n as f64
}

/// Affiche le tableau Phase 1 : régime × redondance → perte réelle. `cargo run -- phase1`.
pub fn run_phase1() {
    let (n, rate, sbytes, seed) = (4000usize, 20.0, 182.0, 0xC0FFEE_u64);
    println!("=== PHASE 1 — REDONDANCE SOUS PERTE CONNUE (déterministe, graine {seed:#x}) ===");
    println!("    {n} états @ {rate:.0} Hz, état = {sbytes:.0} o ; bundle = nos K derniers états (KIND_STATE_BUNDLE)\n");

    // Débits du goulot. Congestion LÉGÈRE = pile le besoin de K=1 ; SÉVÈRE = 60 % du besoin de K=1.
    let bw_leger = rate * sbytes; // octets/s
    let bw_severe = rate * sbytes * 0.6;

    let regimes: [(&str, Loss, Option<f64>); 4] = [
        ("ALÉATOIRE p=0.60 (débit illimité)", Loss::Random(0.60), None),
        ("RAFALE ~0.60 (Gilbert-Elliott, débit illimité)", Loss::Burst { g2b: 0.15, b2g: 0.10 }, None),
        ("CONGESTION LÉGÈRE (0 perte alea, débit = besoin K=1)", Loss::None, Some(bw_leger)),
        ("CONGESTION SÉVÈRE + RAFALE (≈ 4G saturé du 29 juin)", Loss::Burst { g2b: 0.15, b2g: 0.10 }, Some(bw_severe)),
    ];

    println!("  {:<54}{:>9}{:>9}{:>9}", "régime", "K=1", "K=2", "K=3");
    for (name, loss, bw) in regimes {
        let mut cells = [0.0f64; 3];
        for (i, k) in [1usize, 2, 3].into_iter().enumerate() {
            cells[i] = simulate(&Params { n, rate_hz: rate, state_bytes: sbytes, k, bandwidth_bps: bw, loss, seed });
        }
        println!("  {:<54}{:>8.1}%{:>8.1}%{:>8.1}%", name, cells[0] * 100.0, cells[1] * 100.0, cells[2] * 100.0);
    }
    println!("\n  Lecture : ALÉATOIRE → la redondance DIVISE la perte (~p^k). RAFALE → gain bien moindre (les copies");
    println!("  consécutives tombent dans la même rafale). CONGESTION LÉGÈRE → neutre (le chevauchement de nos bundles");
    println!("  rattrape les trous espacés). CONGESTION SÉVÈRE + RAFALE → redondance INUTILE : ~0 gain pour 2-3× les");
    println!("  octets = gâchis net (et en vrai ces octets aggravent la congestion — non modélisé ici). = le cas 4G.");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(k: usize, loss: Loss, bw: Option<f64>) -> f64 {
        simulate(&Params { n: 4000, rate_hz: 20.0, state_bytes: 182.0, k, bandwidth_bps: bw, loss, seed: 0xC0FFEE })
    }

    /// ALÉATOIRE : la redondance DIVISE la perte vers ~p^k (le cas idéal). Critère pré-enregistré.
    #[test]
    fn aleatoire_la_redondance_divise_vers_p_puissance_k() {
        let (r1, r2, r3) = (run(1, Loss::Random(0.60), None), run(2, Loss::Random(0.60), None), run(3, Loss::Random(0.60), None));
        assert!((r1 - 0.60).abs() < 0.05, "K=1 ≈ p (60%), obtenu {:.3}", r1);
        assert!((r2 - 0.36).abs() < 0.06, "K=2 ≈ p^2 (36%), obtenu {:.3}", r2);
        assert!((r3 - 0.216).abs() < 0.06, "K=3 ≈ p^3 (21,6%), obtenu {:.3}", r3);
        assert!(r3 < r2 && r2 < r1, "la perte DÉCROÎT avec K : {r1:.3} > {r2:.3} > {r3:.3}");
    }

    /// RAFALE : pertes corrélées → la redondance aide BEAUCOUP MOINS (les copies consécutives tombent
    /// dans la même rafale). Le gain relatif K=1→K=2 est nettement pire qu'en aléatoire.
    #[test]
    fn rafale_le_gain_de_redondance_s_effondre() {
        let burst = Loss::Burst { g2b: 0.15, b2g: 0.10 };
        let (b1, b2) = (run(1, burst, None), run(2, burst, None));
        let (r1, r2) = (run(1, Loss::Random(0.60), None), run(2, Loss::Random(0.60), None));
        let gain_rafale = (b1 - b2) / b1; // réduction relative
        let gain_alea = (r1 - r2) / r1;
        assert!(b1 > 0.45, "rafale K=1 ≈ fraction mauvaise (~60%), obtenu {:.3}", b1);
        assert!(gain_rafale < gain_alea * 0.6, "le gain en rafale ({gain_rafale:.2}) doit s'effondrer vs aléatoire ({gain_alea:.2})");
    }

    /// CONGESTION LÉGÈRE (débit = besoin de K=1, 0 perte aléatoire) : la redondance est NEUTRE — le
    /// chevauchement de nos bundles rattrape les trous régulièrement espacés du seau à jetons.
    #[test]
    fn congestion_legere_la_redondance_est_neutre() {
        let bw = Some(20.0 * 182.0);
        let (c1, c2) = (run(1, Loss::None, bw), run(2, Loss::None, bw));
        assert!(c1 < 0.02 && c2 < 0.02, "trous espacés rattrapés par le chevauchement : {c1:.3}, {c2:.3}");
    }

    /// CONGESTION SÉVÈRE + RAFALE (≈ 4G saturé du 29 juin) : la redondance devient INUTILE — ~0 gain
    /// alors qu'elle coûte 2-3× les octets (gâchis net ; en vrai ces octets aggravent encore). À comparer
    /// avec l'aléatoire où la redondance gagne franchement → c'est l'explication mécaniste du live négatif.
    #[test]
    fn congestion_severe_la_redondance_devient_inutile() {
        let bw = Some(20.0 * 182.0 * 0.6); // débit = 60% du besoin de K=1
        let burst = Loss::Burst { g2b: 0.15, b2g: 0.10 };
        let (c1, c2, c3) = (run(1, burst, bw), run(2, burst, bw), run(3, burst, bw));
        assert!(c2 > 0.6, "lien saturé+bursty reste catastrophique : K=2 {c2:.3}");
        assert!((c1 - c2).abs() < 0.05 && (c1 - c3).abs() < 0.05, "redondance ~0 gain ici : {c1:.3} -> {c2:.3} -> {c3:.3}");
        // Contraste : en aléatoire, la même redondance gagne franchement.
        let (r1, r2) = (run(1, Loss::Random(0.60), None), run(2, Loss::Random(0.60), None));
        assert!((r1 - r2) > 0.20, "en aléatoire la redondance DOIT gagner (contraste) : {r1:.3} -> {r2:.3}");
    }
}
