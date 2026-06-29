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

/// Interpolation linéaire de la POSITION seule (ordre 0 : le récepteur ignore la vitesse). Coupe les virages
/// franchement (corde du segment) → la référence « basse » contre laquelle l'ordre 1 (Hermite) doit gagner.
fn lerp_pos(a: &Snap, b: &Snap, tc: f64) -> Vec3 {
    let dt = b.t - a.t;
    if dt <= 0.0 {
        return a.pos;
    }
    let s = ((tc - a.t) / dt) as f32;
    a.pos + (b.pos - a.pos) * s
}

/// Au-delà de cet horizon (s), on cesse de FAIRE CONFIANCE à l'accélération estimée : le terme quadratique
/// `½·a·Δ²` est non borné, donc sur un TROU LONG (rafale de pertes) il s'emballe. On plafonne l'horizon du SEUL
/// terme d'accélération (la vitesse, elle, prolonge sans limite comme l'ordre 1) → la prédiction reste bornée.
/// 0,15 s ≈ 3 paquets à 20 Hz : assez pour suivre un virage, trop court pour qu'une parabole parte en vrille.
const EXTRAP_HORIZON_S: f32 = 0.15;

/// Extrapolation d'ORDRE 2 : on prolonge par la vitesse ET l'accélération `p̂ = p + v·Δ + ½·a·Δ_q²`. `accel` est
/// estimée par différence finie des deux dernières vitesses reçues (AUCUN changement du format wire — on garde
/// `(pos, vel)`). Suit les virages là où l'ordre 1 (droite) part en tangente. **Garde-fou** : l'horizon du terme
/// quadratique `Δ_q` est plafonné (`EXTRAP_HORIZON_S`) pour qu'un trou long ne fasse pas exploser la parabole ;
/// dans le régime normal (`Δ ≤ horizon`) c'est sans effet, donc les chiffres mesurés sont inchangés.
fn extrapol2(a: &Snap, accel: Vec3, tc: f64) -> Vec3 {
    let d = (tc - a.t) as f32;
    let dq = d.min(EXTRAP_HORIZON_S); // l'accélération n'est fiable que sur un court horizon
    a.pos + a.vel * d + accel * (0.5 * dq * dq)
}

/// Ordre de PRÉDICTION du récepteur (le levier que le banc balaie pour le `CONTRAT_SIDECAR`) :
/// ce que le récepteur exploite pour reconstruire entre/au-delà des snapshots.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Ordre {
    /// 0 — position SEULE : interpolation linéaire, extrapolation = on tient la position (coupe les virages).
    Zero,
    /// 1 — position + vitesse : Hermite (respecte les vitesses) + extrapolation tangente. **Comportement de réf.**
    Un,
    /// 2 — + accélération (estimée) : Hermite + extrapolation quadratique (suit la courbure en prédiction).
    Deux,
}

/// Réconciliation amortie (ressort « critically damped », façon SmoothDamp) : la position affichée POURSUIT la
/// cible **sans à-coup ni dépassement**, en ~`smooth_time` secondes. Lisse le saut d'une correction (paquet en
/// retard) → moins de saccades, au prix d'un léger lag de suivi. `smooth_time = 0` ⇒ suit instantanément (= avant).
/// `vel` est l'état de vitesse de suivi, persistant entre frames.
fn smooth_damp(courant: Vec3, cible: Vec3, vel: &mut Vec3, smooth_time: f32, dt: f32) -> Vec3 {
    if smooth_time <= 0.0 {
        *vel = Vec3::ZERO;
        return cible;
    }
    let omega = 2.0 / smooth_time;
    let x = omega * dt;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x); // approximation stable de e^(−x)
    let change = courant - cible;
    let temp = (*vel + change * omega) * dt;
    *vel = (*vel - temp * omega) * exp;
    cible + (change + temp) * exp
}

