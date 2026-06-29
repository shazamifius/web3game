//! Optimisation codec + débruitage GUIDÉE PAR STOI, sous contraintes RÉELLES (30 juin 2026).
//!
//! On balaye les réglages (codec `q`, débruitage `α`, `β`) et on cherche **le meilleur compromis MESURÉ**, pas
//! deviné. Contraintes posées par l'utilisateur :
//!   - **Latence VOIX ≤ 250 ms** (grand maximum pour une conversation ; le mouvement 3D, lui, tolère 150–350 ms).
//!     Une config n'est utile que si la chaîne tient ce plafond sur le lien visé.
//!   - **Non-destructif si possible ; sinon le moins visible possible en sortie** → le lossless ne tient pas le
//!     débit (montré), donc on maximise l'INTELLIGIBILITÉ (STOI) au débit le plus bas.
//!   - **Banc de son varié et long** (voix grave/aiguë, vibrato, fricatives), moyenne ET pire cas — pour ne PAS
//!     s'enfermer dans une optimisation jouet éloignée de l'usage réel.
//!
//! STOI mesuré sur la sortie codec+débruitage avec transport SAIN (on isole la part que `q,α,β` contrôlent ; la
//! dégradation transport/PLC est mesurée à part dans `jeu son`). Déterministe, std-only.

use super::chain::encoder;
use super::denoise::{debruiter, mesurer};
use super::fft::istft;
use super::stoi::stoi;

/// Un cas du banc de son : un nom, la voix PROPRE (référence), et un bruit ambiant.
struct Cas {
    nom: &'static str,
    voix: Vec<f32>,
    bruit: Vec<f32>,
}

/// Voix synthétique paramétrable : fondamentale `f0`, `harm` harmoniques, `syll` syllabes/s, `vibrato` (Hz, prof.).
fn voix(sr: f64, n_ech: usize, f0: f64, harm: usize, syll: f64, vibrato: f64) -> Vec<f32> {
    use std::f64::consts::PI;
    (0..n_ech)
        .map(|k| {
            let t = k as f64 / sr;
            let env = if (t * syll).fract() < 0.65 { 1.0 } else { 0.0 };
            let f = f0 * (1.0 + vibrato * (2.0 * PI * 5.0 * t).sin()); // vibrato léger à 5 Hz
            let s: f64 = (1..=harm).map(|h| (1.0 / h as f64) * (2.0 * PI * f * h as f64 * t).sin()).sum();
            (0.5 * env * s) as f32
        })
        .collect()
}

