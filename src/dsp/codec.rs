//! Codec voix FAIT MAIN, transform-domain, white-box (décidé le 30 juin 2026).
//!
//! Pipeline transparent, bâti sur le socle `fft.rs` : STFT (Hann, 50 % recouvrement) → **quantification scalaire**
//! des coefficients spectraux → ISTFT (overlap-add). Aucun modèle de parole (≠ vocoder LPC) → **agnostique au
//! signal** : chuchotement, chant ET beatbox survivent, ils coûtent juste plus ou moins de bits. Zéro dépendance.
//!
//! Deux allocations de bits, comparables :
//!   - **uniforme** : un seul pas `Δ` pour tous les bins (le point de départ, naïf) ;
//!   - **perceptuelle** (`psycho.rs`) : un pas PAR BANDE critique, réglé pour glisser le bruit de quantification
//!     JUSTE sous le seuil de masquage → on ne dépense des bits que là où l'oreille entend. Le levier qui se
//!     rapproche d'Opus, en restant 100 % explicable.
//!
//! Métriques : débit = **entropie de Shannon** des coefficients quantifiés (borne basse d'un codeur idéal) ;
//! distorsion brute = **SNR** ; distorsion PERÇUE = **NMR** (bruit / masque, en dB ; ≤ 0 dB = inaudible). À débit
//! égal, l'allocation perceptuelle baisse le NMR : le bruit est mieux enfoui sous le masque. C'est ça, le gain.

use super::fft::{hann, istft, stft, Cplx};
use super::psycho::{bandes_par_bin, seuil_masquage};

/// Nombre de bins UNIQUES d'un signal réel de taille `n` (le reste = symétrie conjuguée).
fn n_bins_uniques(n: usize) -> usize {
    n / 2 + 1
}

/// Quantifie les bins uniques (re, im) avec un pas PAR BIN `pas` (len = n/2+1) → entiers signés (les symboles).
fn quantifier(spectre: &[Cplx], n: usize, pas: &[f64]) -> Vec<i32> {
    let m = n_bins_uniques(n);
    let mut q = Vec::with_capacity(2 * m);
    for k in 0..m {
        q.push((spectre[k].re / pas[k]).round() as i32);
        q.push((spectre[k].im / pas[k]).round() as i32);
    }
    q
}

