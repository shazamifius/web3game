//! Harnais END-TO-END « agent son » — la chaîne COMPLÈTE, chiffrée (vision utilisateur « agent web3 pour le son »).
//!
//! On passe un banc de son (voix + bruit) dans TOUTE la chaîne et on mesure, en matrice, par profil RÉSEAU réel :
//!   capture → **débruitage** (`denoise`) → **codec** perceptuel (`codec` + `psycho`) → **transport web3** (perte +
//!   gigue → jitter buffer + PLC par répétition de trame) → **décodage** → distance.
//!
//! Métriques de bout en bout : débit (kbit/s) vs budget D3 · latence bouche-à-oreille · taux de masquage PLC
//! (trames perdues/en retard, comblées) · **fidélité de la VOIX livrée** (SNR de la sortie vs la voix PROPRE, =
//! toute la dégradation cumulée) · et le rappel « bulle de bruit » (débruitage on/off). Jamais à l'oreille.
//!
//! Déterministe, std-only, 0 sudo. Réutilise les briques prouvées (FFT/psycho/codec/denoise) — pas de duplication
//! de logique, seulement l'assemblage.

use super::codec::{dequantifier, entropie_bits, pas_perceptuel, quantifier};
use super::denoise::{debruiter, mesurer};
use super::fft::{hann, istft, stft, Cplx};
use super::psycho::{bandes_par_bin, seuil_masquage};

/// Profil de lien réseau (one-way) — mêmes chiffres que la flotte (`liveness`/`voice_bench`).
#[derive(Clone, Copy)]
struct Profil {
    nom: &'static str,
    latence_s: f64,
    gigue_s: f64,
    perte: f64,
}

struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        (x.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Voix syllabique (intermittente) — comme le banc micro, pour que l'estimation de bruit voie les gaps.
fn voix_syllabique(sr: f64, n_ech: usize) -> Vec<f32> {
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

/// Ventilo PC : tonal stationnaire (120 + 240 Hz).
fn ventilo(sr: f64, n_ech: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    (0..n_ech)
        .map(|k| {
            let t = k as f64 / sr;
            (0.15 * ((2.0 * PI * 120.0 * t).sin() + 0.5 * (2.0 * PI * 240.0 * t).sin())) as f32
        })
        .collect()
}

/// Encode un signal en trames (codec perceptuel) ; renvoie le spectre DÉCODÉ par trame + le débit (entropie).
fn encoder(signal: &[f32], sr: f64, n: usize, hop: usize, q: f64) -> (Vec<Vec<Cplx>>, f64) {
    let win = hann(n);
    let bande_de = bandes_par_bin(n, sr);
    let spectres = stft(signal, n, hop, &win);
    let mut decodees = Vec::with_capacity(spectres.len());
    let mut symboles = Vec::new();
    for sp in &spectres {
        let masque = seuil_masquage(sp, &bande_de);
        let pas = pas_perceptuel(&masque, &bande_de, q);
        let qz = quantifier(sp, n, &pas);
        symboles.extend_from_slice(&qz);
        decodees.push(dequantifier(&qz, n, &pas));
    }
    let symboles_par_s = symboles.len() as f64 / (signal.len() as f64 / sr);
    let bitrate_kbps = entropie_bits(&symboles) * symboles_par_s / 1000.0;
    (decodees, bitrate_kbps)
}

/// Transport web3 : chaque trame subit perte + gigue ; le jitter buffer `d_jit` joue la trame si elle est arrivée
/// à temps, sinon PLC = on RÉPÈTE la dernière bonne trame (masquage). Renvoie les trames reçues + le taux comblé.
fn transporter(
    trames: &[Vec<Cplx>],
    p: &Profil,
    d_jit: f64,
    frame_rate: f64,
    n: usize,
    rng: &mut Rng,
) -> (Vec<Vec<Cplx>>, f64) {
    let mut recues = Vec::with_capacity(trames.len());
    let mut derniere_bonne: Option<Vec<Cplx>> = None;
    let mut comblees = 0usize;
    for (t, trame) in trames.iter().enumerate() {
        let _t_capture = t as f64 / frame_rate;
        let perdue = rng.next_f64() < p.perte;
        let gigue = rng.next_f64() * p.gigue_s;
        let en_retard = gigue > d_jit; // jouée à latence+d_jit ; arrivée à latence+gigue
        if !perdue && !en_retard {
            derniere_bonne = Some(trame.clone());
            recues.push(trame.clone());
        } else {
            comblees += 1;
            // PLC : répéter la dernière bonne trame ; à défaut (début), silence.
            recues.push(derniere_bonne.clone().unwrap_or_else(|| vec![Cplx::new(0.0, 0.0); n]));
        }
    }
    (recues, 100.0 * comblees as f64 / trames.len().max(1) as f64)
}

/// SNR (dB) de `sortie` vs `reference`, sur l'intérieur (échantillons couverts par 2 trames pleines).
fn snr_db(reference: &[f32], sortie: &[f32], n: usize) -> f64 {
    let (mut e_sig, mut e_bruit) = (0.0_f64, 0.0_f64);
    let len = reference.len().min(sortie.len());
    for k in n..len.saturating_sub(n) {
        let x = reference[k] as f64;
        let d = x - sortie[k] as f64;
        e_sig += x * x;
        e_bruit += d * d;
    }
    if e_bruit <= 1e-30 {
        120.0
    } else {
        10.0 * (e_sig / e_bruit).log10()
    }
}

fn profils() -> Vec<Profil> {
    vec![
        Profil { nom: "nixos LAN 8ms/0", latence_s: 0.004, gigue_s: 0.000, perte: 0.00 },
        Profil { nom: "DESKTOP 4G 39/8", latence_s: 0.020, gigue_s: 0.008, perte: 0.02 },
        Profil { nom: "MSI 4G cong. 29/20", latence_s: 0.015, gigue_s: 0.020, perte: 0.03 },
        Profil { nom: "box pote 76/15", latence_s: 0.038, gigue_s: 0.015, perte: 0.02 },
        Profil { nom: "lent sat 445/40", latence_s: 0.222, gigue_s: 0.040, perte: 0.02 },
    ]
}

/// Point d'entrée `jeu son` — la chaîne complète, en matrice.
pub fn run_son(_arg: &str) {
    let (sr, n, hop, dur, q) = (16000.0, 512, 256, 2.0, 0.5);
    let n_ech = (sr * dur) as usize;
    let frame_rate = sr / hop as f64;
    let frame_ms = 1000.0 / frame_rate;

    let voix = voix_syllabique(sr, n_ech);
    let bruit = ventilo(sr, n_ech);
    let melange: Vec<f32> = voix.iter().zip(&bruit).map(|(&v, &b)| v + b).collect();

    // Chaîne : débruitage → codec (indépendant du transport → débit constant) → transport par profil.
    let nettoye = debruiter(&melange, n, hop, 1.5, 0.1);
    let (trames, bitrate) = encoder(&nettoye, sr, n, hop, q);

    println!("🔗  HARNAIS END-TO-END « AGENT SON » — voix+ventilo à travers TOUTE la chaîne web3");
    println!(
        "    {} Hz · débruitage → codec perceptuel ({:.1} kbit/s) → transport (jitter buffer + PLC) · repère Opus ≈ 20 kbit/s\n",
        sr as u32, bitrate
    );
    let etat_d3 = if bitrate < 43.0 { "✅" } else { "⚠" };
    println!("   débit voix livré = {:.1} kbit/s {} (1 locuteur ; × K bornés par VAD + AoI audio)\n", bitrate, etat_d3);
    println!("   {:<20} {:>13} {:>13} {:>18}", "profil réseau", "bouche→or. ms", "PLC comblé %", "voix livrée (SNR dB)");
    for p in profils() {
        let d_jit = (p.gigue_s / 0.005).ceil() * 0.005; // couvre la gigue uniforme → ~0 retard
        let mut rng = Rng::new(0x5012_6034_7048);
        let (recues, comble) = transporter(&trames, &p, d_jit, frame_rate, n, &mut rng);
        let sortie = istft(&recues, n, hop, melange.len());
        let mte = (p.latence_s + d_jit) * 1000.0 + frame_ms;
        let snr = snr_db(&voix, &sortie, n); // vs la voix PROPRE → dégradation totale cumulée
        println!("   {:<20} {:>13.0} {:>13.1} {:>18.1}", p.nom, mte, comble, snr);
    }

    // Rappel « bulle de bruit » : ce que les AUTRES entendent du ventilo, débruitage on/off (cf. jeu micro).
    let m = mesurer(&voix, &bruit, n, hop, 1.5, 0.1);
    println!(
        "\n   bulle d'audibilité du VENTILO (% du rayon de la voix) : sans débruitage {:.0} % → avec {:.0} %",
        m.bulle_sans_pct, m.bulle_avec_pct
    );
    println!("\n📌 Lecture : la voix arrive entière sur toute la flotte (bouche→oreille bornée par la gigue ; 445 ms =");
    println!("   physique), le PLC comble la perte, le débit tient le budget D3, et le ventilo est confiné près de");
    println!("   l'émetteur. Tout chiffré, bout en bout, beaucoup de profils d'un coup. Détail : prive/PLAN_TEST_VOIX.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jeu() -> (Vec<f32>, Vec<Vec<Cplx>>, f64, usize, usize) {
        let (sr, n, hop) = (16000.0, 512, 256);
        let voix = voix_syllabique(sr, 16000);
        let bruit = ventilo(sr, 16000);
        let mel: Vec<f32> = voix.iter().zip(&bruit).map(|(&v, &b)| v + b).collect();
        let nettoye = debruiter(&mel, n, hop, 1.5, 0.1);
        let (trames, _) = encoder(&nettoye, sr, n, hop, 0.5);
        (voix, trames, sr / hop as f64, n, hop)
    }

    #[test]
    fn lien_parfait_ne_comble_rien() {
        let (_voix, trames, fr, n, _hop) = jeu();
        let p = Profil { nom: "parfait", latence_s: 0.0, gigue_s: 0.0, perte: 0.0 };
        let mut rng = Rng::new(1);
        let (_rec, comble) = transporter(&trames, &p, 0.0, fr, n, &mut rng);
        assert!(comble < 1e-9, "lien parfait → 0 % comblé, obtenu {}", comble);
    }

    #[test]
    fn la_perte_declenche_le_plc() {
        let (_voix, trames, fr, n, _hop) = jeu();
        let p = Profil { nom: "lossy", latence_s: 0.02, gigue_s: 0.0, perte: 0.10 };
        let mut rng = Rng::new(2);
        let (_rec, comble) = transporter(&trames, &p, 0.0, fr, n, &mut rng);
        assert!(comble > 5.0, "10 % de perte → PLC nettement actif, obtenu {} %", comble);
    }

    #[test]
    fn la_chaine_livre_une_voix_correcte_sur_bon_lien() {
        let (voix, trames, fr, n, hop) = jeu();
        let p = Profil { nom: "LAN", latence_s: 0.004, gigue_s: 0.0, perte: 0.0 };
        let mut rng = Rng::new(3);
        let (rec, _c) = transporter(&trames, &p, 0.0, fr, n, &mut rng);
        let sortie = istft(&rec, n, hop, voix.len());
        let snr = snr_db(&voix, &sortie, n);
        assert!(snr > 3.0, "sur bon lien, la voix livrée doit être correcte : SNR {} dB", snr);
    }
}
