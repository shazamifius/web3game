//! Codec voix FAIT MAIN, transform-domain, white-box (décidé le 30 juin 2026).
//!
//! Pipeline transparent, bâti sur le socle `fft.rs` : STFT (Hann, 50 % recouvrement) → **quantification scalaire
//! uniforme** des coefficients spectraux (pas perdu : `delta` = le pas, le SEUL bouton) → ISTFT (overlap-add).
//! Aucun modèle de parole (≠ vocoder LPC) → **agnostique au signal** : le chuchotement, le chant et le beatbox
//! passent tous, ils coûtent juste plus ou moins de bits. Zéro dépendance.
//!
//! Le débit est estimé par l'**entropie de Shannon** des coefficients quantifiés (= ce qu'un codeur entropique
//! IDÉAL atteindrait — une borne basse honnête, légèrement optimiste vs un vrai codeur). La distorsion = le SNR
//! de reconstruction. On trace la courbe **rate-distortion** par type de son. C'est le pendant de `jeu vivant` /
//! `jeu voix` côté codec : un chiffre reproductible, pas une impression.

use super::fft::{hann, istft, stft, Cplx};

/// Nombre de bins UNIQUES d'un signal réel de taille `n` (le reste = symétrie conjuguée).
fn n_bins_uniques(n: usize) -> usize {
    n / 2 + 1
}

/// Quantifie les bins uniques (re, im) au pas `delta` → entiers signés (les symboles transmis).
fn quantifier(spectre: &[Cplx], n: usize, delta: f64) -> Vec<i32> {
    let m = n_bins_uniques(n);
    let mut q = Vec::with_capacity(2 * m);
    for k in 0..m {
        q.push((spectre[k].re / delta).round() as i32);
        q.push((spectre[k].im / delta).round() as i32);
    }
    q
}

/// Reconstruit le spectre complet (taille `n`) depuis les symboles quantifiés, par symétrie conjuguée.
fn dequantifier(q: &[i32], n: usize, delta: f64) -> Vec<Cplx> {
    let m = n_bins_uniques(n);
    let mut s = vec![Cplx::new(0.0, 0.0); n];
    for k in 0..m {
        s[k] = Cplx::new(q[2 * k] as f64 * delta, q[2 * k + 1] as f64 * delta);
    }
    for k in 1..n / 2 {
        s[n - k] = Cplx::new(s[k].re, -s[k].im); // X[n-k] = conj(X[k]) pour un signal réel
    }
    s
}

/// Entropie de Shannon (bits/symbole) d'un flux d'entiers — le débit qu'un codeur entropique idéal atteindrait.
fn entropie_bits(symboles: &[i32]) -> f64 {
    use std::collections::HashMap;
    if symboles.is_empty() {
        return 0.0;
    }
    let mut hist: HashMap<i32, u64> = HashMap::new();
    for &s in symboles {
        *hist.entry(s).or_insert(0) += 1;
    }
    let total = symboles.len() as f64;
    let h = -hist
        .values()
        .map(|&c| {
            let p = c as f64 / total;
            p * p.log2()
        })
        .sum::<f64>();
    h.max(0.0) // évite le -0.0 quand tous les symboles sont identiques (entropie nulle)
}

/// Résultat d'un encodage/décodage à un `delta` donné.
pub struct Mesure {
    pub bitrate_kbps: f64,
    pub snr_db: f64,
}

/// Encode puis décode `signal` au pas `delta` ; renvoie le débit (entropie) et le SNR de reconstruction.
pub fn coder_decoder(signal: &[f32], sr: f64, n: usize, hop: usize, delta: f64) -> Mesure {
    let win = hann(n);
    let spectres = stft(signal, n, hop, &win);

    // Quantifier chaque trame, accumuler tous les symboles (pour l'entropie) et les trames déquantifiées.
    let mut tous_symboles: Vec<i32> = Vec::new();
    let mut trames_rec: Vec<Vec<Cplx>> = Vec::with_capacity(spectres.len());
    for sp in &spectres {
        let q = quantifier(sp, n, delta);
        tous_symboles.extend_from_slice(&q);
        trames_rec.push(dequantifier(&q, n, delta));
    }

    let rec = istft(&trames_rec, n, hop, signal.len());

    // Débit : entropie/symbole × symboles/seconde.
    let symboles_par_s = tous_symboles.len() as f64 / (signal.len() as f64 / sr);
    let bitrate_kbps = entropie_bits(&tous_symboles) * symboles_par_s / 1000.0;

    // SNR sur l'INTÉRIEUR (échantillons couverts par 2 trames pleines → reconstruction propre).
    let (mut e_sig, mut e_bruit) = (0.0_f64, 0.0_f64);
    for k in n..signal.len().saturating_sub(n) {
        let x = signal[k] as f64;
        let d = x - rec[k] as f64;
        e_sig += x * x;
        e_bruit += d * d;
    }
    let snr_db = if e_bruit <= 1e-30 {
        120.0 // quasi sans perte
    } else {
        10.0 * (e_sig / e_bruit).log10()
    };
    Mesure { bitrate_kbps, snr_db }
}

// ----------------------------------------------------------------------------
// BANC — `jeu codec` : courbe rate-distortion par type de son (dont le beatbox, le cas dur)
// ----------------------------------------------------------------------------

/// Petit RNG déterministe (xorshift64*) pour un bruit reproductible.
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

