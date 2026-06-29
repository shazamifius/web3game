//! FFT radix-2 (Cooley-Tukey) fait main + STFT/overlap-add, std-only, zéro dépendance.
//!
//! Tout est explicable (white-box) : papillons en place, twiddles calculés à la volée, fenêtre de Hann. La
//! reconstruction analyse→synthèse est EXACTE à l'intérieur grâce à la condition COLA (Hann à 50 % de
//! recouvrement : `w(k) + w(k + N/2) = 1`). C'est le socle commun du codec voix et de l'étude du micro.

use std::f64::consts::PI;

/// Un complexe minimal (parties réelle/imaginaire en f64 pour la précision des papillons).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cplx {
    pub re: f64,
    pub im: f64,
}

impl Cplx {
    pub fn new(re: f64, im: f64) -> Self {
        Cplx { re, im }
    }
}

/// Transformée en place, radix-2. `n = buf.len()` DOIT être une puissance de 2. `inverse=false` → directe ;
/// `inverse=true` → inverse (avec normalisation 1/n, pour que `ifft(fft(x)) == x`).
fn transformer(buf: &mut [Cplx], inverse: bool) {
    let n = buf.len();
    assert!(n.is_power_of_two(), "FFT : longueur {} non puissance de 2", n);
    if n <= 1 {
        return;
    }
    // 1) Permutation par inversion de bits (met les entrées dans l'ordre des papillons).
    let mut j = 0usize;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            buf.swap(i, j);
        }
    }
    // 2) Papillons, par tailles de bloc croissantes (2, 4, 8, … n).
    let mut taille = 2usize;
    while taille <= n {
        let angle = (if inverse { 2.0 } else { -2.0 }) * PI / taille as f64;
        let (wre, wim) = (angle.cos(), angle.sin()); // facteur de rotation élémentaire
        let demi = taille / 2;
        let mut base = 0usize;
        while base < n {
            // twiddle courant, avancé multiplicativement (white-box : pas de table opaque)
            let (mut tr, mut ti) = (1.0_f64, 0.0_f64);
            for k in 0..demi {
                let a = buf[base + k];
                let b = buf[base + k + demi];
                let bre = tr * b.re - ti * b.im;
                let bim = tr * b.im + ti * b.re;
                buf[base + k] = Cplx::new(a.re + bre, a.im + bim);
                buf[base + k + demi] = Cplx::new(a.re - bre, a.im - bim);
                let ntr = tr * wre - ti * wim;
                ti = tr * wim + ti * wre;
                tr = ntr;
            }
            base += taille;
        }
        taille <<= 1;
    }
    if inverse {
        let inv = 1.0 / n as f64;
        for x in buf.iter_mut() {
            x.re *= inv;
            x.im *= inv;
        }
    }
}

/// FFT directe en place.
pub fn fft(buf: &mut [Cplx]) {
    transformer(buf, false);
}

/// FFT inverse en place (normalisée → `ifft(fft(x)) == x` à l'epsilon près).
pub fn ifft(buf: &mut [Cplx]) {
    transformer(buf, true);
}

/// Fenêtre de Hann périodique de taille `n` : `w(k) = 0.5·(1 − cos(2πk/n))`.
/// À 50 % de recouvrement, `w(k) + w(k + n/2) = 1` (COLA) → overlap-add exact sans fenêtre de synthèse.
pub fn hann(n: usize) -> Vec<f64> {
    (0..n)
        .map(|k| 0.5 * (1.0 - (2.0 * PI * k as f64 / n as f64).cos()))
        .collect()
}

/// STFT : découpe `signal` en trames de `n` (puissance de 2) au saut `hop`, applique `win`, FFT chaque trame.
/// Renvoie la suite des spectres (un `Vec<Cplx>` de taille `n` par trame).
pub fn stft(signal: &[f32], n: usize, hop: usize, win: &[f64]) -> Vec<Vec<Cplx>> {
    assert_eq!(win.len(), n, "fenêtre de taille ≠ trame");
    let mut frames = Vec::new();
    let mut start = 0usize;
    while start + n <= signal.len() {
        let mut buf: Vec<Cplx> = (0..n)
            .map(|k| Cplx::new(signal[start + k] as f64 * win[k], 0.0))
            .collect();
        fft(&mut buf);
        frames.push(buf);
        start += hop;
    }
    frames
}

