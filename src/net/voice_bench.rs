//! Banc « voix de proximité » (D35) — mesurer la FAISABILITÉ du transport audio, façon `jeu vivant`.
//!
//! Même méthode que `liveness.rs` : un émetteur produit des TRAMES voix horodatées à `f_tx` (50 Hz = 20 ms,
//! standard Opus) ; un **canal** déterministe applique latence/gigue/perte/réordonnancement (les profils RÉELS
//! de la flotte, calibrés par la sonde) ; un **jitter buffer** côté récepteur joue les trames avec un retard
//! `d_jit`. On chiffre les métriques OBJECTIVES de la voix :
//!   1. **Latence bouche-à-oreille** = latence one-way + `d_jit` + durée de trame. À MINIMISER (conversation
//!      naturelle ≤ ~200 ms ; au-delà de ~400 ms on se coupe la parole — repère ITU G.114).
//!   2. **% de trames EN RETARD** (arrivées après leur instant de lecture → masquées par le PLC d'Opus = glitch).
//!      Réductible par `d_jit`. À distinguer de la **perte canal** (irréductible, du ressort de PLC/FEC).
//!   3. **Octets/s REÇUS** (en-têtes compris) × K locuteurs → le mur D3 (~43 Ko/s déjà pris par le jeu).
//!
//! Sortie clé (comme vivant traçait `d_opt` vs lien) : **`d_jit` optimal vs qualité du lien** → « tient-on la
//! conversation naturelle sur le 4G congestionné / le satellite ? » devient une réponse CHIFFRÉE, par profil RÉEL.
//!
//! Tout est SIMULÉ et déterministe (graine fixe), std-only, 0 sudo, rejouable à toute échelle. Le codec (Opus)
//! et la capture/lecture/spatialisation vivent côté Unreal (cf. `prive/PLAN_TEST_VOIX.md`) ; ICI on ne modélise
//! que le TRANSPORT (la qualité Opus/PLC est connue + confirmée par le spike humain).
//!
//! ⚠ Les profils sont les MÊMES que `liveness.rs` (flotte du 29 juin, sonde STUN) — à unifier dans un
//! `link_profiles.rs` partagé quand on aura un 3e usage (pour l'instant : éviter de toucher le banc vivant prouvé).

/// RNG déterministe (xorshift64*), std-only — pertes/gigue reproductibles. (Identique à `liveness.rs`.)
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Profil de lien : latence ONE-WAY (s), gigue max (s, ajoutée uniformément), perte (fraction).
/// Une voix bouche-à-oreille parcourt UN sens → la latence one-way (= RTT/2) est la bonne grandeur.
#[derive(Clone, Copy)]
struct Profil {
    nom: &'static str,
    latence_s: f64,
    gigue_s: f64,
    perte: f64,
}

/// Une trame voix : l'instant de CAPTURE (= l'audio qu'elle contient) ; sa taille de payload est fixe (Opus CBR).
#[derive(Clone, Copy)]
struct Trame {
    t_capture: f64,
}

/// Une trame REÇUE après le canal : l'instant d'ARRIVÉE + l'instant de capture qu'elle représente.
#[derive(Clone, Copy)]
struct Recu {
    t_arrive: f64,
    t_capture: f64,
}

/// Émet une trame toutes les `1/f_tx` s sur `duree` s.
fn emettre(duree: f64, f_tx: f64) -> Vec<Trame> {
    let dt = 1.0 / f_tx;
    let n = (duree * f_tx) as usize;
    (0..n).map(|k| Trame { t_capture: k as f64 * dt }).collect()
}

/// Applique le canal : perte (drop), puis arrivée = `t_capture + latence + U(0,gigue)`. Trie par arrivée
/// (le réordonnancement émerge si la gigue dépasse l'intervalle inter-trames — 20 ms à 50 Hz).
fn canal(trames: &[Trame], p: &Profil, rng: &mut Rng) -> Vec<Recu> {
    let mut out = Vec::new();
    for t in trames {
        if rng.next_f64() < p.perte {
            continue; // perdu par le réseau (irréductible : du ressort du PLC/FEC)
        }
        let gigue = rng.next_f64() * p.gigue_s;
        out.push(Recu { t_arrive: t.t_capture + p.latence_s + gigue, t_capture: t.t_capture });
    }
    out.sort_by(|a, b| a.t_arrive.partial_cmp(&b.t_arrive).unwrap());
    out
}

