//! BANC « VIVANT » — mesurer la FIDÉLITÉ du mouvement perçu vs la vérité (D27).
//!
//! **Idée de l'utilisateur (29 juin 2026).** Transformer « est-ce vivant ? » (subjectif) en une métrique
//! OBJECTIVE, déterministe, optimisable. Méthode complète : `prive/PLAN_TEST_VIVANT.md`.
//!
//! Le bot joue une **trajectoire vraie** `p*(t)` (analytique → on la connaît à tout instant) ; il émet des
//! états `(pos, vel)` à `f_tx` ; un **canal** déterministe applique latence/gigue/perte ; un **récepteur de
//! référence** reconstruit la position affichée `p̂(t)` à `f_rx` par interpolation **Hermite** (qui respecte les
//! vitesses → reproduit EXACTEMENT une ligne droite) + extrapolation par la vitesse quand le futur manque. On
//! compare les deux courbes :
//!   • **Fidélité `F`** = `min_d` RMSE(`p̂(t)`, `p*(t−d)`) → l'erreur de FORME, à retard compensé (« 500 ms de
//!     retard mais identique = parfait »).
//!   • **Fraîcheur `d_eff`** = le `d` qui réalise ce min → le retard EFFECTIF perçu (à MINIMISER ; cible 150 ms).
//!   • **Fluidité `J`** = jerk RMS de `p̂` + nombre de **sauts** (téléports visibles).
//!
//! Tout est SIMULÉ et déterministe (graine fixe) : aucun réseau réel, aucun sudo → rejouable à n'importe quelle
//! échelle (les vrais liens, via la sonde Phase 2, ne servent qu'à CALIBRER les profils injectés ici).
//!
//! Lancement :  jeu vivant [calme|agitee]

use crate::math::Vec3;

/// RNG déterministe (xorshift64*), std-only — pertes/gigue reproductibles.
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
    /// Flottant dans [0, 1).
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

// ----------------------------------------------------------------------------
// VÉRITÉ — trajectoires analytiques (position + vitesse exactes à tout instant)
// ----------------------------------------------------------------------------

/// Le type de mouvement joué par le bot. Somme de sinus → trajectoire lisse (C∞) à vitesse et accélération
/// variables (changements de direction), bien plus représentative qu'un bruit blanc.
#[derive(Clone, Copy)]
pub enum Traj {
    /// Marche tranquille (basses fréquences, petites amplitudes).
    Calme,
    /// Course avec esquives (fréquences ×3 → virages serrés : le cas DUR pour la prédiction).
    Agitee,
}

/// Position ET vitesse vraies à l'instant `t` (vitesse = dérivée analytique exacte des sinus).
fn echantillon_verite(traj: Traj, t: f64) -> (Vec3, Vec3) {
    // (ax, wx) … : amplitude (m) et pulsation (rad/s) par axe et par harmonique.
    let (k, a1, w1, a2, w2, ay, wy) = match traj {
        Traj::Calme => (1.0_f64, 4.0, 0.6, 1.0, 1.3, 0.2, 2.0),
        Traj::Agitee => (1.0_f64, 4.0, 1.8, 1.5, 3.9, 0.3, 5.0),
    };
    let _ = k;
    let x = a1 * (w1 * t).sin() + a2 * (w2 * t).sin();
    let z = a1 * (w1 * 0.83 * t).cos() + a2 * (w2 * 0.77 * t).cos();
    let y = ay * (wy * t).sin();
    let vx = a1 * w1 * (w1 * t).cos() + a2 * w2 * (w2 * t).cos();
    let vz = -a1 * (w1 * 0.83) * (w1 * 0.83 * t).sin() - a2 * (w2 * 0.77) * (w2 * 0.77 * t).sin();
    let vy = ay * wy * (wy * t).cos();
    (Vec3::new(x as f32, y as f32, z as f32), Vec3::new(vx as f32, vy as f32, vz as f32))
}

// ----------------------------------------------------------------------------
// ÉMISSION + CANAL
// ----------------------------------------------------------------------------

/// Un état émis par le bot (horodaté de l'instant qu'il représente).
#[derive(Clone, Copy)]
struct Paquet {
    t_send: f64,
    pos: Vec3,
    vel: Vec3,
}

