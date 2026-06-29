//! STOI — Short-Time Objective Intelligibility (Taal et al., 2011), version fait-main white-box.
//!
//! Prédit l'INTELLIGIBILITÉ de la parole (≈ % de mots compris), là où le SNR brut ne dit rien d'utile pour des
//! traitements perceptuels (débruitage, codec). Score `d ∈ [0,1]` (haut = compréhensible). 100 % formule, ZÉRO ML
//! (≠ POLQA/ViSQOL opaques) → conforme au README « sans boîte noire ».
//!
//! Recette (fidèle à l'article, adaptée à notre STFT) :
//!   1. STFT propre & dégradé, mêmes trames ;
//!   2. enveloppes par **15 bandes tiers-d'octave** (à partir de 150 Hz) = la résolution fréquentielle de la parole ;
//!   3. on RETIRE les trames silencieuses (énergie propre < −40 dB du max) — le silence ne porte pas d'info ;
//!   4. par bande, sur des **segments glissants de 30 trames**, on NORMALISE le dégradé à la norme du propre puis on
//!      CLIPPE (borne basse −15 dB : une grosse dégradation locale ne « sur-pénalise » pas) ;
//!   5. **corrélation** (Pearson) propre↔dégradé par bande/segment ; le STOI = la MOYENNE de toutes ces corrélations.

use super::fft::{hann, stft, Cplx};

const SEG: usize = 30; // longueur de segment (trames) ≈ 0,48 s à notre cadence

/// Bandes tiers-d'octave (plages de bins) à partir de 150 Hz, 15 bandes — la grille perceptuelle de la parole.
fn bandes_tiers_octave(n: usize, sr: f64) -> Vec<(usize, usize)> {
    let demi = n / 2;
    (0..15)
        .map(|j| {
            let fc = 150.0 * 2f64.powf(j as f64 / 3.0);
            let lo_f = fc / 2f64.powf(1.0 / 6.0);
            let hi_f = fc * 2f64.powf(1.0 / 6.0);
            let lo = ((lo_f * n as f64 / sr).floor() as usize).min(demi);
            let hi = ((hi_f * n as f64 / sr).ceil() as usize).min(demi + 1);
            (lo, hi.max(lo + 1))
        })
        .collect()
}

fn norme(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

/// Corrélation de Pearson entre deux segments de même longueur.
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let (mut cov, mut vx, mut vy) = (0.0, 0.0, 0.0);
    for (&a, &b) in x.iter().zip(y) {
        cov += (a - mx) * (b - my);
        vx += (a - mx) * (a - mx);
        vy += (b - my) * (b - my);
    }
    if vx < 1e-12 || vy < 1e-12 {
        return 0.0;
    }
    cov / (vx.sqrt() * vy.sqrt())
}