/// Résultat d'une lecture par jitter buffer à un `d_jit` donné.
struct Lecture {
    retard_pct: f64, // % de trames ARRIVÉES mais TROP TARD (réductible par d_jit)
    perte_pct: f64,  // % de trames perdues par le canal (irréductible)
    max_gap: usize,  // plus longue série de trames consécutives masquées (longueur du glitch audible)
}

/// Simule le jitter buffer : une trame capturée à `t_capture` est jouée à `t_capture + latence + d_jit`.
/// Elle est UTILISABLE si elle est arrivée avant cet instant ; sinon « en retard » (masquée par le PLC).
/// `n_emis` = nombre total de trames émises (pour les pourcentages, perte canal comprise).
fn lire(recus: &[Recu], p: &Profil, d_jit: f64, n_emis: usize, f_tx: f64) -> Lecture {
    use std::collections::HashSet;
    let mut a_temps: HashSet<u64> = HashSet::new();
    for r in recus {
        let t_play = r.t_capture + p.latence_s + d_jit;
        if r.t_arrive <= t_play + 1e-9 {
            // index de trame = instant de capture / dt (déterministe, sert de clé)
            a_temps.insert((r.t_capture * f_tx).round() as u64);
        }
    }
    let mut max_gap = 0usize;
    let mut gap = 0usize;
    for k in 0..n_emis as u64 {
        if a_temps.contains(&k) {
            gap = 0;
        } else {
            gap += 1;
            max_gap = max_gap.max(gap);
        }
    }
    let manquantes = n_emis - a_temps.len();
    let perdues = n_emis - recus.len(); // perte canal
    let en_retard = manquantes.saturating_sub(perdues); // arrivées mais trop tard
    Lecture {
        retard_pct: 100.0 * en_retard as f64 / n_emis as f64,
        perte_pct: 100.0 * perdues as f64 / n_emis as f64,
        max_gap,
    }
}

/// Plus petit `d_jit` (pas de 5 ms, plafond 250 ms) tel que le retard ≤ `seuil_retard_pct`. Retourne aussi la
/// lecture à ce `d_jit`. Si aucun ne suffit (gigue énorme), retourne le plafond.
fn d_jit_optimal(recus: &[Recu], p: &Profil, n_emis: usize, f_tx: f64, seuil: f64) -> (f64, Lecture) {
    let mut d = 0.0_f64;
    loop {
        let lec = lire(recus, p, d, n_emis, f_tx);
        if lec.retard_pct <= seuil || d >= 0.250 {
            return (d, lec);
        }
        d += 0.005;
    }
}

/// Verdict conversation, à partir de la latence bouche-à-oreille (ms) et du retard résiduel (%).
fn verdict(bouche_oreille_ms: f64, retard_pct: f64) -> &'static str {
    if retard_pct > 2.0 {
        "haché (gigue non absorbée)"
    } else if bouche_oreille_ms <= 200.0 {
        "✓ conversation naturelle"
    } else if bouche_oreille_ms <= 400.0 {
        "audible, léger délai"
    } else {
        "audible mais en retard (physique du lien)"
    }
}

/// Octets par trame : payload Opus (CBR) + en-tête applicatif + en-tête IP/UDP (non maîtrisé).
/// `id_octets` = taille de l'étiquette émetteur (32 = clé pub brute ; 2 = id de session court → le levier).
fn octets_par_trame(bitrate_kbps: f64, frame_s: f64, id_octets: usize) -> (usize, usize, usize) {
    let payload = (bitrate_kbps * 1000.0 / 8.0 * frame_s).round() as usize; // Opus CBR
    let app = 1 /*KIND_VOICE*/ + 1 /*version*/ + id_octets + 2 /*len u16*/;
    let ip_udp = 28; // IPv4 (20) + UDP (8)
    (payload, app, ip_udp)
}