/// Échantillonne la vérité à `f_tx` et produit la suite d'états émis sur `duree` secondes.
fn emettre(traj: Traj, duree: f64, f_tx: f64) -> Vec<Paquet> {
    let dt = 1.0 / f_tx;
    let n = (duree * f_tx) as usize;
    (0..n)
        .map(|k| {
            let t = k as f64 * dt;
            let (pos, vel) = echantillon_verite(traj, t);
            Paquet { t_send: t, pos, vel }
        })
        .collect()
}

/// Profil de lien : latence ONE-WAY (s), gigue max (s, ajoutée uniformément), perte (fraction).
/// *(Les valeurs s'inspirent des liens réels mesurés par la sonde — RTT/2 pour le one-way.)*
#[derive(Clone, Copy)]
struct Profil {
    nom: &'static str,
    latence_s: f64,
    gigue_s: f64,
    perte: f64,
}

/// Un état reçu (après le canal) : l'instant d'ARRIVÉE + l'instant qu'il REPRÉSENTE (`t_send`).
#[derive(Clone, Copy)]
struct Recu {
    t_arrive: f64,
    t_send: f64,
    pos: Vec3,
    vel: Vec3,
}

/// Applique le canal : perte (drop), puis arrivée = `t_send + latence + U(0,gigue)`. Trie par arrivée
/// (le ré-ordonnancement émerge naturellement si la gigue dépasse l'intervalle inter-paquets).
fn canal(paquets: &[Paquet], p: &Profil, rng: &mut Rng) -> Vec<Recu> {
    let mut out = Vec::new();
    for paq in paquets {
        if rng.next_f64() < p.perte {
            continue; // perdu
        }
        let gigue = rng.next_f64() * p.gigue_s;
        out.push(Recu {
            t_arrive: paq.t_send + p.latence_s + gigue,
            t_send: paq.t_send,
            pos: paq.pos,
            vel: paq.vel,
        });
    }
    out.sort_by(|a, b| a.t_arrive.partial_cmp(&b.t_arrive).unwrap());
    out
}

// ----------------------------------------------------------------------------
// RÉCEPTEUR DE RÉFÉRENCE (interpolation Hermite + extrapolation par la vitesse)
// ----------------------------------------------------------------------------

/// Un snapshot disponible côté récepteur (l'instant qu'il représente + pos/vel).
#[derive(Clone, Copy)]
struct Snap {
    t: f64,
    pos: Vec3,
    vel: Vec3,
}

/// Interpolation Hermite cubique entre deux snapshots à l'instant `tc ∈ [a.t, b.t]`. Respecte les vitesses
/// aux extrémités → courbe lisse, et **reproduit EXACTEMENT une ligne droite** (vitesses = pente du segment).
fn hermite(a: &Snap, b: &Snap, tc: f64) -> Vec3 {
    let dt = b.t - a.t;
    if dt <= 0.0 {
        return a.pos;
    }
    let s = ((tc - a.t) / dt) as f32;
    let dtf = dt as f32;
    let s2 = s * s;
    let s3 = s2 * s;
    let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
    let h10 = s3 - 2.0 * s2 + s;
    let h01 = -2.0 * s3 + 3.0 * s2;
    let h11 = s3 - s2;
    a.pos * h00 + a.vel * (dtf * h10) + b.pos * h01 + b.vel * (dtf * h11)
}

/// Extrapolation par la vitesse (quand le snapshot « après » n'est pas encore arrivé).
fn extrapol(a: &Snap, tc: f64) -> Vec3 {
    a.pos + a.vel * ((tc - a.t) as f32)
}