/// Reconstruit la courbe affichée `p̂(t)` à la cadence `f_rx`, avec un délai d'interpolation `d_interp` :
/// on vise l'instant `t − d_interp`, on interpole (Hermite) entre les deux snapshots encadrants DÉJÀ arrivés
/// (causalité stricte), ou on extrapole par la vitesse si le futur manque. Déterministe.
fn reconstruire(recus: &[Recu], duree: f64, f_rx: f64, d_interp: f64, smooth_time: f64, ordre: Ordre) -> Vec<(f64, Vec3)> {
    let dt = 1.0 / f_rx;
    let n_frames = (duree * f_rx) as usize;
    let mut store: Vec<Snap> = Vec::new(); // trié par t (croissant)
    let mut next = 0usize;
    let mut out = Vec::with_capacity(n_frames);
    let mut p_disp = Vec3::ZERO; // position affichée (suivie par le ressort)
    let mut vel_suivi = Vec3::ZERO; // état du ressort
    let mut amorce = false;

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
        // La CIBLE brute (interpolation/extrapolation), avant lissage.
        let cible = if store.is_empty() {
            Vec3::ZERO
        } else {
            let k = store.partition_point(|s| s.t <= tc); // 1er snapshot avec t > tc
            if k == 0 {
                store[0].pos // cible avant le 1er snapshot connu : on tient la position de départ
            } else if k < store.len() {
                // encadré : interpolation (ordre 0 = corde linéaire ; ordres 1/2 = Hermite, exacte pour le segment).
                match ordre {
                    Ordre::Zero => lerp_pos(&store[k - 1], &store[k], tc),
                    Ordre::Un | Ordre::Deux => hermite(&store[k - 1], &store[k], tc),
                }
            } else {
                // futur pas encore arrivé : EXTRAPOLATION — c'est ici que l'ordre de prédiction pèse vraiment.
                match ordre {
                    Ordre::Zero => store[k - 1].pos, // tient la position (pas de vitesse connue)
                    Ordre::Un => extrapol(&store[k - 1], tc), // tangente (vitesse)
                    Ordre::Deux => {
                        // Accélération par différence finie des deux dernières vitesses reçues (sinon 0 → ordre 1).
                        let accel = if k >= 2 {
                            let dtv = store[k - 1].t - store[k - 2].t;
                            if dtv > 0.0 {
                                (store[k - 1].vel - store[k - 2].vel) / dtv as f32
                            } else {
                                Vec3::ZERO
                            }
                        } else {
                            Vec3::ZERO
                        };
                        extrapol2(&store[k - 1], accel, tc)
                    }
                }
            }
        };
        // Réconciliation amortie : la position affichée poursuit la cible (lisse les sauts de correction).
        let pos = if smooth_time <= 0.0 || store.is_empty() {
            p_disp = cible;
            cible
        } else {
            if !amorce {
                p_disp = cible; // démarrer SUR la cible (pas de rampe depuis ZERO)
                amorce = true;
            }
            p_disp = smooth_damp(p_disp, cible, &mut vel_suivi, smooth_time as f32, dt as f32);
            p_disp
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

/// Exporte les deux courbes (vérité + perçu, à chaque frame) en CSV — pour les VISUALISER (tableur/grapheur).
fn ecrire_csv(chemin: &str, percu: &[(f64, Vec3)], traj: Traj) -> std::io::Result<()> {
    use std::io::Write;
    let mut f = std::fs::File::create(chemin)?;
    writeln!(f, "t,vrai_x,vrai_y,vrai_z,percu_x,percu_y,percu_z")?;
    for &(t, p) in percu {
        let v = echantillon_verite(traj, t).0;
        writeln!(f, "{t:.4},{:.4},{:.4},{:.4},{:.4},{:.4},{:.4}", v.x, v.y, v.z, p.x, p.y, p.z)?;
    }
    Ok(())
}

/// Génère un graphe SVG (sans aucune dépendance) : la composante X au cours du temps, vérité (noir) vs chaque
/// série perçue, sur la fenêtre `[t0, t1]`. Un SVG s'ouvre d'un double-clic (navigateur) ET s'affiche sur GitHub.
fn ecrire_svg(chemin: &str, traj: Traj, series: &[(&str, &str, &[(f64, Vec3)])], t0: f64, t1: f64) -> std::io::Result<()> {
    let (w, h) = (920.0_f64, 380.0_f64);
    let (ml, mr, mt, mb) = (56.0_f64, 180.0_f64, 34.0_f64, 42.0_f64);
    let (pw, ph) = (w - ml - mr, h - mt - mb);
    let comp = |v: Vec3| v.x as f64; // on trace la composante X (le retard et les saccades s'y voient bien)

    // Vérité échantillonnée finement sur la fenêtre.
    let nv = 600usize;
    let verite: Vec<(f64, f64)> = (0..=nv)
        .map(|i| { let t = t0 + (t1 - t0) * i as f64 / nv as f64; (t, comp(echantillon_verite(traj, t).0)) })
        .collect();
    let (mut ymin, mut ymax) = (f64::INFINITY, f64::NEG_INFINITY);
    for &(_, y) in &verite {
        ymin = ymin.min(y);
        ymax = ymax.max(y);
    }
    for (_, _, pts) in series {
        for &(t, p) in *pts {
            if t >= t0 && t <= t1 {
                let y = comp(p);
                ymin = ymin.min(y);
                ymax = ymax.max(y);
            }
        }
    }
    let pad = (ymax - ymin) * 0.08 + 1e-6;
    ymin -= pad;
    ymax += pad;
    let sx = |t: f64| ml + (t - t0) / (t1 - t0) * pw;
    let sy = |y: f64| mt + (1.0 - (y - ymin) / (ymax - ymin)) * ph;
    let poly = |pts: &[(f64, f64)]| pts.iter().map(|&(t, y)| format!("{:.1},{:.1}", sx(t), sy(y))).collect::<Vec<_>>().join(" ");

    let mut s = String::new();
    s.push_str(&format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" font-family=\"sans-serif\" font-size=\"13\">\n"));
    s.push_str(&format!("<rect width=\"{w}\" height=\"{h}\" fill=\"white\"/>\n"));
    s.push_str(&format!("<text x=\"{ml}\" y=\"20\" fill=\"#333\" font-weight=\"bold\">Banc vivant — X(t) : verite vs percu (4G congestionne, d=100 ms)</text>\n"));
    // Axes.
    s.push_str(&format!("<line x1=\"{ml}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke=\"#999\"/>\n", mt + ph, ml + pw, mt + ph));
    s.push_str(&format!("<line x1=\"{ml}\" y1=\"{mt}\" x2=\"{ml}\" y2=\"{}\" stroke=\"#999\"/>\n", mt + ph));
    s.push_str(&format!("<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"#555\">temps (s)</text>\n", ml + pw / 2.0, h - 10.0));
    // Vérité (noir, épais) puis chaque série.
    s.push_str(&format!("<polyline fill=\"none\" stroke=\"#222\" stroke-width=\"2.6\" points=\"{}\"/>\n", poly(&verite)));
    let mut ly = mt + 6.0;
    s.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"#222\">— verite</text>\n", ml + pw + 14.0, ly));
    for (label, col, pts) in series {
        let p: Vec<(f64, f64)> = pts.iter().filter(|&&(t, _)| t >= t0 && t <= t1).map(|&(t, v)| (t, comp(v))).collect();
        s.push_str(&format!("<polyline fill=\"none\" stroke=\"{col}\" stroke-width=\"1.6\" points=\"{}\"/>\n", poly(&p)));
        ly += 22.0;
        s.push_str(&format!("<text x=\"{}\" y=\"{}\" fill=\"{col}\">— {label}</text>\n", ml + pw + 14.0, ly));
    }
    s.push_str("</svg>\n");
    std::fs::write(chemin, s)
}

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
    let verdict_de = |f: f64, j: f64, sauts: usize, d_eff: f64| -> &'static str {
        if sauts > 0 || j > j_ok {
            "✗ saccadé"
        } else if f > f_ok {
            "✗ flou"
        } else if d_eff > 0.5 + 1e-6 {
            "✗ en retard"
        } else {
            "✓ vivant"
        }
    };

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
            let percu = reconstruire(&recus, duree, f_rx, d, 0.0, Ordre::Un);
            let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.7);
            let j = jerk_rms(&percu, f_rx, warmup);
            let sauts = n_sauts(&percu, warmup, seuil_saut);
            let verdict = verdict_de(f, j, sauts, d_eff);
            println!(
                "   {:>7.0}ms | {:>6.0}ms | {:>10.2} | {:>10.1} | {:>5} | {}",
                d * 1000.0, d_eff * 1000.0, f * 100.0, j, sauts, verdict
            );
        }
        println!();
    }
    // ====== Effet du RESSORT de réconciliation sur le cas DUR (4G congestionné) ======
    let dur = Profil { nom: "4G congestionné", latence_s: 0.030, gigue_s: 0.030, perte: 0.08 };
    let mut rng = Rng::new(seed ^ (dur.nom.len() as u64).wrapping_mul(0x9E3779B97F4A7C15));
    let recus_dur = canal(&paquets, &dur, &mut rng);
    println!("── 🌿 RESSORT de réconciliation sur le cas dur ({}) : reste-t-on vivant à plus BAS délai ?", dur.nom);
    println!(
        "   {:>9} | {:>9} | {:>8} | {:>10} | {:>10} | {}",
        "d_interp", "ressort", "d_eff", "F (cm)", "J (m/s³)", "verdict"
    );
    for &d in &[0.05, 0.10, 0.15] {
        for &st in &[0.0, 0.03, 0.06, 0.10] {
            let percu = reconstruire(&recus_dur, duree, f_rx, d, st, Ordre::Un);
            let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.9);
            let j = jerk_rms(&percu, f_rx, warmup);
            let sauts = n_sauts(&percu, warmup, seuil_saut);
            println!(
                "   {:>7.0}ms | {:>7.0}ms | {:>6.0}ms | {:>10.2} | {:>10.1} | {}",
                d * 1000.0, st * 1000.0, d_eff * 1000.0, f * 100.0, j, verdict_de(f, j, sauts, d_eff)
            );
        }
    }
    println!();

    // ====== Effet de l'ORDRE DE PRÉDICTION (0/1/2) à BAS délai sur les liens 4G ======
    // Hypothèse à tester (PLAN_TEST_VIVANT §RESTE) : prédire avec l'accélération (ordre 2) suit mieux les virages
    // là où le ressort FLOUTE → MEILLEUR F et/ou MOINS de saccades à délai réduit, sans le retard ajouté du ressort.
    println!("── 🔮 ORDRE DE PRÉDICTION (0=pos · 1=+vitesse · 2=+accél.) — suit-on mieux les virages à BAS délai ?");
    println!(
        "   {:>16} | {:>9} | {:>5} | {:>8} | {:>10} | {:>10} | {}",
        "profil", "d_interp", "ordre", "d_eff", "F (cm)", "J (m/s³)", "verdict"
    );
    let durs = [
        Profil { nom: "4G correct", latence_s: 0.019, gigue_s: 0.008, perte: 0.03 },
        Profil { nom: "4G congestionné", latence_s: 0.030, gigue_s: 0.030, perte: 0.08 },
    ];
    for p in durs {
        let mut rng = Rng::new(seed ^ (p.nom.len() as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let recus = canal(&paquets, &p, &mut rng);
        for &d in &[0.0, 0.05, 0.10, 0.15] {
            for (lbl, ordre) in [("0", Ordre::Zero), ("1", Ordre::Un), ("2", Ordre::Deux)] {
                let percu = reconstruire(&recus, duree, f_rx, d, 0.0, ordre);
                let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.7);
                let j = jerk_rms(&percu, f_rx, warmup);
                let sauts = n_sauts(&percu, warmup, seuil_saut);
                println!(
                    "   {:>16} | {:>7.0}ms | {:>5} | {:>6.0}ms | {:>10.2} | {:>10.1} | {}",
                    p.nom, d * 1000.0, lbl, d_eff * 1000.0, f * 100.0, j, verdict_de(f, j, sauts, d_eff)
                );
            }
        }
        println!();
    }

    // ====== ORDRE 2 + ressort léger : franchit-on le « ✓ vivant » SOUS 150 ms sur le cas dur ? ======
    // L'ordre 2 prédit déjà bien (corrections petites) → un ressort LÉGER ne « coupe » presque plus les virages :
    // on cherche le couple (d_interp, ressort) le plus FRAIS qui passe vivant, là où chaque levier SEUL échouait.
    println!("── 🔗 ORDRE 2 + ressort léger (cas dur 4G congestionné) — le plus BAS d_eff qui reste « ✓ vivant »");
    println!(
        "   {:>9} | {:>9} | {:>8} | {:>10} | {:>10} | {}",
        "d_interp", "ressort", "d_eff", "F (cm)", "J (m/s³)", "verdict"
    );
    for &d in &[0.08, 0.10] {
        for &st in &[0.0, 0.02, 0.03, 0.05] {
            let percu = reconstruire(&recus_dur, duree, f_rx, d, st, Ordre::Deux);
            let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.9);
            let j = jerk_rms(&percu, f_rx, warmup);
            let sauts = n_sauts(&percu, warmup, seuil_saut);
            println!(
                "   {:>7.0}ms | {:>7.0}ms | {:>6.0}ms | {:>10.2} | {:>10.1} | {}",
                d * 1000.0, st * 1000.0, d_eff * 1000.0, f * 100.0, j, verdict_de(f, j, sauts, d_eff)
            );
        }
    }
    println!();

    // ====== Effet de la CADENCE d'émission f_tx (ordre 2, cas dur) — vaut-il le coût en octets reçus ? ======
    // Plus de paquets = extrapolation plus COURTE + meilleure estimation d'accélération → plus fidèle. MAIS le coût
    // est linéaire en octets REÇUS (le mur D3 du budget de réception). On cherche la f_tx la plus BASSE qui passe.
    println!("── 📡 CADENCE f_tx (ordre 2, 4G congestionné, d_interp=100 ms) — la plus BASSE qui reste « ✓ vivant »");
    println!("   {:>6} | {:>8} | {:>10} | {:>10} | {}", "f_tx", "d_eff", "F (cm)", "J (m/s³)", "verdict");
    for &ftx in &[10.0_f64, 20.0, 30.0] {
        let paq = emettre(traj, duree, ftx);
        let mut rng = Rng::new(seed ^ (dur.nom.len() as u64).wrapping_mul(0x9E3779B97F4A7C15));
        let recus = canal(&paq, &dur, &mut rng);
        let percu = reconstruire(&recus, duree, f_rx, 0.10, 0.0, Ordre::Deux);
        let (f, d_eff) = fidelite(&percu, pos_vraie, warmup, 0.9);
        let j = jerk_rms(&percu, f_rx, warmup);
        let sauts = n_sauts(&percu, warmup, seuil_saut);
        println!(
            "   {:>4.0}Hz | {:>6.0}ms | {:>10.2} | {:>10.1} | {}",
            ftx, d_eff * 1000.0, f * 100.0, j, verdict_de(f, j, sauts, d_eff)
        );
    }
    println!();

    // ====== Export des courbes (cas dur, d_interp = 100 ms) pour les VISUALISER — les 3 régimes ======
    let sans = reconstruire(&recus_dur, duree, f_rx, 0.10, 0.0, Ordre::Un); // ordre 1 brut → saccadé
    let avec = reconstruire(&recus_dur, duree, f_rx, 0.10, 0.06, Ordre::Un); // ordre 1 + ressort fort → flou
    let ord2 = reconstruire(&recus_dur, duree, f_rx, 0.10, 0.0, Ordre::Deux); // ordre 2 → colle la vérité
    let _ = ecrire_csv("vivant_courbes_sans_ressort.csv", &sans, traj);
    let _ = ecrire_csv("vivant_courbes_avec_ressort.csv", &avec, traj);
    let _ = ecrire_csv("vivant_courbes_ordre2.csv", &ord2, traj);
    let _ = ecrire_svg(
        "vivant_courbes.svg",
        traj,
        &[
            ("ordre 1 brut (rouge, saccadé)", "#d62728", &sans),
            ("ordre 1 + ressort (bleu, flou)", "#1f77b4", &avec),
            ("ordre 2 (vert, fidèle)", "#2ca02c", &ord2),
        ],
        4.0,
        9.0,
    );
    println!("Courbes exportées (4G congestionné, d_interp = 100 ms) :");
    println!("  • vivant_courbes.svg → double-clic (navigateur) : vérité (noir) · ordre 1 brut (rouge) · +ressort (bleu) · ordre 2 (vert).");
    println!("  • vivant_courbes_{{sans,avec}}_ressort.csv / _ordre2.csv → données (séparateur VIRGULE).\n");

    println!("Lecture : le BON d_interp est le plus PETIT qui reste « ✓ vivant ». L'ordre 2 (prédiction par");
    println!("l'accélération) baisse À LA FOIS l'erreur de forme ET le jerk (~6× à bas délai) sans ajouter de retard ;");
    println!("ordre 2 + un ressort LÉGER passe « vivant » dès ~100 ms d_eff sur le 4G congestionné (cf. prive/PLAN_TEST_VIVANT.md).");
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
            let percu = reconstruire(&recus, duree, f_rx, d, 0.0, Ordre::Un);
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
            let percu = reconstruire(&recus, duree, f_rx, 0.10, 0.0, Ordre::Un);
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
        let d_eff = |d: f64| fidelite(&reconstruire(&recus, duree, f_rx, d, 0.0, Ordre::Un), pos_vraie, 2.0, 0.7).1;
        let (a, b) = (d_eff(0.05), d_eff(0.20));
        assert!(b > a + 0.10, "d_eff doit croître avec d_interp : {a} -> {b}");
    }

    /// Le ressort converge vers la cible **sans dépasser** (critically damped) ; `smooth_time = 0` suit
    /// instantanément. Garde-fou de l'instrument de lissage.
    #[test]
    fn ressort_converge_sans_depasser() {
        let mut v = Vec3::ZERO;
        assert_eq!(smooth_damp(Vec3::ZERO, Vec3::X, &mut v, 0.0, 0.016), Vec3::X);
        let cible = Vec3::new(10.0, 0.0, 0.0);
        let (mut p, mut vel, mut prev) = (Vec3::ZERO, Vec3::ZERO, -1.0_f32);
        for _ in 0..600 {
            p = smooth_damp(p, cible, &mut vel, 0.1, 1.0 / 60.0);
            assert!(p.x <= 10.0 + 1e-3, "pas de dépassement : {}", p.x);
            assert!(p.x >= prev - 1e-4, "approche monotone");
            prev = p.x;
        }
        assert!((p.x - 10.0).abs() < 1e-2, "converge vers la cible, obtenu {}", p.x);
    }

    /// GARDE-FOU ORDRE 2 : sur une trajectoire à **accélération CONSTANTE** (parabole), en régime d'EXTRAPOLATION
    /// pure (latence seule → le futur n'est jamais encore arrivé), l'ordre 2 (qui prédit avec l'accélération
    /// estimée) reproduit la courbe quasi exactement, là où l'ordre 1 (tangente) garde une erreur résiduelle.
    /// Prouve que le mécanisme d'ordre 2 capture bien la courbure. (Déterministe.)
    #[test]
    fn ordre2_suit_une_acceleration_constante_mieux_que_ordre1() {
        let p0 = Vec3::new(0.0, 0.0, 0.0);
        let v0 = Vec3::new(1.0, 0.0, 0.0);
        let acc = Vec3::new(0.0, 0.0, 2.0); // accélère selon z
        let pos_vraie = |t: f64| p0 + v0 * (t as f32) + acc * (0.5 * (t * t) as f32);
        let vel_vraie = |t: f64| v0 + acc * (t as f32);
        let (duree, f_tx, f_rx, lat) = (8.0, 20.0, 60.0, 0.08);
        // Canal à latence PURE (pas de perte/gigue) → tc = t devance toujours le dernier snapshot → extrapolation.
        let recus: Vec<Recu> = (0..(duree * f_tx) as usize)
            .map(|k| {
                let t = k as f64 / f_tx;
                Recu { t_arrive: t + lat, t_send: t, pos: pos_vraie(t), vel: vel_vraie(t) }
            })
            .collect();
        let f_de = |ordre| fidelite(&reconstruire(&recus, duree, f_rx, 0.0, 0.0, ordre), pos_vraie, 1.0, 0.3).0;
        let (f1, f2) = (f_de(Ordre::Un), f_de(Ordre::Deux));
        assert!(f1 > 1e-3, "garde-fou : l'ordre 1 doit garder une erreur sur une parabole, obtenu {f1}");
        assert!(f2 < f1 * 0.2, "l'ordre 2 doit suivre l'accélération bien mieux que l'ordre 1 : {f1} -> {f2}");
    }

    /// GARDE-FOU clamp : sur un TROU LONG (extrapolation très loin), le terme d'accélération est PLAFONNÉ
    /// (`½·a·horizon²`) pour ne pas s'emballer, tandis que le terme de vitesse reste linéaire (comme l'ordre 1).
    #[test]
    fn extrapol2_borne_le_terme_quadratique_sur_trou_long() {
        let a = Snap { t: 0.0, pos: Vec3::ZERO, vel: Vec3::new(1.0, 0.0, 0.0) };
        let accel = Vec3::new(0.0, 0.0, 100.0); // grosse accélération estimée
        let p = extrapol2(&a, accel, 2.0); // 2 s dans le futur : bien au-delà de l'horizon
        let borne = 0.5 * 100.0 * EXTRAP_HORIZON_S * EXTRAP_HORIZON_S;
        assert!((p.z - borne).abs() < 1e-3, "terme accel plafonné à {borne}, obtenu {}", p.z);
        assert!((p.x - 2.0).abs() < 1e-3, "le terme de vitesse reste linéaire (non plafonné) : {}", p.x);
    }

    /// Le ressort LISSE les saccades : à bas délai sur un lien lossy, activer la réconciliation réduit le jerk.
    #[test]
    fn le_ressort_lisse_les_saccades() {
        let traj = Traj::Agitee;
        let (duree, f_tx, f_rx) = (20.0, 20.0, 60.0);
        let paquets = emettre(traj, duree, f_tx);
        let p = Profil { nom: "x", latence_s: 0.030, gigue_s: 0.030, perte: 0.08 };
        let mut rng = Rng::new(0xABCDEF);
        let recus = canal(&paquets, &p, &mut rng);
        let jerk = |st: f64| jerk_rms(&reconstruire(&recus, duree, f_rx, 0.05, st, Ordre::Un), f_rx, 2.0);
        let (sans, avec) = (jerk(0.0), jerk(0.06));
        assert!(avec < sans, "le ressort doit réduire le jerk : {sans} -> {avec}");
    }
}