/// Reconstruit le spectre complet (taille `n`) depuis les symboles quantifiés, par symétrie conjuguée.
fn dequantifier(q: &[i32], n: usize, pas: &[f64]) -> Vec<Cplx> {
    let m = n_bins_uniques(n);
    let mut s = vec![Cplx::new(0.0, 0.0); n];
    for k in 0..m {
        s[k] = Cplx::new(q[2 * k] as f64 * pas[k], q[2 * k + 1] as f64 * pas[k]);
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

/// Pas de quantification PERCEPTUEL par bin : par bande, `Δ` tel que le bruit ≈ `q²·masque` (le bouton `q` règle
/// le NMR cible — `q=1` → bruit au niveau du masque ; `q<1` → sous le masque, plus de bits).
fn pas_perceptuel(masque: &[f64], bande_de: &[usize], q: f64) -> Vec<f64> {
    let nb = masque.len();
    let mut compte = vec![0usize; nb];
    for &b in bande_de {
        compte[b] += 1;
    }
    // bruit total d'une bande ≈ nbins·Δ²/6 (re+im) ; on veut = q²·masque[b]
    let pas_bande: Vec<f64> = (0..nb)
        .map(|b| (6.0 * q * q * masque[b] / compte[b].max(1) as f64).sqrt().max(1e-9))
        .collect();
    bande_de.iter().map(|&b| pas_bande[b]).collect()
}

/// Comment allouer les bits.
#[derive(Clone, Copy)]
pub enum Alloc {
    /// Un pas unique pour tous les bins.
    Uniforme(f64),
    /// Un pas par bande, réglé sous le masque (bouton qualité `q`).
    Perceptuel(f64),
}

/// Résultat d'un encodage/décodage.
pub struct Mesure {
    pub bitrate_kbps: f64,
    pub snr_db: f64,
    /// Rapport bruit/masque MAX sur les bandes (dB) : ≤ 0 = bruit sous le masque = inaudible. La vraie qualité.
    pub nmr_db: f64,
}

/// Encode puis décode `signal` selon l'allocation `alloc` ; renvoie débit, SNR et NMR.
pub fn coder_decoder(signal: &[f32], sr: f64, n: usize, hop: usize, alloc: Alloc) -> Mesure {
    let win = hann(n);
    let bande_de = bandes_par_bin(n, sr);
    let nb = bande_de.iter().copied().max().unwrap_or(0) + 1;
    let m = n_bins_uniques(n);

    let spectres = stft(signal, n, hop, &win);
    let mut tous_symboles: Vec<i32> = Vec::new();
    let mut trames_rec: Vec<Vec<Cplx>> = Vec::with_capacity(spectres.len());
    // Accumulateurs NMR par bande (bruit de quantif et masque, sommés sur les trames).
    let (mut bruit_bande, mut masque_bande) = (vec![0.0_f64; nb], vec![0.0_f64; nb]);

    for sp in &spectres {
        let masque = seuil_masquage(sp, &bande_de);
        let pas = match alloc {
            Alloc::Uniforme(d) => vec![d; m],
            Alloc::Perceptuel(q) => pas_perceptuel(&masque, &bande_de, q),
        };
        let q = quantifier(sp, n, &pas);
        let rec = dequantifier(&q, n, &pas);
        // NMR : bruit de quantif par bande (sur les bins uniques) vs masque
        for k in 0..m {
            let dr = sp[k].re - rec[k].re;
            let di = sp[k].im - rec[k].im;
            bruit_bande[bande_de[k]] += dr * dr + di * di;
        }
        for (b, &mb) in masque.iter().enumerate() {
            masque_bande[b] += mb;
        }
        tous_symboles.extend_from_slice(&q);
        trames_rec.push(rec);
    }

    let rec = istft(&trames_rec, n, hop, signal.len());

    // Débit : entropie/symbole × symboles/seconde.
    let symboles_par_s = tous_symboles.len() as f64 / (signal.len() as f64 / sr);
    let bitrate_kbps = entropie_bits(&tous_symboles) * symboles_par_s / 1000.0;

    // SNR sur l'INTÉRIEUR (échantillons couverts par 2 trames pleines).
    let (mut e_sig, mut e_bruit) = (0.0_f64, 0.0_f64);
    for k in n..signal.len().saturating_sub(n) {
        let x = signal[k] as f64;
        let d = x - rec[k] as f64;
        e_sig += x * x;
        e_bruit += d * d;
    }
    let snr_db = if e_bruit <= 1e-30 { 120.0 } else { 10.0 * (e_sig / e_bruit).log10() };

    // NMR max sur les bandes ayant un masque non négligeable.
    let nmr_db = (0..nb)
        .filter(|&b| masque_bande[b] > 1e-12)
        .map(|b| 10.0 * (bruit_bande[b] / masque_bande[b]).max(1e-12).log10())
        .fold(f64::NEG_INFINITY, f64::max);
    let nmr_db = if nmr_db.is_finite() { nmr_db } else { -120.0 };

    Mesure { bitrate_kbps, snr_db, nmr_db }
}

/// Cherche le pas uniforme qui atteint ~`debit_cible` kbit/s (débit décroissant avec Δ → recherche dichotomique).
fn uniforme_a_debit(signal: &[f32], sr: f64, n: usize, hop: usize, debit_cible: f64) -> Mesure {
    let (mut lo, mut hi) = (1e-3_f64, 1e6_f64); // Δ : fin → débit haut ; grossier → débit bas
    let mut best = coder_decoder(signal, sr, n, hop, Alloc::Uniforme(hi));
    for _ in 0..40 {
        let mid = (lo * hi).sqrt(); // dichotomie géométrique
        let m = coder_decoder(signal, sr, n, hop, Alloc::Uniforme(mid));
        if m.bitrate_kbps > debit_cible {
            lo = mid; // trop de débit → pas plus grossier
        } else {
            hi = mid;
        }
        best = m;
    }
    best
}

// ----------------------------------------------------------------------------
// BANCS — `jeu codec` (uniforme) et `jeu codec p` (perceptuel vs uniforme à débit égal)
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
    use std::f64::consts::PI;
    let n = (sr * dur) as usize;
    let t = |k: usize| k as f64 / sr;

    let sinus: Vec<f32> = (0..n).map(|k| 0.8 * (2.0 * PI * 220.0 * t(k)).sin() as f32).collect();

    let chant: Vec<f32> = (0..n)
        .map(|k| {
            let f = 165.0;
            let s: f64 = (1..=5).map(|h| (1.0 / h as f64) * (2.0 * PI * f * h as f64 * t(k)).sin()).sum();
            (0.5 * s) as f32
        })
        .collect();

    let beatbox: Vec<f32> = (0..n)
        .map(|k| {
            let periode = (sr * 0.18) as usize;
            let phase = k % periode.max(1);
            let env = (-(phase as f64) / (sr * 0.012)).exp();
            (0.8 * env * (2.0 * PI * 1400.0 * t(k)).sin()) as f32
        })
        .collect();

    let mut rng = Rng(0xC0DEC_F00D);
    let bruit: Vec<f32> = (0..n).map(|_| 0.6 * rng.next_f32()).collect();

    vec![("sinus 220 Hz", sinus), ("chant (5 harm.)", chant), ("beatbox (transit.)", beatbox), ("bruit blanc", bruit)]
}

/// Banc rate-distortion uniforme (le point de départ).
fn run_uniforme(sr: f64, n: usize, hop: usize, dur: f64) {
    let deltas = [2.0_f64, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0];
    println!("🎛️  BANC CODEC — allocation UNIFORME (point de départ naïf)");
    println!("    {} Hz · STFT {} (Hann 50 %) · débit = entropie · repère Opus ≈ 20 kbit/s\n", sr as u32, n);
    for (nom, sig) in signaux(sr, dur) {
        println!("── {} :", nom);
        println!("   {:>6}   {:>12}   {:>9}   {:>9}", "pas Δ", "débit kbit/s", "SNR dB", "NMR dB");
        for &delta in &deltas {
            let m = coder_decoder(&sig, sr, n, hop, Alloc::Uniforme(delta));
            println!("   {:>6.0}   {:>12.1}   {:>9.1}   {:>9.1}", delta, m.bitrate_kbps, m.snr_db, m.nmr_db);
        }
        println!();
    }
}

/// Banc PERCEPTUEL : à DÉBIT ÉGAL, le NMR de l'allocation perceptuelle vs l'uniforme (gain = NMR_unif − NMR_perc).
fn run_perceptuel(sr: f64, n: usize, hop: usize, dur: f64) {
    let qs = [0.25_f64, 0.5, 1.0, 2.0]; // boutons qualité (NMR cible décroissant → débit décroissant)
    println!("🧠  BANC CODEC — allocation PERCEPTUELLE vs UNIFORME, À DÉBIT ÉGAL");
    println!("    {} Hz · STFT {} · NMR ≤ 0 dB = bruit sous le masque = INAUDIBLE · gain = baisse de NMR à débit égal\n", sr as u32, n);
    for (nom, sig) in signaux(sr, dur) {
        println!("── {} :", nom);
        println!("   {:>12}   {:>12}   {:>12}   {:>10}", "débit kbit/s", "NMR perceptuel", "NMR uniforme", "gain dB");
        for &q in &qs {
            let mp = coder_decoder(&sig, sr, n, hop, Alloc::Perceptuel(q));
            if mp.bitrate_kbps < 0.05 {
                continue;
            }
            let mu = uniforme_a_debit(&sig, sr, n, hop, mp.bitrate_kbps);
            let gain = mu.nmr_db - mp.nmr_db; // > 0 → perceptuel met le bruit plus bas sous le masque
            println!("   {:>12.1}   {:>12.1}   {:>12.1}   {:>10.1}", mp.bitrate_kbps, mp.nmr_db, mu.nmr_db, gain);
        }
        println!();
    }
    println!("📌 Lecture HONNÊTE : le gain est SIGNAL/DÉBIT-dépendant. Aux débits UTILES (bonne qualité), le perceptuel");
    println!("   gagne nettement — surtout sur les sons riches/transitoires : beatbox ~+9 dB, chant ~+4 dB. Aux débits de");
    println!("   FAMINE (tout est mauvais), c'est un match nul. → le masquage white-box paie là où ça compte. Détail : prive/PLAN_TEST_VOIX.md");
}

/// Point d'entrée `jeu codec [p]`.
pub fn run_codec(arg: &str) {
    let (sr, n, hop, dur) = (16000.0, 512, 256, 1.0);
    if arg.starts_with('p') {
        run_perceptuel(sr, n, hop, dur);
    } else {
        run_uniforme(sr, n, hop, dur);
    }
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
        let sig = signal_test(8192);
        let m = coder_decoder(&sig, 16000.0, 512, 256, Alloc::Uniforme(0.05));
        assert!(m.snr_db > 40.0, "pas fin → SNR élevé, obtenu {}", m.snr_db);
    }

    #[test]
    fn pas_grossier_baisse_le_debit_et_le_snr() {
        let sig = signal_test(8192);
        let fin = coder_decoder(&sig, 16000.0, 512, 256, Alloc::Uniforme(2.0));
        let gros = coder_decoder(&sig, 16000.0, 512, 256, Alloc::Uniforme(64.0));
        assert!(gros.bitrate_kbps < fin.bitrate_kbps, "pas grossier → moins de débit");
        assert!(gros.snr_db < fin.snr_db, "pas grossier → moins de SNR");
    }

    #[test]
    fn un_ton_pur_coute_moins_quun_bruit_blanc() {
        let n = 8192;
        let ton: Vec<f32> =
            (0..n).map(|k| 0.8 * (2.0 * std::f64::consts::PI * 220.0 * k as f64 / 16000.0).sin() as f32).collect();
        let mut rng = Rng(1);
        let bruit: Vec<f32> = (0..n).map(|_| 0.6 * rng.next_f32()).collect();
        let m_ton = coder_decoder(&ton, 16000.0, 512, 256, Alloc::Uniforme(8.0));
        let m_bruit = coder_decoder(&bruit, 16000.0, 512, 256, Alloc::Uniforme(8.0));
        assert!(m_ton.bitrate_kbps < m_bruit.bitrate_kbps, "ton {} < bruit {}", m_ton.bitrate_kbps, m_bruit.bitrate_kbps);
    }

    #[test]
    fn perceptuel_bat_uniforme_a_debit_egal() {
        // LE point : sur un signal au spectre déséquilibré (chant à harmoniques), à débit égal, l'allocation
        // perceptuelle obtient un NMR <= celui de l'uniforme (bruit mieux glissé sous le masque).
        let sr = 16000.0;
        let n = 512;
        let hop = 256;
        let f = 165.0;
        let chant: Vec<f32> = (0..16000)
            .map(|k| {
                let t = k as f64 / sr;
                let s: f64 = (1..=5)
                    .map(|h| (1.0 / h as f64) * (2.0 * std::f64::consts::PI * f * h as f64 * t).sin())
                    .sum();
                (0.5 * s) as f32
            })
            .collect();
        // À un point de fonctionnement UTILE (q=0.5 → bonne qualité), le perceptuel doit STRICTEMENT battre
        // l'uniforme à débit égal. (À débit de FAMINE, c'est un match nul — propriété honnête, pas affirmée ici.)
        let mp = coder_decoder(&chant, sr, n, hop, Alloc::Perceptuel(0.5));
        let mu = uniforme_a_debit(&chant, sr, n, hop, mp.bitrate_kbps);
        assert!(
            mp.nmr_db < mu.nmr_db,
            "à débit égal ({:.1} kbit/s), perceptuel NMR {:.1} doit < uniforme NMR {:.1}",
            mp.bitrate_kbps,
            mp.nmr_db,
            mu.nmr_db
        );
    }
}
