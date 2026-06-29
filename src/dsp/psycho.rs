//! Modèle psychoacoustique MINIMAL, white-box — masquage simultané sur l'échelle de Bark.
//!
//! Sert l'allocation perceptuelle de bits du codec : dépenser les bits là où l'oreille ENTEND, quantifier
//! grossièrement là où un son fort **masque** le bruit. Aucune boîte noire : juste l'échelle de Bark (Zwicker)
//! et la fonction d'étalement de Schroeder — des formules qu'on écrit et qu'on explique.
//!
//! L'idée : le bruit de quantification est INAUDIBLE tant qu'il reste sous le **seuil de masquage** de sa bande.
//! On calcule ce seuil par bande critique (l'énergie de chaque bande « déborde » sur ses voisines via l'étalement),
//! puis le codec règle le pas de quantification de chaque bande pour glisser son bruit JUSTE sous ce seuil.

use super::fft::Cplx;

/// Échelle de Bark (Zwicker) : fréquence (Hz) → numéro de bande critique (continu, ~0 à 24 sur 0–16 kHz).
pub fn bark(f_hz: f64) -> f64 {
    13.0 * (0.00076 * f_hz).atan() + 3.5 * (f_hz / 7500.0).powi(2).atan()
}

/// Indice de bande critique (entier) de chaque bin unique `0..=n/2` (un bin = une fréquence `k·sr/n`).
pub fn bandes_par_bin(n: usize, sr: f64) -> Vec<usize> {
    (0..=n / 2)
        .map(|k| bark(k as f64 * sr / n as f64).floor().max(0.0) as usize)
        .collect()
}

/// Fonction d'étalement de Schroeder (en dB) selon la distance `dz` en Bark : ~0 dB au centre, décroît vite
/// (un masqueur fort « éclaire » ses voisines, de moins en moins loin qu'il s'éloigne).
fn etalement_db(dz: f64) -> f64 {
    15.81 + 7.5 * (dz + 0.474) - 17.5 * (1.0 + (dz + 0.474).powi(2)).sqrt()
}

/// Seuil de masquage par BANDE (énergie linéaire) : l'énergie de chaque bande, étalée sur toutes les bandes
/// via la fonction d'étalement. `m[b]` = niveau de bruit qui resterait INAUDIBLE dans la bande `b`.
pub fn seuil_masquage(spectre: &[Cplx], bande_de: &[usize]) -> Vec<f64> {
    let nb = bande_de.iter().copied().max().unwrap_or(0) + 1;
    // (1) énergie par bande
    let mut e = vec![0.0_f64; nb];
    for (k, &b) in bande_de.iter().enumerate() {
        e[b] += spectre[k].re * spectre[k].re + spectre[k].im * spectre[k].im;
    }
    // (2) étalement (convolution dans le domaine Bark — bandes ~1 Bark de large)
    let mut m = vec![0.0_f64; nb];
    for (b, mb) in m.iter_mut().enumerate() {
        *mb = e
            .iter()
            .enumerate()
            .map(|(bp, &ebp)| ebp * 10f64.powf(etalement_db(b as f64 - bp as f64) / 10.0))
            .sum();
    }
    // (3) PLANCHER d'audition (ATH simplifié, white-box) : rien sous -40 dB du masque le plus fort n'est audible.
    // Sans lui, une bande quasi vide a un masque minuscule → un résidu infime y crée un NMR aberrant. Ce plancher
    // est relatif (invariant d'échelle) et capture l'idée du seuil absolu d'audition.
    let plancher = 1e-4 * m.iter().copied().fold(0.0_f64, f64::max); // -40 dB
    for mb in m.iter_mut() {
        *mb = mb.max(plancher);
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bark_croit_avec_la_frequence() {
        assert!(bark(100.0) < bark(1000.0));
        assert!(bark(1000.0) < bark(8000.0));
    }

    #[test]
    fn un_son_fort_masque_ses_voisines() {
        // Une seule bande chargée d'énergie → le seuil de masquage de la bande VOISINE (vide) est relevé
        // au-dessus de sa propre énergie (= 0) grâce à l'étalement. C'est tout l'effet du masquage.
        let n = 512;
        let sr = 16000.0;
        let bande_de = bandes_par_bin(n, sr);
        // spectre nul sauf un pic à un bin de fréquence moyenne
        let mut spectre = vec![Cplx::new(0.0, 0.0); n];
        let k_pic = 60; // ~1875 Hz
        spectre[k_pic] = Cplx::new(100.0, 0.0);
        let m = seuil_masquage(&spectre, &bande_de);
        let b_pic = bande_de[k_pic];
        assert!(m[b_pic] > 0.0);
        // une bande voisine (vide d'énergie propre) reçoit quand même un seuil > 0 par étalement
        let b_voisin = b_pic + 1;
        if b_voisin < m.len() {
            assert!(m[b_voisin] > 0.0, "le masque doit déborder sur la bande voisine");
        }
    }
}