/// Reconstruit la courbe affichée `p̂(t)` à la cadence `f_rx`, avec un délai d'interpolation `d_interp` :
/// on vise l'instant `t − d_interp`, on interpole (Hermite) entre les deux snapshots encadrants DÉJÀ arrivés
/// (causalité stricte), ou on extrapole par la vitesse si le futur manque. Déterministe.
fn reconstruire(recus: &[Recu], duree: f64, f_rx: f64, d_interp: f64) -> Vec<(f64, Vec3)> {
    let dt = 1.0 / f_rx;
    let n_frames = (duree * f_rx) as usize;
    let mut store: Vec<Snap> = Vec::new(); // trié par t (croissant)
    let mut next = 0usize;
    let mut out = Vec::with_capacity(n_frames);

    for i in 0..n_frames {
        let t = i as f64 * dt;
        // Intégrer les états désormais arrivés, en gardant `store` trié par t_send (gère le ré-ordre).
        while next < recus.len() && recus[next].t_arrive <= t {
            let r = recus[next];
            let snap = Snap { t: r.t_send, pos: r.pos, vel: r.vel };
            let k = store.partition_point(|s| s.t < snap.t);
            if !(k < store.len() && store[k].t == snap.t) {
                store.insert(k, snap); // dédup sur t_send identique
            }
            next += 1;
        }

        let tc = t - d_interp;
        let pos = if store.is_empty() {
            Vec3::ZERO
        } else {
            let k = store.partition_point(|s| s.t <= tc); // 1er snapshot avec t > tc
            if k == 0 {
                store[0].pos // cible avant le 1er snapshot connu : on tient la position de départ
            } else if k < store.len() {
                hermite(&store[k - 1], &store[k], tc) // encadré : interpolation
            } else {
                extrapol(&store[k - 1], tc) // futur pas encore arrivé : extrapolation
            }
        };
        out.push((t, pos));
    }
    out
}

// ----------------------------------------------------------------------------
// MÉTRIQUES
// ----------------------------------------------------------------------------

/// Fidélité de FORME + retard EFFECTIF : on cherche le décalage `d ≥ 0` qui aligne au mieux la courbe perçue
/// sur la vérité, et on renvoie (RMSE résiduelle `F` en mètres, `d_eff` en secondes). `pos_vraie` donne la
/// position vraie à un instant. On ignore les `warmup` premières secondes (remplissage du buffer).
fn fidelite(percu: &[(f64, Vec3)], pos_vraie: impl Fn(f64) -> Vec3, warmup: f64, d_max: f64) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    let mut d = 0.0;
    while d <= d_max + 1e-9 {
        let mut sum = 0.0;
        let mut n = 0u32;
        for &(t, ph) in percu {
            let tv = t - d;
            if tv < warmup {
                continue;
            }
            let e = ph.distance(pos_vraie(tv)) as f64;
            sum += e * e;
            n += 1;
        }
        if n > 0 {
            let rmse = (sum / n as f64).sqrt();
            if rmse < best.0 {
                best = (rmse, d);
            }
        }
        d += 0.002; // grille de 2 ms
    }
    best
}

/// Jerk RMS (norme de la 3ᵉ différence finie de la position perçue, m/s³). Un trou de paquet mal comblé pique.
fn jerk_rms(percu: &[(f64, Vec3)], f_rx: f64, warmup: f64) -> f64 {
    let dt = 1.0 / f_rx;
    let dt3 = (dt * dt * dt) as f32;
    let mut sum = 0.0;
    let mut n = 0u32;
    for i in 3..percu.len() {
        if percu[i].0 < warmup {
            continue;
        }
        let j = (percu[i].1 - percu[i - 1].1 * 3.0 + percu[i - 2].1 * 3.0 - percu[i - 3].1) / dt3;
        let m = j.length() as f64;
        sum += m * m;
        n += 1;
    }
    if n > 0 {
        (sum / n as f64).sqrt()
    } else {
        0.0
    }
}

/// Nombre de « sauts » : déplacements entre deux frames > `seuil_m` (téléports visibles).
fn n_sauts(percu: &[(f64, Vec3)], warmup: f64, seuil_m: f32) -> usize {
    percu
        .windows(2)
        .filter(|w| w[1].0 >= warmup && w[1].1.distance(w[0].1) > seuil_m)
        .count()
}

// ----------------------------------------------------------------------------
// LE BANC
// ----------------------------------------------------------------------------