/// Banc complet : balayage des profils RÉELS + le budget D3 + l'effet de l'en-tête.
pub fn run_voix(_arg: &str) {
    let f_tx = 50.0; // 50 Hz = trames de 20 ms (standard Opus)
    let frame_s = 1.0 / f_tx;
    let duree = 30.0; // 30 s d'audio simulé → stats stables
    let seuil_retard = 0.5; // on vise < 0,5 % de trames en retard (le PLC d'Opus absorbe l'isolé)
    let frame_ms = frame_s * 1000.0;

    println!("🎙️  BANC VOIX DE PROXIMITÉ (D35) — faisabilité du transport audio P2P");
    println!(
        "    émission {:.0} Hz (trames {:.0} ms) · {:.0} s simulés · seuil retard ≤ {:.1} % · graine fixe (déterministe)\n",
        f_tx, frame_ms, duree, seuil_retard
    );

    // ── Profils RÉELS de la flotte (sonde STUN, 29 juin) + bornes (parfait / satellite) ───────────────
    // latence_s = one-way = RTT/2. Mêmes chiffres que liveness.rs §profils réels (à unifier plus tard).
    let profils = [
        Profil { nom: "parfait (réf.)",       latence_s: 0.000, gigue_s: 0.000, perte: 0.00 },
        Profil { nom: "nixos LAN 8ms/0",      latence_s: 0.004, gigue_s: 0.000, perte: 0.00 },
        Profil { nom: "Nagashima CH 21ms/2",  latence_s: 0.010, gigue_s: 0.002, perte: 0.01 },
        Profil { nom: "DESKTOP 4G 39ms/8",    latence_s: 0.020, gigue_s: 0.008, perte: 0.02 },
        Profil { nom: "MSI 4G cong. 29ms/20", latence_s: 0.015, gigue_s: 0.020, perte: 0.03 },
        Profil { nom: "box pote 76ms/15",     latence_s: 0.038, gigue_s: 0.015, perte: 0.02 },
        Profil { nom: "lent (sat/mobile) 445ms/40", latence_s: 0.222, gigue_s: 0.040, perte: 0.02 },
    ];

    println!("── d_jit OPTIMAL vs qualité du lien (le compromis tracé, pas deviné) :");
    println!(
        "   {:<26} {:>8} {:>10} {:>10} {:>9}  {}",
        "profil", "d_jit", "bouche→or.", "retard", "perte", "verdict"
    );
    for p in &profils {
        let trames = emettre(duree, f_tx);
        let n = trames.len();
        let mut rng = Rng::new(0x5151_2626_3737_4848);
        let recus = canal(&trames, p, &mut rng);
        let (d_jit, lec) = d_jit_optimal(&recus, p, n, f_tx, seuil_retard);
        let bouche_oreille_ms = (p.latence_s + d_jit) * 1000.0 + frame_ms; // + une trame (encodage/paquetisation)
        println!(
            "   {:<26} {:>6.0} ms {:>7.0} ms {:>8.2} % {:>7.1} %  {}",
            p.nom,
            d_jit * 1000.0,
            bouche_oreille_ms,
            lec.retard_pct,
            lec.perte_pct,
            verdict(bouche_oreille_ms, lec.retard_pct)
        );
        if lec.max_gap > 1 {
            println!("        ↳ trou audible le plus long : {} trames consécutives masquées ({:.0} ms)", lec.max_gap, lec.max_gap as f64 * frame_ms);
        }
    }

    // ── Le mur D3 : octets reçus × K locuteurs ────────────────────────────────────────────────────────
    println!("\n── 🔴 MUR D3 — octets/s REÇUS (en-têtes compris) vs le budget ~43 Ko/s (déjà pris par le jeu) :");
    let bitrate = 20.0; // Opus parole = 16–24 kbit/s ; 20 = transparent
    for (label, id_oct) in [("id PLEINE (clé pub 32 o)", 32usize), ("id SESSION courte (2 o)", 2usize)] {
        let (payload, app, ip_udp) = octets_par_trame(bitrate, frame_s, id_oct);
        let par_trame = payload + app + ip_udp;
        let par_flux = par_trame as f64 * f_tx; // o/s pour 1 locuteur actif
        println!(
            "   {:<26} {} o/trame (payload {} + app {} + IP/UDP {}) → {:.1} Ko/s par locuteur ACTIF",
            label, par_trame, payload, app, ip_udp, par_flux / 1000.0
        );
        for k in [1usize, 4, 8] {
            let recu = par_flux * k as f64 / 1000.0;
            let etat = if recu < 43.0 { "✅ tient" } else { "⚠ dépasse" };
            println!("        {} locuteurs simultanés près de moi → {:.1} Ko/s reçus  {}", k, recu, etat);
        }
    }
    println!(
        "   → l'en-tête (32→2 o) est un VRAI levier à {:.0} Hz ; le VAD (silence = 0 octet) borne K aux gens qui parlent VRAIMENT.",
        f_tx
    );

    println!("\n📌 Lecture : la GIGUE pilote `d_jit` (donc la latence bouche-à-oreille) ; la PERTE est du ressort du PLC/FEC ;");
    println!("   le DÉBIT reçu est tenu par VAD + AoI audio + Opus bas débit + id de session courte. Détail : prive/PLAN_TEST_VOIX.md");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(p: &Profil, d_jit: f64) -> Lecture {
        let f_tx = 50.0;
        let trames = emettre(10.0, f_tx);
        let n = trames.len();
        let mut rng = Rng::new(42);
        let recus = canal(&trames, p, &mut rng);
        lire(&recus, p, d_jit, n, f_tx)
    }

    #[test]
    fn lien_parfait_zero_retard_zero_perte() {
        // Latence/gigue/perte nulles → rien en retard, rien perdu, même à d_jit = 0.
        let p = Profil { nom: "parfait", latence_s: 0.0, gigue_s: 0.0, perte: 0.0 };
        let lec = run(&p, 0.0);
        assert!(lec.retard_pct < 1e-9, "retard {}", lec.retard_pct);
        assert!(lec.perte_pct < 1e-9, "perte {}", lec.perte_pct);
        assert_eq!(lec.max_gap, 0);
    }

    #[test]
    fn la_gigue_exige_du_buffer() {
        // À d_jit = 0 sur un lien à grosse gigue, beaucoup de trames arrivent en retard ;
        // un d_jit ≥ gigue les rattrape (le retard tombe à ~0).
        let p = Profil { nom: "gigue", latence_s: 0.02, gigue_s: 0.030, perte: 0.0 };
        let sans = run(&p, 0.0);
        let avec = run(&p, 0.030);
        assert!(sans.retard_pct > 10.0, "sans buffer, retard attendu élevé : {}", sans.retard_pct);
        assert!(avec.retard_pct < 1.0, "avec buffer ≥ gigue, retard ~0 : {}", avec.retard_pct);
    }

    #[test]
    fn la_perte_est_irreductible_par_le_buffer() {
        // La perte CANAL ne se rattrape PAS en bufferisant : elle reste ≈ p.perte quel que soit d_jit.
        let p = Profil { nom: "perte", latence_s: 0.01, gigue_s: 0.002, perte: 0.10 };
        let lec = run(&p, 0.100);
        assert!((lec.perte_pct - 10.0).abs() < 3.0, "perte ~10 % attendue : {}", lec.perte_pct);
    }

    #[test]
    fn d_jit_optimal_croit_avec_la_gigue() {
        // Plus de gigue → d_jit optimal plus grand (le compromis qu'on trace).
        let f_tx = 50.0;
        let faible = Profil { nom: "faible gigue", latence_s: 0.02, gigue_s: 0.005, perte: 0.0 };
        let forte = Profil { nom: "forte gigue", latence_s: 0.02, gigue_s: 0.040, perte: 0.0 };
        let opt = |p: &Profil| {
            let tr = emettre(20.0, f_tx);
            let n = tr.len();
            let mut rng = Rng::new(7);
            let recus = canal(&tr, p, &mut rng);
            d_jit_optimal(&recus, p, n, f_tx, 0.5).0
        };
        assert!(opt(&forte) > opt(&faible), "d_jit doit croître avec la gigue");
    }
}