/// ISTFT : IFFT chaque spectre puis overlap-add à `hop`. Avec une analyse Hann à 50 % de recouvrement, la
/// reconstruction est EXACTE à l'intérieur (les bords, couverts par une seule trame, sont atténués par la fenêtre).
pub fn istft(frames: &[Vec<Cplx>], n: usize, hop: usize, out_len: usize) -> Vec<f32> {
    let mut out = vec![0.0_f64; out_len];
    for (f, frame) in frames.iter().enumerate() {
        let mut buf = frame.clone();
        ifft(&mut buf);
        let start = f * hop;
        for k in 0..n {
            if start + k < out_len {
                out[start + k] += buf[k].re;
            }
        }
    }
    out.iter().map(|&x| x as f32).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un signal déterministe « riche » (somme de sinus à fréquences incommensurables) — représentatif d'une
    /// voix (plusieurs harmoniques), pas du bruit blanc.
    fn signal(n: usize) -> Vec<f32> {
        (0..n)
            .map(|k| {
                let t = k as f64;
                (0.6 * (0.10 * t).sin() + 0.3 * (0.27 * t).sin() + 0.1 * (0.53 * t).cos()) as f32
            })
            .collect()
    }

    #[test]
    fn fft_puis_ifft_redonne_lentree() {
        let n = 1024;
        let sig = signal(n);
        let mut buf: Vec<Cplx> = sig.iter().map(|&x| Cplx::new(x as f64, 0.0)).collect();
        fft(&mut buf);
        ifft(&mut buf);
        let err: f64 = sig
            .iter()
            .zip(&buf)
            .map(|(&x, c)| (x as f64 - c.re).abs())
            .fold(0.0, f64::max);
        assert!(err < 1e-9, "round-trip FFT→IFFT doit être exact, erreur max = {}", err);
    }

    #[test]
    fn fft_concentre_un_cosinus_sur_son_bin() {
        // Un cosinus pur à exactement `bin` périodes sur la fenêtre → toute l'énergie aux bins `bin` et n-bin.
        let n = 512;
        let bin = 7usize;
        let mut buf: Vec<Cplx> = (0..n)
            .map(|k| Cplx::new((2.0 * PI * bin as f64 * k as f64 / n as f64).cos(), 0.0))
            .collect();
        fft(&mut buf);
        let module = |c: &Cplx| (c.re * c.re + c.im * c.im).sqrt();
        let pic = module(&buf[bin]);
        // Tous les autres bins (hors bin et son symétrique n-bin) doivent être négligeables.
        let fuite: f64 = (0..n)
            .filter(|&k| k != bin && k != n - bin)
            .map(|k| module(&buf[k]))
            .fold(0.0, f64::max);
        assert!(pic > 1e3 * (fuite + 1e-12), "le pic ({}) doit dominer la fuite ({})", pic, fuite);
    }

    #[test]
    fn stft_puis_istft_reconstruit_a_linterieur() {
        let n = 1024;
        let hop = n / 2; // 50 % → COLA Hann
        let win = hann(n);
        let sig = signal(4096);
        let frames = stft(&sig, n, hop, &win);
        let rec = istft(&frames, n, hop, sig.len());
        // Intérieur = échantillons couverts par 2 trames pleines (on saute la 1re et la dernière demi-trame).
        let err: f64 = (n..sig.len() - n)
            .map(|k| (sig[k] - rec[k]).abs() as f64)
            .fold(0.0, f64::max);
        assert!(err < 1e-5, "reconstruction STFT→ISTFT exacte à l'intérieur, erreur max = {}", err);
    }
}