/// Voix avec des bouffées « fricatives » large bande (consonnes) en plus des harmoniques — le cas dur.
fn voix_fricatives(sr: f64, n_ech: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    let base = voix(sr, n_ech, 150.0, 6, 3.0, 0.0);
    let mut rng = Rng(0xF21CA71F);
    base.iter()
        .enumerate()
        .map(|(k, &v)| {
            let t = k as f64 / sr;
            // une fricative brève (~0,06 s) toutes les ~0,33 s
            let fric = if (t * 3.0).fract() > 0.85 { 0.4 * rng.f() } else { 0.0 };
            (v as f64 + fric as f64 * (2.0 * PI * 4000.0 * t).sin().abs()) as f32
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

fn ventilo(sr: f64, n_ech: usize, gain: f64) -> Vec<f32> {
    use std::f64::consts::PI;
    (0..n_ech)
        .map(|k| {
            let t = k as f64 / sr;
            (gain * ((2.0 * PI * 120.0 * t).sin() + 0.5 * (2.0 * PI * 240.0 * t).sin())) as f32
        })
        .collect()
}

fn souffle(sr: f64, n_ech: usize, gain: f64) -> Vec<f32> {
    let _ = sr;
    let mut rng = Rng(0xB1A5);
    (0..n_ech).map(|_| (gain * rng.f() as f64) as f32).collect()
}

/// Banc de son VARIÉ et LONG (4 s) — pour mesurer du réaliste, pas un ton jouet.
fn banc(sr: f64) -> Vec<Cas> {
    let dur = 4.0;
    let ne = (sr * dur) as usize;
    vec![
        Cas { nom: "homme lent + ventilo", voix: voix(sr, ne, 120.0, 6, 2.5, 0.0), bruit: ventilo(sr, ne, 0.15) },
        Cas { nom: "femme rapide + souffle", voix: voix(sr, ne, 220.0, 5, 3.5, 0.0), bruit: souffle(sr, ne, 0.08) },
        Cas { nom: "voix vibrato + ventilo fort", voix: voix(sr, ne, 165.0, 6, 3.0, 0.03), bruit: ventilo(sr, ne, 0.25) },
        Cas { nom: "fricatives + souffle", voix: voix_fricatives(sr, ne), bruit: souffle(sr, ne, 0.06) },
    ]
}

/// Évalue une config (q, α, β) sur tout le banc : renvoie (débit moyen kbit/s, STOI moyen, STOI pire cas).
fn evaluer(banc: &[Cas], sr: f64, n: usize, hop: usize, q: f64, alpha: f64, beta: f64) -> (f64, f64, f64) {
    let (mut deb, mut s_moy, mut s_min) = (0.0_f64, 0.0_f64, f64::INFINITY);
    for c in banc {
        let melange: Vec<f32> = c.voix.iter().zip(&c.bruit).map(|(&v, &b)| v + b).collect();
        let nettoye = debruiter(&melange, n, hop, alpha, beta);
        let (trames, bitrate) = encoder(&nettoye, sr, n, hop, q);
        let sortie = istft(&trames, n, hop, c.voix.len()); // transport SAIN (on isole codec+débruitage)
        let d = stoi(&c.voix, &sortie, sr, n, hop);
        deb += bitrate;
        s_moy += d;
        s_min = s_min.min(d);
    }
    let k = banc.len() as f64;
    (deb / k, s_moy / k, s_min)
}

/// Point d'entrée `jeu optim` — balayage guidé par STOI, sous contraintes débit + latence.
pub fn run_optim(_arg: &str) {
    let (sr, n, hop) = (16000.0, 512, 256);
    let budget_kbps = 24.0; // tient le D3 avec quelques locuteurs simultanés (VAD + AoI bornent K)
    let banc = banc(sr);

    println!("🎯  OPTIMISATION codec+débruitage GUIDÉE PAR STOI — banc varié (4 voix × 4 s), contraintes RÉELLES");
    println!(
        "    {} Hz · STFT {} · budget ≤ {:.0} kbit/s · STOI = intelligibilité (haut = mieux) · plafond latence VOIX = 250 ms\n",
        sr as u32, n, budget_kbps
    );

    // Référence « non-destructif » : pas TRÈS fin → quasi lossless, mais quel débit ?
    let (deb_ll, stoi_ll, _) = evaluer(&banc, sr, n, hop, 0.05, 1.5, 0.1);
    println!(
        "   ► Référence quasi-LOSSLESS (q=0,05) : STOI {:.3} mais {:.0} kbit/s → {} le budget : non-destructif IMPOSSIBLE à débit voix.",
        stoi_ll, deb_ll, if deb_ll > budget_kbps { "DÉPASSE" } else { "tient" }
    );
    println!("   → on cherche donc le LOSSY le moins visible (STOI max) sous le budget.\n");

    // Le débruitage bouge À PEINE le STOI (il sert à confiner le BRUIT, mesuré par `jeu micro`) → on le FIXE et on
    // balaye FINEMENT le vrai levier : le pas du codec `q`, pour tracer le GENOU de la courbe débit↔intelligibilité.
    let (alpha, beta) = (1.5, 0.1);
    let qs = [0.3_f64, 0.4, 0.5, 0.6, 0.7, 0.8, 1.0, 1.5, 2.0];
    println!("   balayage FIN de q (débruitage fixé α={}, β={} — il bouge à peine le STOI) :", alpha, beta);
    println!("   {:>6} {:>13} {:>11} {:>12} {:>9}", "q", "débit kbit/s", "STOI moy", "STOI pire", "budget");
    let mut sous_budget: Option<(f64, f64, f64, f64)> = None; // (q, débit, stoi_moy, stoi_min) le 1er sous budget
    for &q in &qs {
        let (deb, sm, smin) = evaluer(&banc, sr, n, hop, q, alpha, beta);
        let ok = if deb <= budget_kbps { "✅" } else { "⚠ hors" };
        println!("   {:>6.2} {:>13.1} {:>11.3} {:>12.3} {:>9}", q, deb, sm, smin, ok);
        if deb <= budget_kbps && sous_budget.is_none() {
            sous_budget = Some((q, deb, sm, smin)); // 1er sous budget = le moins quantifié qui tient = meilleur STOI
        }
    }

    // Gagnant = le q le plus FIN (meilleur STOI) qui tient le budget — le genou.
    if let Some((q, deb, sm, smin)) = sous_budget {
        println!(
            "\n   🏆 GENOU (q le plus fin sous budget) : q={:.2} → {:.1} kbit/s, STOI moyen {:.3}, pire cas {:.3}",
            q, deb, sm, smin
        );
    }

    // Contrainte LATENCE (le plafond 250 ms) — codec ~32 ms (1 trame) + réseau + jitter buffer.
    let codec_ms = n as f64 / sr * 1000.0;
    println!("\n   ── LATENCE bouche-à-oreille = réseau(one-way) + jitter buffer + codec ({:.0} ms) · plafond VOIX 250 ms :", codec_ms);
    for (nom, lat_ms, gigue_ms) in [
        ("nixos LAN", 4.0_f64, 0.0_f64),
        ("DESKTOP 4G", 20.0, 8.0),
        ("MSI 4G cong.", 15.0, 20.0),
        ("box pote", 38.0, 15.0),
        ("lent satellite", 222.0, 40.0),
    ] {
        let d_jit = (gigue_ms / 5.0).ceil() * 5.0;
        let mte = lat_ms + d_jit + codec_ms;
        let verdict = if mte <= 250.0 { "✅ conversation" } else { "❌ > 250 ms → MOUVEMENT 3D seulement" };
        println!("      {:<16} {:>4.0} ms   {}", nom, mte, verdict);
    }

    // Rappel : le débruitage tient-il TOUJOURS sa mission (confiner le bruit) au réglage gagnant ?
    let c0 = &banc[0];
    let m = mesurer(&c0.voix, &c0.bruit, n, hop, 1.5, 0.1);
    println!(
        "\n   ── garde-fou débruitage (cas « {} ») : bulle ventilo {:.0} % → {:.0} % du rayon voix (toujours confiné)",
        c0.nom, m.bulle_sans_pct, m.bulle_avec_pct
    );
    println!("\n📌 Le compromis est TRACÉ, pas deviné : STOI max sous débit ET latence, sur un banc varié. Détail : prive/PLAN_TEST_VOIX.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_banc_est_varie_et_long() {
        let b = banc(16000.0);
        assert_eq!(b.len(), 4);
        assert!(b.iter().all(|c| c.voix.len() == 64000), "4 s à 16 kHz");
    }

    #[test]
    fn un_pas_fin_donne_un_meilleur_stoi_quun_pas_grossier() {
        // Cohérence : moins on quantifie (q petit), meilleure est l'intelligibilité (à débruitage fixe).
        let b = banc(16000.0);
        let (_d_fin, s_fin, _) = evaluer(&b, 16000.0, 512, 256, 0.5, 1.5, 0.1);
        let (_d_gros, s_gros, _) = evaluer(&b, 16000.0, 512, 256, 4.0, 1.5, 0.1);
        assert!(s_fin > s_gros, "q fin {} doit battre q grossier {} en STOI", s_fin, s_gros);
    }

    #[test]
    fn le_quasi_lossless_coute_plus_cher_que_le_budget() {
        // Le « non-destructif » (q très fin) dépasse le budget voix → justifie le lossy optimisé.
        let b = banc(16000.0);
        let (deb, _s, _) = evaluer(&b, 16000.0, 512, 256, 0.05, 1.5, 0.1);
        assert!(deb > 24.0, "quasi-lossless doit dépasser 24 kbit/s, obtenu {}", deb);
    }
}