/// Affiche le balayage : pour chaque profil de lien, et chaque délai d'interpolation, les 3 métriques.
/// `jeu vivant [calme|agitee]`.
pub fn run_vivant(traj_name: &str) {
    let traj = match traj_name {
        "calme" => Traj::Calme,
        _ => Traj::Agitee,
    };
    let nom_traj = match traj {
        Traj::Calme => "calme (marche)",
        Traj::Agitee => "agitée (course + esquives)",
    };
    let (duree, f_tx, f_rx, warmup, seed) = (20.0_f64, 20.0_f64, 60.0_f64, 2.0_f64, 0x5EED_u64);
    let seuil_saut = 0.5_f32; // m entre deux frames = téléport visible

    println!("=== BANC VIVANT — trajectoire {nom_traj} ; f_tx={f_tx:.0} Hz, f_rx={f_rx:.0} Hz, {duree:.0} s ===");
    println!("    F = erreur de FORME (à retard compensé) · d_eff = retard EFFECTIF perçu · J = jerk (saccades)");
    println!("    Cible « vivant » : F faible · d_eff ≤ 500 ms (ambition 150) · J bas · 0 saut.");

    let paquets = emettre(traj, duree, f_tx);
    let pos_vraie = |t: f64| echantillon_verite(traj, t).0;

    // Jerk NATUREL de la trajectoire (la vérité échantillonnée à l'affichage) = la référence de fluidité :
    // une reconstruction parfaite atteint ce jerk-là, jamais moins. Bien au-dessus = saccades artificielles.
    let n_frames = (duree * f_rx) as usize;
    let verite_frames: Vec<(f64, Vec3)> =
        (0..n_frames).map(|i| { let t = i as f64 / f_rx; (t, pos_vraie(t)) }).collect();
    let j_ref = jerk_rms(&verite_frames, f_rx, warmup);
    // Seuils de verdict, CALIBRÉS sur ces premiers runs (cf. PLAN_TEST_VIVANT §9, à affiner) :
    let (f_ok, j_ok) = (0.02_f64, j_ref * 2.0); // F ≤ 2 cm ; jerk ≤ 2× le naturel ; + 0 saut ; d_eff ≤ 500 ms
    println!("    Jerk NATUREL de cette trajectoire = {j_ref:.0} m/s³ (référence : « vivant » ⇒ J ≲ {j_ok:.0}).\n");

    let profils = [
        Profil { nom: "parfait", latence_s: 0.0, gigue_s: 0.0, perte: 0.0 },
        Profil { nom: "fibre (~LAN)", latence_s: 0.004, gigue_s: 0.0, perte: 0.0 },
        Profil { nom: "fixe lointain (CH)", latence_s: 0.010, gigue_s: 0.003, perte: 0.005 },
        Profil { nom: "4G correct", latence_s: 0.019, gigue_s: 0.008, perte: 0.03 },
        Profil { nom: "4G congestionné", latence_s: 0.030, gigue_s: 0.030, perte: 0.08 },
    ];
    let grille_d = [0.0, 0.05, 0.10, 0.15, 0.20, 0.30, 0.50];

    for p in profils {
        // Un SEUL tirage réseau par profil (graine fixe) → les délais d sont comparables entre eux.
        let mut rng = Rng::new(seed ^ (p.nom.len() as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let recus = canal(&paquets, &p, &mut rng);
        let recu_pct = 100.0 * recus.len() as f64 / paquets.len() as f64;
        println!(
            "── {} (latence {:.0} ms, gigue {:.0} ms, perte {:.0} % → {:.0} % reçus)",
            p.nom, p.latence_s * 1000.0, p.gigue_s * 1000.0, p.perte * 100.0, recu_pct
        );
        println!(
            "   {:>9} | {:>8} | {:>10} | {:>10} | {:>5} | {}",
            "d_interp", "d_eff", "F (cm)", "J (m/s³)", "sauts", "verdict"
        );
        for &d in &grille_d {
            let percu = reconstruire(&recus, duree, f_rx, d);
            let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.7);
            let j = jerk_rms(&percu, f_rx, warmup);
            let sauts = n_sauts(&percu, warmup, seuil_saut);
            let verdict = if sauts > 0 || j > j_ok {
                "✗ saccadé"
            } else if f > f_ok {
                "✗ flou"
            } else if d_eff > 0.5 + 1e-6 {
                "✗ en retard"
            } else {
                "✓ vivant"
            };
            println!(
                "   {:>7.0}ms | {:>6.0}ms | {:>10.2} | {:>10.1} | {:>5} | {}",
                d * 1000.0, d_eff * 1000.0, f * 100.0, j, sauts, verdict
            );
        }
        println!();
    }
    println!("Lecture : pour chaque lien, le BON d_interp est le plus PETIT qui garde F et J bas (0 saut).");
    println!("→ c'est le compromis fraîcheur ↔ fidélité, tracé par la mesure (cf. prive/PLAN_TEST_VIVANT.md).");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// GARDE-FOU : ligne droite à vitesse constante + canal PARFAIT → reconstruction EXACTE (Hermite
    /// reproduit le linéaire) : `F ≈ 0`, aucun saut, jerk nul. Si ce test casse, l'instrument est faux.
    #[test]
    fn ligne_droite_canal_parfait_reconstruction_exacte() {
        let v = Vec3::new(2.0, 0.0, -1.0);
        let p0 = Vec3::new(5.0, 1.0, -3.0);
        let pos_vraie = |t: f64| p0 + v * (t as f32);
        // États sur la droite, à 20 Hz, canal parfait (arrivée = t_send).
        let (duree, f_tx, f_rx) = (10.0, 20.0, 60.0);
        let recus: Vec<Recu> = (0..(duree * f_tx) as usize)
            .map(|k| {
                let t = k as f64 / f_tx;
                Recu { t_arrive: t, t_send: t, pos: p0 + v * (t as f32), vel: v }
            })
            .collect();
        for d in [0.0, 0.1, 0.2] {
            let percu = reconstruire(&recus, duree, f_rx, d);
            let (f, _d_eff) = fidelite(&percu, pos_vraie, 1.0, 0.5);
            assert!(f < 1e-3, "ligne droite, canal parfait, d={d}: F doit être ~0, obtenu {f}");
            assert_eq!(n_sauts(&percu, 1.0, 0.5), 0, "aucun saut attendu sur une droite");
            // NB : on ne teste pas le jerk absolu ici — il amplifie le bruit d'arrondi f32 (÷ dt³ minuscule)
            // même sur une reconstruction exacte. La fluidité est validée RELATIVEMENT (perte → jerk monte)
            // dans `la_perte_degrade_fidelite_et_fluidite`. Ici, F ≈ 0 + 0 saut prouvent la forme exacte.
        }
    }

    /// La PERTE dégrade la fidélité ET la fluidité : à profil égal par ailleurs, 30 % de perte donne une
    /// erreur de forme et un jerk strictement pires que 0 %. (Déterministe, graine fixe.)
    #[test]
    fn la_perte_degrade_fidelite_et_fluidite() {
        let traj = Traj::Agitee;
        let (duree, f_tx, f_rx) = (20.0, 20.0, 60.0);
        let paquets = emettre(traj, duree, f_tx);
        let pos_vraie = |t: f64| echantillon_verite(traj, t).0;
        let mesure = |perte: f64| {
            let mut rng = Rng::new(0xC0FFEE);
            let p = Profil { nom: "x", latence_s: 0.019, gigue_s: 0.008, perte };
            let recus = canal(&paquets, &p, &mut rng);
            let percu = reconstruire(&recus, duree, f_rx, 0.10);
            (fidelite(&percu, pos_vraie, 2.0, 0.7).0, jerk_rms(&percu, f_rx, 2.0))
        };
        let (f0, j0) = mesure(0.0);
        let (f30, j30) = mesure(0.30);
        assert!(f30 > f0, "30 % de perte doit dégrader la forme : {f0} -> {f30}");
        assert!(j30 > j0, "30 % de perte doit augmenter le jerk : {j0} -> {j30}");
    }

    /// Le retard EFFECTIF mesuré capte bien la somme (délai d'interpolation + latence one-way) : sur un canal
    /// propre, augmenter `d_interp` augmente `d_eff` d'autant.
    #[test]
    fn le_retard_effectif_suit_le_delai_dinterpolation() {
        let traj = Traj::Calme;
        let (duree, f_tx, f_rx) = (20.0, 20.0, 60.0);
        let paquets = emettre(traj, duree, f_tx);
        let pos_vraie = |t: f64| echantillon_verite(traj, t).0;
        let mut rng = Rng::new(1);
        let p = Profil { nom: "x", latence_s: 0.020, gigue_s: 0.0, perte: 0.0 };
        let recus = canal(&paquets, &p, &mut rng);
        let d_eff = |d: f64| fidelite(&reconstruire(&recus, duree, f_rx, d), pos_vraie, 2.0, 0.7).1;
        let (a, b) = (d_eff(0.05), d_eff(0.20));
        assert!(b > a + 0.10, "d_eff doit croître avec d_interp : {a} -> {b}");
    }
}
