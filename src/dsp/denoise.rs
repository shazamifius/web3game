//! « L'ÉTUDE DU MICRO » — débruitage white-box, mesuré automatiquement (idée utilisateur, 30 juin 2026).
//!
//! Contre-modèle de Krisp (boîte noire ML qui rase tout) : on ESTIME le bruit, on n'enlève QUE lui, par des maths
//! transparentes. Pipeline (sur le socle `fft.rs`) :
//!   1. **estimer le bruit** par bin = statistique de minimum (le p-ième percentile de l'amplitude sur les trames :
//!      pendant les silences/gaps de parole, l'amplitude ≈ le bruit stationnaire — ventilo, souffle) ;
//!   2. **soustraction spectrale** : gain par bin `G = max(β, 1 − α·bruit/|X|)` → on rabaisse les bins dominés par
//!      le bruit, on laisse intacts ceux dominés par la voix ; on garde la phase.
//!
//! Et SURTOUT : on ne juge pas à l'oreille. Le banc `jeu micro` passe des bancs de son (voix + un type de bruit) et
//! CHIFFRE, en matrice : réduction de bruit (dB), VOIX préservée (dB), et le **rayon d'audibilité du bruit vs la
//! voix** (réponse directe à « rendre les bruits répétitifs inaudibles À DISTANCE »). Beaucoup de tests d'un coup,
//! reproductibles. La décomposition est rigoureuse : on applique le MÊME gain `G` (calculé sur le mélange) à la voix
//! SEULE et au bruit SEUL → on sépare proprement « ce qu'on a enlevé du bruit » de « ce qu'on a abîmé de la voix ».

use super::fft::{hann, stft, Cplx};

fn n_bins_uniques(n: usize) -> usize {
    n / 2 + 1
}

/// Estime l'amplitude du bruit par bin : le `p`-ième percentile de `|X[t,k]|` sur toutes les trames `t`.
/// (La voix est intermittente → le bas percentile capture le plancher de bruit stationnaire.)
fn estimer_bruit(spectres: &[Vec<Cplx>], n: usize, p: f64) -> Vec<f64> {
    let m = n_bins_uniques(n);
    let mut profil = vec![0.0_f64; m];
    for (k, pk) in profil.iter_mut().enumerate() {
        let mut amps: Vec<f64> = spectres.iter().map(|s| (s[k].re * s[k].re + s[k].im * s[k].im).sqrt()).collect();
        amps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p * (amps.len().saturating_sub(1)) as f64).round() as usize).min(amps.len().saturating_sub(1));
        *pk = amps.get(idx).copied().unwrap_or(0.0);
    }
    profil
}

/// Gain de soustraction spectrale pour un bin : `max(β, 1 − α·bruit/|x|)`.
fn gain(amp_x: f64, bruit: f64, alpha: f64, beta: f64) -> f64 {
    if amp_x <= 1e-12 {
        return beta;
    }
    (1.0 - alpha * bruit / amp_x).max(beta)
}

/// Résultat du débruitage mesuré sur un mélange voix+bruit.
pub struct MesureDenoise {
    pub reduction_bruit_db: f64, // énergie bruit AVANT / APRÈS (haut = bien retiré)
    pub voix_preservee_db: f64,  // énergie voix / énergie de la distorsion infligée (haut = bien préservée)
    pub bulle_sans_pct: f64,     // rayon d'audibilité du bruit / rayon de la voix, AVANT (% — petit = confiné)
    pub bulle_avec_pct: f64,     // idem APRÈS débruitage
}

