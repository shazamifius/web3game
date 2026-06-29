//! Contrôleur ADAPTATIF voix — la boucle « JUGE + CALCULE » (idée utilisateur, 30 juin 2026).
//!
//! Les réglages ne sont PAS figés une fois pour toutes : ils s'adaptent aux conditions du moment. Architecture
//! à deux étages (le point clé) :
//!   - **Juge HORS-LIGNE = STOI** (sur bancs, plein-référence) → conçoit la POLITIQUE (quel réglage pour quelles
//!     conditions). C'est `jeu optim`. STOI ne peut PAS juger EN DIRECT (il faut la voix propre de référence).
//!   - **Juge EN LIGNE = les conditions OBSERVABLES** (RTT/gigue/perte de la sonde, budget reçu, niveau de bruit) →
//!     le **CONTRÔLEUR** (ci-dessous) applique la politique et ré-adapte en continu.
//!
//! Le contrôleur est 100 % white-box (des règles qu'on écrit/explique), chaque règle justifiée par une mesure :
//!   - `q` ← **régulation de débit** : on cherche le `q` le plus fin (meilleur STOI) dont le débit RÉEL du signal
//!     courant tient le budget reçu (le budget bouge avec le lien et le nombre de locuteurs : VAD + AoI).
//!   - `d_jit` ← couvre la gigue mesurée, MAIS plafonné par le budget latence VOIX 250 ms ; si même sans tampon la
//!     latence dépasse 250 ms → la voix est déclarée INFAISABLE (repli mouvement 3D). Honnête, pas magique.
//!   - débruitage ← activé si le niveau de bruit estimé dépasse un seuil.

use super::chain::chaine;
use super::denoise::debruiter;
use super::stoi::stoi;

const CODEC_MS: f64 = 32.0; // latence algorithmique du codec ≈ 1 trame (n/sr à 512/16k)
const PLAFOND_VOIX_MS: f64 = 250.0;

/// Conditions OBSERVABLES en direct (ce que le « juge en ligne » lit — pas de STOI ici).
#[derive(Clone, Copy)]
pub struct Conditions {
    pub rtt_ms: f64,
    pub gigue_ms: f64,
    pub perte: f64,
    pub budget_kbps: f64,  // débit reçu disponible pour MA voix (D3 / nb locuteurs)
    pub bruit_niveau: f64, // amplitude RMS du bruit ambiant estimé (0 = silencieux)
}

/// Réglages PRODUITS par le contrôleur (le « calcule »).
#[derive(Clone, Copy)]
pub struct Reglages {
    pub q: f64,
    pub d_jit_ms: f64,
    pub debruitage: bool,
    pub mte_ms: f64,
    pub faisable_voix: bool,
}

/// Régulation de débit : le `q` le plus FIN (meilleur STOI) dont le débit du signal courant tient le budget.
/// (Débit décroissant en `q` → recherche dichotomique de la frontière.)
fn q_pour_budget(signal: &[f32], sr: f64, n: usize, hop: usize, budget_kbps: f64) -> f64 {
    use super::chain::encoder;
    let (mut lo, mut hi) = (0.05_f64, 16.0_f64);
    for _ in 0..30 {
        let mid = (lo * hi).sqrt();
        let (_, br) = encoder(signal, sr, n, hop, mid);
        if br > budget_kbps {
            lo = mid; // trop de débit → quantifier plus grossièrement (q plus grand)
        } else {
            hi = mid;
        }
    }
    hi // le q le plus fin qui tient le budget
}

/// Politique latence : `d_jit` couvre la gigue, plafonné par le budget 250 ms ; faisabilité voix honnête.
fn politique_latence(rtt_ms: f64, gigue_ms: f64) -> (f64, f64, bool) {
    let one_way = rtt_ms / 2.0;
    let d_jit_ideal = (gigue_ms / 5.0).ceil() * 5.0; // couvre la gigue (pas de 5 ms)
    let budget_buffer = (PLAFOND_VOIX_MS - CODEC_MS - one_way).max(0.0);
    let d_jit = d_jit_ideal.min(budget_buffer);
    let mte = one_way + CODEC_MS + d_jit;
    let faisable = one_way + CODEC_MS <= PLAFOND_VOIX_MS; // tient-on même sans tampon ?
    (d_jit, mte, faisable)
}