/// Calcule le STOI entre un signal PROPRE (référence) et un signal DÉGRADÉ (sortie de chaîne), même cadence.
pub fn stoi(propre: &[f32], degrade: &[f32], sr: f64, n: usize, hop: usize) -> f64 {
    let win = hann(n);
    let len = propre.len().min(degrade.len());
    let sp_c = stft(&propre[..len], n, hop, &win);
    let sp_d = stft(&degrade[..len], n, hop, &win);
    let frames = sp_c.len().min(sp_d.len());
    if frames == 0 {
        return 0.0;
    }
    let m = n / 2 + 1;

    // Retrait des trames silencieuses (sur l'énergie du PROPRE).
    let energie: Vec<f64> = (0..frames)
        .map(|t| (0..m).map(|k| sp_c[t][k].re * sp_c[t][k].re + sp_c[t][k].im * sp_c[t][k].im).sum::<f64>())
        .collect();
    let emax = energie.iter().copied().fold(0.0, f64::max);
    let seuil = emax * 1e-4; // -40 dB
    let gardes: Vec<usize> = (0..frames).filter(|&t| energie[t] > seuil).collect();
    if gardes.len() < SEG {
        return 0.0; // pas assez de parole pour un verdict
    }

    let bandes = bandes_tiers_octave(n, sr);
    // Enveloppes par bande sur les trames gardées.
    let enveloppes = |sp: &[Vec<Cplx>]| -> Vec<Vec<f64>> {
        bandes
            .iter()
            .map(|&(lo, hi)| {
                gardes
                    .iter()
                    .map(|&t| (lo..hi).map(|k| sp[t][k].re * sp[t][k].re + sp[t][k].im * sp[t][k].im).sum::<f64>().sqrt())
                    .collect()
            })
            .collect()
    };
    let xc = enveloppes(&sp_c);
    let yc = enveloppes(&sp_d);

    let plafond = 1.0 + 10f64.powf(15.0 / 20.0); // clipping β = -15 dB
    let (mut somme, mut cnt) = (0.0_f64, 0usize);
    let mtot = gardes.len();
    for j in 0..bandes.len() {
        for fin in SEG..=mtot {
            let x = &xc[j][fin - SEG..fin];
            let y = &yc[j][fin - SEG..fin];
            let nx = norme(x);
            if nx < 1e-12 {
                continue;
            }
            let ny = norme(y);
            let alpha = if ny > 1e-12 { nx / ny } else { 0.0 };
            let yb: Vec<f64> = (0..SEG).map(|i| (alpha * y[i]).min(plafond * x[i])).collect();
            somme += pearson(x, &yb);
            cnt += 1;
        }
    }
    if cnt == 0 {
        0.0
    } else {
        somme / cnt as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn voix(sr: f64, n_ech: usize) -> Vec<f32> {
        use std::f64::consts::PI;
        (0..n_ech)
            .map(|k| {
                let t = k as f64 / sr;
                let env = if (t * 3.0).fract() < 0.65 { 1.0 } else { 0.0 };
                let s: f64 = (1..=5).map(|h| (1.0 / h as f64) * (2.0 * PI * 165.0 * h as f64 * t).sin()).sum();
                (0.5 * env * s) as f32
            })
            .collect()
    }

    struct Rng(u64);
    impl Rng {
        fn f(&mut self) -> f32 {
            let mut x = self.0;
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            self.0 = x;
            ((x.wrapping_mul(0x2545F4914F6CDD1D) >> 40) as f64 / (1u64 << 24) as f64) as f32 * 2.0 - 1.0
        }
    }

    #[test]
    fn signal_identique_donne_stoi_proche_de_1() {
        let (sr, n, hop) = (16000.0, 512, 256);
        let v = voix(sr, 16000);
        let d = stoi(&v, &v, sr, n, hop);
        assert!(d > 0.99, "propre vs lui-même → STOI ≈ 1, obtenu {}", d);
    }

    #[test]
    fn invariant_a_lechelle() {
        // STOI normalise → un simple gain ne change pas l'intelligibilité.
        let (sr, n, hop) = (16000.0, 512, 256);
        let v = voix(sr, 16000);
        let demi: Vec<f32> = v.iter().map(|&x| 0.5 * x).collect();
        let d = stoi(&v, &demi, sr, n, hop);
        assert!(d > 0.99, "gain pur → STOI ≈ 1, obtenu {}", d);
    }

    #[test]
    fn du_bruit_pur_score_nettement_sous_le_propre() {
        // Du bruit décorrélé doit scorer NETTEMENT sous le propre (~1,0). La calibration absolue en % de mots
        // compris est un raffinement ; ici STOI sert en RELATIF (comparer des réglages de chaîne).
        let (sr, n, hop) = (16000.0, 512, 256);
        let v = voix(sr, 16000);
        let mut rng = Rng(7);
        let bruit: Vec<f32> = (0..16000).map(|_| rng.f()).collect();
        let d = stoi(&v, &bruit, sr, n, hop);
        assert!(d < 0.8, "bruit décorrélé → STOI nettement < propre, obtenu {}", d);
    }

    #[test]
    fn plus_de_bruit_baisse_le_stoi() {
        let (sr, n, hop) = (16000.0, 512, 256);
        let v = voix(sr, 16000);
        let mut rng = Rng(9);
        let bruit: Vec<f32> = (0..16000).map(|_| rng.f()).collect();
        let peu: Vec<f32> = v.iter().zip(&bruit).map(|(&s, &b)| s + 0.1 * b).collect();
        let beaucoup: Vec<f32> = v.iter().zip(&bruit).map(|(&s, &b)| s + 0.8 * b).collect();
        let d_peu = stoi(&v, &peu, sr, n, hop);
        let d_bcp = stoi(&v, &beaucoup, sr, n, hop);
        assert!(d_bcp < d_peu, "plus de bruit → STOI plus bas ({} vs {})", d_bcp, d_peu);
    }
}