/// Passe `voix` et `bruit` (mêmes longueurs) dans le débruitage et chiffre l'effet — décomposition rigoureuse :
/// le gain calculé sur le MÉLANGE est appliqué séparément à la voix et au bruit.
pub fn mesurer(voix: &[f32], bruit: &[f32], n: usize, hop: usize, alpha: f64, beta: f64) -> MesureDenoise {
    let win = hann(n);
    let m = n_bins_uniques(n);
    let melange: Vec<f32> = voix.iter().zip(bruit).map(|(&v, &b)| v + b).collect();

    let sp_x = stft(&melange, n, hop, &win);
    let sp_s = stft(voix, n, hop, &win);
    let sp_n = stft(bruit, n, hop, &win);
    let profil = estimer_bruit(&sp_x, n, 0.10); // 10e percentile = plancher de bruit

    let (mut e_n, mut e_n_res, mut e_s, mut e_dist) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for ((fx, fs), fnn) in sp_x.iter().zip(&sp_s).zip(&sp_n) {
        for k in 0..m {
            let amp_x = (fx[k].re * fx[k].re + fx[k].im * fx[k].im).sqrt();
            let g = gain(amp_x, profil[k], alpha, beta);
            // bruit : avant (|N|²) vs après (|G·N|²)
            let en = fnn[k].re * fnn[k].re + fnn[k].im * fnn[k].im;
            e_n += en;
            e_n_res += g * g * en;
            // voix : énergie vs distorsion infligée par le gain (|(G−1)·S|²)
            let es = fs[k].re * fs[k].re + fs[k].im * fs[k].im;
            e_s += es;
            e_dist += (g - 1.0) * (g - 1.0) * es;
        }
    }

    let reduction_bruit_db = 10.0 * (e_n / e_n_res.max(1e-30)).log10();
    let voix_preservee_db = 10.0 * (e_s / e_dist.max(1e-30)).log10();
    // Rayon d'audibilité ∝ sqrt(énergie) (amplitude ∝ 1/d) ; en % du rayon de la voix (le seuil se simplifie).
    let bulle_sans_pct = 100.0 * (e_n / e_s.max(1e-30)).sqrt();
    let bulle_avec_pct = 100.0 * (e_n_res / e_s.max(1e-30)).sqrt();
    MesureDenoise { reduction_bruit_db, voix_preservee_db, bulle_sans_pct, bulle_avec_pct }
}

// ----------------------------------------------------------------------------
// BANC `jeu micro` — bancs de son × types de bruit, débruitage MESURÉ (pas à l'oreille)
// ----------------------------------------------------------------------------

struct Rng(u64);
impl Rng {
    fn next_f32(&mut self) -> f32 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f64 / (1u64 << 24) as f64) as f32 * 2.0 - 1.0
    }
}

/// Voix SYNTHÉTIQUE intermittente (syllabes on/off) — les gaps laissent voir le bruit (estimation réaliste).
fn voix_syllabique(sr: f64, n_ech: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    (0..n_ech)
        .map(|k| {
            let t = k as f64 / sr;
            // enveloppe syllabique : ~3 syllabes/s, 65 % allumé
            let cycle = (t * 3.0).fract();
            let env = if cycle < 0.65 { 1.0 } else { 0.0 };
            let f = 165.0;
            let s: f64 = (1..=5).map(|h| (1.0 / h as f64) * (2.0 * PI * f * h as f64 * t).sin()).sum();
            (0.5 * env * s) as f32
        })
        .collect()
}

/// Les types de bruit du banc (le « micro qui donne quoi »).
fn bruits(sr: f64, n_ech: usize) -> Vec<(&'static str, Vec<f32>)> {
    use std::f64::consts::PI;
    let t = |k: usize| k as f64 / sr;

    // (1) ventilo PC : tonal stationnaire (120 Hz + harmoniques) — le cas que la soustraction spectrale ADORE
    let ventilo: Vec<f32> = (0..n_ech)
        .map(|k| (0.15 * ((2.0 * PI * 120.0 * t(k)).sin() + 0.5 * (2.0 * PI * 240.0 * t(k)).sin())) as f32)
        .collect();

    // (2) souffle large bande (hiss) stationnaire, faible niveau
    let mut rng = Rng(0xBADCAFE);
    let souffle: Vec<f32> = (0..n_ech).map(|_| 0.08 * rng.next_f32()).collect();

    // (3) clics RÉPÉTÉS (transitoires brefs ~2/s) — non stationnaire : la stat de minimum les voit MAL
    let clics: Vec<f32> = (0..n_ech)
        .map(|k| {
            let periode = (sr * 0.5) as usize;
            let phase = k % periode.max(1);
            let env = (-(phase as f64) / (sr * 0.004)).exp();
            (0.5 * env * (2.0 * PI * 2000.0 * t(k)).sin()) as f32
        })
        .collect();

    vec![("ventilo tonal", ventilo), ("souffle large bande", souffle), ("clics répétés", clics)]
}