/// Réglages de débruitage (α, β) selon l'activation. β=1 → gain 1 → débruitage NEUTRE (passe-tout).
fn params_debruitage(actif: bool) -> (f64, f64) {
    if actif {
        (1.5, 0.1)
    } else {
        (0.0, 1.0)
    }
}

/// LE CONTRÔLEUR : conditions + signal courant → réglages (politique tirée des mesures `jeu optim`/`voix`/`micro`).
pub fn controler(c: &Conditions, signal: &[f32], sr: f64, n: usize, hop: usize) -> Reglages {
    let (d_jit_ms, mte_ms, faisable_voix) = politique_latence(c.rtt_ms, c.gigue_ms);
    let debruitage = c.bruit_niveau > 0.02; // seuil : au-dessus, le ventilo/souffle mérite d'être confiné
    // Régulation de q sur le signal RÉELLEMENT encodé (= APRÈS débruitage), sinon le débit visé est faux.
    let (alpha, beta) = params_debruitage(debruitage);
    let encode = if debruitage { debruiter(signal, n, hop, alpha, beta) } else { signal.to_vec() };
    let q = q_pour_budget(&encode, sr, n, hop, c.budget_kbps);
    Reglages { q, d_jit_ms, debruitage, mte_ms, faisable_voix }
}

// ----------------------------------------------------------------------------
// BANC `jeu adaptatif` — l'adaptatif (juge+calcule) BAT le réglage fixe quand les conditions varient
// ----------------------------------------------------------------------------

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

fn ventilo(sr: f64, n_ech: usize, gain: f64) -> Vec<f32> {
    use std::f64::consts::PI;
    (0..n_ech).map(|k| (gain * (2.0 * PI * 120.0 * k as f64 / sr).sin()) as f32).collect()
}

/// Exécute la chaîne avec des réglages donnés et renvoie (STOI, débit, % comblé).
fn evaluer(voix: &[f32], melange: &[f32], c: &Conditions, r: &Reglages, sr: f64, n: usize, hop: usize) -> (f64, f64, f64) {
    let (alpha, beta) = params_debruitage(r.debruitage);
    let (sortie, bitrate, comble) =
        chaine(melange, sr, n, hop, r.q, alpha, beta, c.rtt_ms / 2000.0, c.gigue_ms / 1000.0, c.perte, r.d_jit_ms / 1000.0, 0x99);
    (stoi(voix, &sortie, sr, n, hop), bitrate, comble)
}