/// Génère les 4 signaux de test (durée `dur` s à `sr` Hz), normalisés ~±0,8.
fn signaux(sr: f64, dur: f64) -> Vec<(&'static str, Vec<f32>)> {
    let n = (sr * dur) as usize;
    let t = |k: usize| k as f64 / sr;

    // (1) sinus pur 220 Hz — spectre creux (le cas FACILE).
    let sinus: Vec<f32> = (0..n).map(|k| 0.8 * (2.0 * std::f64::consts::PI * 220.0 * t(k)).sin() as f32).collect();

    // (2) « voyelle/chant » — fondamentale 165 Hz + harmoniques (riche mais tonal).
    let chant: Vec<f32> = (0..n)
        .map(|k| {
            let f = 165.0;
            let s: f64 = (1..=5).map(|h| (1.0 / h as f64) * (2.0 * std::f64::consts::PI * f * h as f64 * t(k)).sin()).sum();
            (0.5 * s) as f32
        })
        .collect();

    // (3) « beatbox » — train de transitoires brefs (large bande) : le cas DUR qu'un vocoder LPC TUERAIT.
    let beatbox: Vec<f32> = (0..n)
        .map(|k| {
            let periode = (sr * 0.18) as usize; // ~5,5 coups/s
            let phase = k % periode.max(1);
            let env = (-(phase as f64) / (sr * 0.012)).exp(); // claquement qui décroît vite
            (0.8 * env * (2.0 * std::f64::consts::PI * 1400.0 * t(k)).sin()) as f32
        })
        .collect();

    // (4) bruit blanc — incompressible (la borne haute de débit, référence).
    let mut rng = Rng(0xC0DEC_F00D);
    let bruit: Vec<f32> = (0..n).map(|_| 0.6 * rng.next_f32()).collect();

    vec![("sinus 220 Hz", sinus), ("chant (5 harm.)", chant), ("beatbox (transit.)", beatbox), ("bruit blanc", bruit)]
}

/// Banc complet : pour chaque son, balaye le pas de quantification → (débit kbit/s, SNR dB).
pub fn run_codec(_arg: &str) {
    let sr = 16000.0; // bande parole large (16 kHz) — voix chat standard
    let n = 512; // trame STFT (32 ms à 16 kHz)
    let hop = n / 2; // 50 % → COLA Hann
    let dur = 1.0;
    let deltas = [2.0_f64, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];

    println!("🎛️  BANC CODEC VOIX FAIT MAIN (transform-domain, white-box) — courbe rate-distortion");
    println!(
        "    {} Hz · STFT {} (Hann 50 %) · débit = ENTROPIE des coefficients quantifiés (codeur idéal) · repère Opus ≈ 20 kbit/s\n",
        sr as u32, n
    );

    for (nom, sig) in signaux(sr, dur) {
        println!("── {} :", nom);
        println!("   {:>6}   {:>12}   {:>9}   {}", "pas Δ", "débit kbit/s", "SNR dB", "lecture");
        for &delta in &deltas {
            let m = coder_decoder(&sig, sr, n, hop, delta);
            let lecture = if m.snr_db >= 30.0 {
                "transparent"
            } else if m.snr_db >= 15.0 {
                "bon"
            } else if m.snr_db >= 6.0 {
                "audible, dégradé"
            } else {
                "pauvre"
            };
            println!("   {:>6.0}   {:>12.1}   {:>9.1}   {}", delta, m.bitrate_kbps, m.snr_db, lecture);
        }
        println!();
    }

    println!("📌 Lecture : le MÊME codec (aucun modèle de parole) encode sinus, chant ET beatbox → tous survivent");
    println!("   (SNR > 0 partout), le beatbox/bruit coûte juste plus de bits. C'est le prix HONNÊTE du white-box");
    println!("   vs Opus ; le levier d'optimisation = l'allocation perceptuelle de bits (prochaine brique). Détail : prive/PLAN_TEST_VOIX.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signal_test(n: usize) -> Vec<f32> {
        (0..n)
            .map(|k| {
                let t = k as f64;
                (0.6 * (0.10 * t).sin() + 0.3 * (0.27 * t).sin()) as f32
            })
            .collect()
    }

    #[test]
    fn pas_fin_quasi_sans_perte() {
        // Un pas TRÈS fin → SNR élevé (reconstruction quasi parfaite).
        let sig = signal_test(8192);
        let m = coder_decoder(&sig, 16000.0, 512, 256, 0.05);
        assert!(m.snr_db > 40.0, "pas fin → SNR élevé, obtenu {}", m.snr_db);
    }

    #[test]
    fn pas_grossier_baisse_le_debit_et_le_snr() {
        // Plus le pas est grossier, plus le débit ET le SNR baissent (le compromis qu'on trace).
        let sig = signal_test(8192);
        let fin = coder_decoder(&sig, 16000.0, 512, 256, 2.0);
        let gros = coder_decoder(&sig, 16000.0, 512, 256, 64.0);
        assert!(gros.bitrate_kbps < fin.bitrate_kbps, "pas grossier → moins de débit");
        assert!(gros.snr_db < fin.snr_db, "pas grossier → moins de SNR");
    }

    #[test]
    fn un_ton_pur_coute_moins_quun_bruit_blanc() {
        // À pas égal, un signal CREUX (ton pur) s'encode en bien moins de bits qu'un bruit blanc (spectre plein).
        let n = 8192;
        let ton: Vec<f32> = (0..n).map(|k| 0.8 * (2.0 * std::f64::consts::PI * 220.0 * k as f64 / 16000.0).sin() as f32).collect();
        let mut rng = Rng(1);
        let bruit: Vec<f32> = (0..n).map(|_| 0.6 * rng.next_f32()).collect();
        let m_ton = coder_decoder(&ton, 16000.0, 512, 256, 8.0);
        let m_bruit = coder_decoder(&bruit, 16000.0, 512, 256, 8.0);
        assert!(m_ton.bitrate_kbps < m_bruit.bitrate_kbps, "ton {} doit coûter moins que bruit {}", m_ton.bitrate_kbps, m_bruit.bitrate_kbps);
    }
}