/// Point d'entrée `jeu micro`.
pub fn run_micro(_arg: &str) {
    let (sr, n, hop, dur) = (16000.0, 512, 256, 2.0);
    let n_ech = (sr * dur) as usize;
    let (alpha, beta) = (1.5, 0.1); // sur-soustraction modérée, plancher anti « bruit musical »

    let voix = voix_syllabique(sr, n_ech);
    println!("🎤  BANC « ÉTUDE DU MICRO » — débruitage white-box MESURÉ (jamais à l'oreille)");
    println!(
        "    {} Hz · STFT {} · soustraction spectrale (α={}, β={}) · profil bruit = 10e percentile par bin\n",
        sr as u32, n, alpha, beta
    );
    println!(
        "   {:<22} {:>14} {:>16} {:>22}",
        "type de bruit", "réduction dB", "voix préservée dB", "bulle bruit (% rayon voix)"
    );
    for (nom, bruit) in bruits(sr, n_ech) {
        let m = mesurer(&voix, &bruit, n, hop, alpha, beta);
        println!(
            "   {:<22} {:>14.1} {:>16.1} {:>10.0} → {:>4.0}",
            nom, m.reduction_bruit_db, m.voix_preservee_db, m.bulle_sans_pct, m.bulle_avec_pct
        );
    }
    println!("\n📌 Lecture : la soustraction spectrale (white-box) écrase les bruits STATIONNAIRES (ventilo, souffle)");
    println!("   → leur « bulle » d'audibilité rétrécit fortement vs la voix, en la préservant. Les bruits RÉPÉTÉS");
    println!("   transitoires (clics) résistent (non stationnaires) → ils appellent un outil dédié (détection de");
    println!("   répétition), prochaine brique. Tout chiffré, beaucoup de cas d'un coup. Détail : prive/PLAN_TEST_VOIX.md §1.8");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_ventilo_tonal_est_fortement_reduit_en_preservant_la_voix() {
        let (sr, n, hop) = (16000.0, 512, 256);
        let n_ech = (sr * 2.0) as usize;
        let voix = voix_syllabique(sr, n_ech);
        let bruit = &bruits(sr, n_ech)[0].1; // ventilo tonal
        let m = mesurer(&voix, bruit, n, hop, 1.5, 0.1);
        assert!(m.reduction_bruit_db > 3.0, "le ventilo doit être nettement réduit : {} dB", m.reduction_bruit_db);
        assert!(m.voix_preservee_db > 6.0, "la voix doit rester préservée : {} dB", m.voix_preservee_db);
        assert!(m.bulle_avec_pct < m.bulle_sans_pct, "la bulle de bruit doit rétrécir");
    }

    #[test]
    fn la_bulle_de_bruit_retrecit_avec_le_debruitage() {
        let (sr, n, hop) = (16000.0, 512, 256);
        let n_ech = (sr * 2.0) as usize;
        let voix = voix_syllabique(sr, n_ech);
        let bruit = &bruits(sr, n_ech)[1].1; // souffle large bande
        let m = mesurer(&voix, bruit, n, hop, 1.5, 0.1);
        assert!(m.bulle_avec_pct < m.bulle_sans_pct, "le débruitage doit confiner le souffle plus près");
    }

    #[test]
    fn le_stationnaire_est_mieux_reduit_que_le_transitoire() {
        // Vérité honnête : la stat de minimum capture le bruit STATIONNAIRE (ventilo) bien mieux que les clics
        // transitoires non stationnaires → réduction du ventilo > réduction des clics.
        let (sr, n, hop) = (16000.0, 512, 256);
        let n_ech = (sr * 2.0) as usize;
        let voix = voix_syllabique(sr, n_ech);
        let bs = bruits(sr, n_ech);
        let ventilo = mesurer(&voix, &bs[0].1, n, hop, 1.5, 0.1);
        let clics = mesurer(&voix, &bs[2].1, n, hop, 1.5, 0.1);
        assert!(
            ventilo.reduction_bruit_db > clics.reduction_bruit_db,
            "ventilo {} doit être mieux réduit que clics {}",
            ventilo.reduction_bruit_db,
            clics.reduction_bruit_db
        );
    }
}