/// Point d'entrée `jeu adaptatif`.
pub fn run_adaptatif(_arg: &str) {
    let (sr, n, hop) = (16000.0, 512, 256);
    let ne = (sr * 3.0) as usize;
    let v = voix(sr, ne);
    let bruit = ventilo(sr, ne, 0.15);
    let melange: Vec<f32> = v.iter().zip(&bruit).map(|(&a, &b)| a + b).collect();
    let bruit_rms = (bruit.iter().map(|&x| (x * x) as f64).sum::<f64>() / bruit.len() as f64).sqrt();

    // Réglage FIXE de référence (le point trouvé par jeu optim) — figé, NON adaptatif.
    let (q_fixe, djit_fixe_ms) = (0.6, 40.0);

    println!("🔄  CONTRÔLEUR ADAPTATIF (juge STOI hors-ligne → politique ; juge conditions en ligne → calcule)");
    println!("    {} Hz · plafond VOIX {:.0} ms · l'adaptatif ré-règle q (budget), d_jit (gigue, plafond latence), débruitage (bruit)\n", sr as u32, PLAFOND_VOIX_MS);
    println!("   {:<22} {:>7} {:>8} | {:>16} | {:>18}", "condition", "budget", "lien", "ADAPTATIF (q·STOI)", "FIXE q=0,6 (STOI·état)");

    // Conditions VARIÉES : budgets serrés/larges × liens de la flotte.
    let scenarios = [
        ("LAN, budget large", 24.0, 4.0_f64, 0.0_f64, 0.0_f64),
        ("4G, budget moyen", 14.0, 40.0, 8.0, 0.02),
        ("4G congestionné serré", 8.0, 30.0, 20.0, 0.03),
        ("box pote, budget moyen", 14.0, 76.0, 15.0, 0.02),
        ("satellite (latence)", 14.0, 445.0, 40.0, 0.02),
    ];
    for (nom, budget, rtt, gigue, perte) in scenarios {
        let c = Conditions { rtt_ms: rtt, gigue_ms: gigue, perte, budget_kbps: budget, bruit_niveau: bruit_rms };
        let r = controler(&c, &melange, sr, n, hop);

        // FIXE : q=0,6, d_jit=40 ms, débruitage on — mêmes conditions.
        let r_fixe = Reglages { q: q_fixe, d_jit_ms: djit_fixe_ms, debruitage: true, mte_ms: 0.0, faisable_voix: true };
        let (stoi_f, deb_f, _cf) = evaluer(&v, &melange, &c, &r_fixe, sr, n, hop);
        let etat_fixe = if deb_f > budget { "⚠ HORS budget" } else { "ok" };

        if !r.faisable_voix {
            // Honnête : on ne prétend pas livrer de la voix au-delà de 250 ms — repli mouvement 3D.
            println!(
                "   {:<22} {:>5.0}k  {:>4.0}ms | VOIX INFAISABLE ({:.0} ms > 250) → 3D | (fixe mentirait : STOI {:.2})",
                nom, budget, rtt, r.mte_ms, stoi_f
            );
            continue;
        }
        let (stoi_a, deb_a, _comble_a) = evaluer(&v, &melange, &c, &r, sr, n, hop);
        println!(
            "   {:<22} {:>5.0}k  {:>4.0}ms | q={:.2} {:>4.1}k STOI {:.2} ({:.0}ms) | STOI {:.2} {:>6.1}k {}",
            nom, budget, rtt, r.q, deb_a, stoi_a, r.mte_ms, stoi_f, deb_f, etat_fixe
        );
    }
    println!("\n📌 Lecture : l'ADAPTATIF tient TOUJOURS le budget (il régule q) et le plafond latence (il borne d_jit, et");
    println!("   déclare la voix infaisable sur le satellite au lieu de mentir) ; le réglage FIXE q=0,6 SORT du budget");
    println!("   quand il se resserre. Juge (STOI/conditions) + calcule (contrôleur) = adaptation auto. Détail : prive/PLAN_TEST_VOIX.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig() -> (Vec<f32>, Vec<f32>) {
        let (sr, ne) = (16000.0, (16000.0 * 3.0) as usize);
        let v = voix(sr, ne);
        let b = ventilo(sr, ne, 0.15);
        let mel = v.iter().zip(&b).map(|(&a, &c)| a + c).collect();
        (v, mel)
    }

    #[test]
    fn budget_serre_donne_q_plus_grossier() {
        let (_v, mel) = sig();
        let q_large = q_pour_budget(&mel, 16000.0, 512, 256, 24.0);
        let q_serre = q_pour_budget(&mel, 16000.0, 512, 256, 8.0);
        assert!(q_serre > q_large, "budget serré → q plus grossier ({} vs {})", q_serre, q_large);
    }

    #[test]
    fn le_satellite_declare_la_voix_infaisable() {
        let (d, _mte, faisable) = politique_latence(445.0, 40.0);
        assert!(!faisable, "445 ms RTT → voix infaisable (> 250 ms)");
        let _ = d;
        let (_d2, _m2, ok) = politique_latence(40.0, 8.0);
        assert!(ok, "40 ms RTT → voix faisable");
    }

    #[test]
    fn ladaptatif_tient_le_budget_ou_le_fixe_le_depasse() {
        // Sur un budget serré (8 kbit/s), l'adaptatif régule q pour tenir ; le fixe q=0,6 (~18 kbit/s) le dépasse.
        let (v, mel) = sig();
        let (sr, n, hop) = (16000.0, 512, 256);
        let c = Conditions { rtt_ms: 30.0, gigue_ms: 20.0, perte: 0.03, budget_kbps: 8.0, bruit_niveau: 0.1 };
        let r = controler(&c, &mel, sr, n, hop);
        let (_stoi_a, deb_a, _) = evaluer(&v, &mel, &c, &r, sr, n, hop);
        let r_fixe = Reglages { q: 0.6, d_jit_ms: 40.0, debruitage: true, mte_ms: 0.0, faisable_voix: true };
        let (_stoi_f, deb_f, _) = evaluer(&v, &mel, &c, &r_fixe, sr, n, hop);
        assert!(deb_a <= 8.0 + 0.5, "adaptatif tient le budget : {} kbit/s", deb_a);
        assert!(deb_f > deb_a, "le fixe dépasse l'adaptatif sur budget serré ({} vs {})", deb_f, deb_a);
    }
}
