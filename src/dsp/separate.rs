//! SÉPARATION DE SOURCES — le « calcul autonome » qui ÉNUMÈRE les bruits, les ISOLE, et n'en retire QUE les cochés.
//! (idée utilisateur, nuit du 30 juin → 1er juillet 2026 — pousse `prive/PLAN_TEST_VOIX.md` §1.8 plus loin.)
//!
//! Le principe, en clair : pas un débruitage « à l'aveugle » qui rase tout (ça, c'est Krisp, interdit). Un calcul
//! qui DÉCOUPE le son en sources distinctes — **bruit 1, bruit 2, bruit 3…** — chacune SÉPARÉE (obligatoire), que
//! l'utilisateur peut **auditionner isolément AVANT de cocher** (on retire tout ce qui touche aux autres bruits + la
//! voix, il entend clairement LE bruit qui sera découpé), puis **cocher** : seul le coché part, la voix ET les
//! bruits non-cochés restent INTACTS (s'il a fait un son exprès, c'est son droit de le garder). **Zéro IA** pour
//! nommer (pour l'instant) : on caractérise chaque source par des descripteurs OBJECTIFS (nature, fréquence,
//! période), l'oreille de l'utilisateur (+ l'audition) fait le reste.
//!
//! Tout white-box, sur le socle `fft.rs`. La décomposition par trame `t`, bin `k` :
//!   - partie STATIONNAIRE `S = min(|X|, plancher[k])` (les bruits constants : ventilo, souffle) ;
//!   - EXCÈS `E = (|X| − plancher[k])₊` (les transitoires + la voix qui passent au-dessus du plancher).
//! Trois familles de sources : **tonale** (raies persistantes regroupées par harmoniques), **large bande** (le
//! plancher plat), **transitoire-périodique** (pics brefs et réguliers de l'excès — les clics). La VOIX = le reste.
//! Les masques forment une PARTITION DE L'UNITÉ (`Σ = 1` là où `|X|>0`) → `isoler(toutes) + voix = original` à
//! l'epsilon près (invariant testé). On garde la phase, la reconstruction overlap-add est exacte (COLA).

use super::fft::{hann, istft, stft, Cplx};

// ---- Réglages (white-box, tous documentés ; le banc dira s'ils généralisent) -------------------------------------
const PERCENTILE_PLANCHER: f64 = 0.10; // 10e percentile par bin = niveau stationnaire (cf. denoise.rs)
const TONAL_RATIO: f64 = 3.0; // une raie = plancher > 3× la ligne de base locale (fenêtre large)
const TONAL_BASE_W: usize = 12; // demi-fenêtre de la LIGNE DE BASE (assez large pour ignorer les raies étroites)
const TONAL_SKIRT: f64 = 1.5; // on étend une raie tant que le plancher > 1.5× la base (capte la jupe proche)
const TONAL_SKIRT_MAX: usize = 2; // jupe BORNÉE (± 2 bins) — ne pas dévorer le large bande voisin
const HARM_TOL_BINS: f64 = 0.5; // un harmonique = position ≈ multiple entier du fondamental, à ½ bin près
const BB_FRAC_MIN: f64 = 0.003; // on ne crée une source que si elle pèse > 0.3 % de l'énergie totale
const CLIC_HF_HZ: f64 = 1500.0; // les transitoires/clics ont du contenu HAUTE-FRÉQUENCE ; la voix voisée non
const CLIC_TAU_MIN_S: f64 = 0.08; // période minimale cherchée (≤ ~12 clics/s)
const CLIC_R_MIN: f64 = 0.15; // autocorrélation HF normalisée : un pic > 0.15 ⇒ VRAIMENT périodique (robuste au bruit)
const CLIC_PEAK_FRAC: f64 = 0.8; // période = plus petit τ atteignant 80 % du pic (le FONDAMENTAL, pas un multiple)
const CLIC_MIN: usize = 3; // au moins 3 occurrences marquées pour créer la source

fn n_bins_uniques(n: usize) -> usize {
    n / 2 + 1
}

// ---- Helpers spectraux -------------------------------------------------------------------------------------------

/// Spectrogramme d'amplitude `A[t][k] = |X[t][k]|` (bins uniques `0..m`).
fn magnitudes(sp: &[Vec<Cplx>], m: usize) -> Vec<Vec<f64>> {
    sp.iter()
        .map(|fx| (0..m).map(|k| (fx[k].re * fx[k].re + fx[k].im * fx[k].im).sqrt()).collect())
        .collect()
}

/// Plancher de bruit par bin = `p`-ième percentile de `A[·][k]` sur le temps (la voix est intermittente → le bas
/// percentile capte le niveau STATIONNAIRE : ventilo, souffle).
fn plancher(a: &[Vec<f64>], m: usize, p: f64) -> Vec<f64> {
    let mut pl = vec![0.0_f64; m];
    for (k, plk) in pl.iter_mut().enumerate() {
        let mut col: Vec<f64> = a.iter().map(|row| row[k]).collect();
        col.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let idx = ((p * col.len().saturating_sub(1) as f64).round() as usize).min(col.len().saturating_sub(1));
        *plk = col.get(idx).copied().unwrap_or(0.0);
    }
    pl
}

fn mediane(mut v: Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

// ---- Détection des raies TONALES (regroupées par harmoniques) ----------------------------------------------------

struct GroupeTonal {
    bins: Vec<usize>, // tous les bins de la source (fondamental + harmoniques + jupe)
    f0_hz: f64,       // fréquence fondamentale (sous-bin, par interpolation parabolique)
    n_harm: usize,    // nombre d'harmoniques détectés (fondamental compris)
}

/// Médiane du plancher sur une fenêtre large autour de `k` = la LIGNE DE BASE locale (ignore les raies étroites).
fn base_locale(pl: &[f64], k: usize, m: usize) -> f64 {
    let lo = k.saturating_sub(TONAL_BASE_W);
    let hi = (k + TONAL_BASE_W).min(m - 1);
    mediane((lo..=hi).map(|j| pl[j]).collect()).max(1e-12)
}

/// Décalage sous-bin d'un pic par interpolation parabolique sur (gauche, centre, droite). Dans [−0.5, 0.5].
fn decalage_parabolique(yl: f64, yc: f64, yr: f64) -> f64 {
    let d = yl - 2.0 * yc + yr;
    if d.abs() < 1e-12 {
        0.0
    } else {
        (0.5 * (yl - yr) / d).clamp(-0.5, 0.5)
    }
}

fn detecter_tonales(pl: &[f64], m: usize, sr: f64, n: usize) -> Vec<GroupeTonal> {
    // 1) Pics proéminents (plancher > 3× la ligne de base large) ET maxima locaux.
    let mut pics: Vec<usize> = Vec::new();
    for k in 2..m.saturating_sub(1) {
        if pl[k] > TONAL_RATIO * base_locale(pl, k, m) && pl[k] >= pl[k - 1] && pl[k] >= pl[k + 1] {
            pics.push(k);
        }
    }
    // 2) Fusionner les pics adjacents (≤ 2 bins = la même raie étalée par la fenêtre) → garder le plus fort.
    let mut fusion: Vec<usize> = Vec::new();
    for &p in &pics {
        if let Some(&last) = fusion.last() {
            if p - last <= 2 {
                if pl[p] > pl[last] {
                    *fusion.last_mut().unwrap() = p;
                }
                continue;
            }
        }
        fusion.push(p);
    }
    // 3) Position FRACTIONNAIRE (interpolation parabolique) de chaque raie → fréquence sous-bin.
    let pos: Vec<f64> = fusion
        .iter()
        .map(|&k| k as f64 + decalage_parabolique(pl[k - 1], pl[k], if k + 1 < m { pl[k + 1] } else { pl[k] }))
        .collect();
    // 4) Regroupement harmonique sur les positions FRACTIONNAIRES (un harmonique = position ≈ multiple du f0).
    let mut utilise = vec![false; fusion.len()];
    let mut groupes: Vec<GroupeTonal> = Vec::new();
    for i in 0..fusion.len() {
        if utilise[i] {
            continue;
        }
        // On suppose la plus basse raie ≈ fondamental, on attribue à chaque autre son rang harmonique entier, puis
        // on RAFFINE f0 par moindres carrés sur (pos ≈ h·f0) — robuste à l'imprécision sous-bin du grave.
        let mut cand: Vec<(usize, f64)> = vec![(i, 1.0)];
        for j in (i + 1)..fusion.len() {
            if utilise[j] {
                continue;
            }
            let h = (pos[j] / pos[i]).round();
            if h >= 2.0 {
                cand.push((j, h));
            }
        }
        let num: f64 = cand.iter().map(|&(j, h)| h * pos[j]).sum();
        let den: f64 = cand.iter().map(|&(_, h)| h * h).sum();
        let f0 = if den > 0.0 { num / den } else { pos[i] };
        let mut membres: Vec<usize> = Vec::new();
        for &(j, h) in &cand {
            if (pos[j] - h * f0).abs() <= HARM_TOL_BINS {
                utilise[j] = true;
                membres.push(j);
            }
        }
        if membres.is_empty() {
            utilise[i] = true;
            membres.push(i);
        }
        // 5) Bins de la source = la JUPE PROCHE et BORNÉE de chaque membre (tant que le plancher domine la base,
        //    au plus ± TONAL_SKIRT_MAX bins → on ne dévore pas le large bande voisin).
        let mut bins: Vec<usize> = Vec::new();
        for &mi in &membres {
            let p = fusion[mi];
            let base = base_locale(pl, p, m);
            bins.push(p);
            for step in 1..=TONAL_SKIRT_MAX {
                if p >= step && pl[p - step] > TONAL_SKIRT * base {
                    bins.push(p - step);
                } else {
                    break;
                }
            }
            for step in 1..=TONAL_SKIRT_MAX {
                if p + step < m && pl[p + step] > TONAL_SKIRT * base {
                    bins.push(p + step);
                } else {
                    break;
                }
            }
        }
        bins.sort_unstable();
        bins.dedup();
        groupes.push(GroupeTonal { bins, f0_hz: f0 * sr / n as f64, n_harm: membres.len() });
    }
    groupes
}

// ---- Détection des transitoires PÉRIODIQUES (les clics) ----------------------------------------------------------

/// Renvoie (trames-clic, période en s, bin de coupure HF) si un train de pics BREFS et RÉGULIERS existe dans le
/// FLUX SPECTRAL en haute fréquence — là où vivent les transitoires (clics) et PAS la voix voisée (concentrée en bas).
/// C'est ce qui sépare les clics du souffle (stationnaire → flux ≈ 0) et de la voix (peu de HF).
fn detecter_transitoire(
    a: &[Vec<f64>],
    est_tonal: &[bool],
    m: usize,
    hop: usize,
    sr: f64,
) -> Option<(Vec<bool>, f64, usize)> {
    let t = a.len();
    if t < 8 {
        return None;
    }
    let n = (m - 1) * 2;
    let hf0 = ((CLIC_HF_HZ * n as f64 / sr).round() as usize).min(m - 1);
    // Enveloppe = flux spectral POSITIF en HF (montée soudaine d'énergie) sur les bins non-tonals.
    let mut env = vec![0.0_f64; t];
    for i in 1..t {
        env[i] = (hf0..m)
            .filter(|&k| !est_tonal[k])
            .map(|k| (a[i][k] - a[i - 1][k]).max(0.0))
            .sum();
    }
    // AUTOCORRÉLATION normalisée de l'enveloppe (centrée) : intègre sur TOUT le signal → un train périodique
    // ressort même quand chaque pic individuel est noyé dans un plancher de bruit élevé.
    let moy = env.iter().sum::<f64>() / t as f64;
    let centre: Vec<f64> = env.iter().map(|&x| x - moy).collect();
    let n0: f64 = centre.iter().map(|&x| x * x).sum();
    if n0 < 1e-12 {
        return None;
    }
    let tau_min = ((CLIC_TAU_MIN_S * sr / hop as f64).round() as usize).max(2);
    let tau_max = t / 2;
    if tau_max <= tau_min {
        return None;
    }
    let r: Vec<f64> = (tau_min..=tau_max)
        .map(|tau| (0..t - tau).map(|i| centre[i] * centre[i + tau]).sum::<f64>() / n0)
        .collect();
    let r_max = r.iter().cloned().fold(0.0_f64, f64::max);
    if r_max < CLIC_R_MIN {
        return None; // pas de périodicité franche → pas de source transitoire
    }
    // Période = le PLUS PETIT τ atteignant 80 % du pic (le fondamental, pas un multiple).
    let tau = tau_min + r.iter().position(|&x| x >= CLIC_PEAK_FRAC * r_max).unwrap_or(0);

    // Trames-clic (pour le masque) = maxima locaux de l'enveloppe au-dessus de moy + 1.5σ.
    let sigma = (n0 / t as f64).sqrt();
    let seuil = moy + 1.5 * sigma;
    let mut clic = vec![false; t];
    let mut nb = 0;
    for i in 1..t - 1 {
        if env[i] >= seuil && env[i] >= env[i - 1] && env[i] >= env[i + 1] {
            clic[i] = true;
            nb += 1;
        }
    }
    if nb < CLIC_MIN {
        return None;
    }
    Some((clic, tau as f64 * hop as f64 / sr, hf0))
}

// ---- Le modèle de séparation -------------------------------------------------------------------------------------

/// Une source de bruit isolée (un « bruit N »). `masque[t][k] ∈ [0,1]` = fraction de `|X[t][k]|` qui lui appartient.
pub struct Source {
    pub nature: &'static str, // "tonale" | "large bande" | "transitoire périodique"
    pub descriptif: String,   // ex. "tonale ~125 Hz (2 harm.)" — objectif, PAS un nom d'IA
    pub energie_frac: f64,    // part de l'énergie totale (tri décroissant → bruit 1, 2, 3…)
    pub bulle_pct: f64,       // rayon d'audibilité du bruit / rayon de la voix (% — petit = confiné)
    masque: Vec<Vec<f64>>,
}

/// Le résultat : la liste des bruits + le masque de la voix. Sait isoler/retirer en réappliquant les masques.
pub struct Separation {
    pub sources: Vec<Source>,
    voix: Vec<Vec<f64>>,
    n: usize,
    hop: usize,
}

/// Applique un masque (bins `0..m`) à la STFT de `signal` et reconstruit (phase gardée, symétrie conjuguée, ISTFT).
fn appliquer(signal: &[f32], masque: &[Vec<f64>], n: usize, hop: usize) -> Vec<f32> {
    let win = hann(n);
    let m = n_bins_uniques(n);
    let sp = stft(signal, n, hop, &win);
    let mut rec: Vec<Vec<Cplx>> = Vec::with_capacity(sp.len());
    for (t, fx) in sp.iter().enumerate() {
        let mut out = vec![Cplx::new(0.0, 0.0); n];
        for k in 0..m {
            let g = masque.get(t).and_then(|r| r.get(k)).copied().unwrap_or(0.0);
            out[k] = Cplx::new(fx[k].re * g, fx[k].im * g);
        }
        for k in 1..n / 2 {
            out[n - k] = Cplx::new(out[k].re, -out[k].im);
        }
        rec.push(out);
    }
    istft(&rec, n, hop, signal.len())
}

impl Separation {
    /// AUDITION : on entend UNIQUEMENT le bruit `k` (tout le reste retiré) → l'utilisateur sait ce que c'est.
    pub fn isoler(&self, signal: &[f32], k: usize) -> Vec<f32> {
        appliquer(signal, &self.sources[k].masque, self.n, self.hop)
    }

    /// La voix seule (le reste après tous les bruits) — pour vérifier qu'on ne l'a pas abîmée.
    pub fn voix(&self, signal: &[f32]) -> Vec<f32> {
        appliquer(signal, &self.voix, self.n, self.hop)
    }

    /// RETRAIT : on enlève SEULEMENT les bruits cochés (`set`) ; la voix et les bruits non-cochés restent intacts.
    pub fn retirer(&self, signal: &[f32], set: &[usize]) -> Vec<f32> {
        let frames = self.voix.len();
        let m = n_bins_uniques(self.n);
        let mut garder = vec![vec![1.0_f64; m]; frames]; // 1 − Σ masques cochés
        for &s in set {
            for (t, row) in self.sources[s].masque.iter().enumerate() {
                for (k, &g) in row.iter().enumerate() {
                    garder[t][k] = (garder[t][k] - g).max(0.0);
                }
            }
        }
        appliquer(signal, &garder, self.n, self.hop)
    }
}

/// LE CALCUL AUTONOME : analyse `signal`, énumère les bruits (sources), construit leurs masques + celui de la voix.
pub fn analyser(signal: &[f32], n: usize, hop: usize, sr: f64) -> Separation {
    let win = hann(n);
    let m = n_bins_uniques(n);
    let sp = stft(signal, n, hop, &win);
    let a = magnitudes(&sp, m);
    let t = a.len();
    let pl = plancher(&a, m, PERCENTILE_PLANCHER);

    // Détections.
    let tonales = detecter_tonales(&pl, m, sr, n);
    let mut est_tonal = vec![false; m];
    let mut groupe_de_bin = vec![usize::MAX; m];
    for (gi, g) in tonales.iter().enumerate() {
        for &b in &g.bins {
            if !est_tonal[b] {
                est_tonal[b] = true;
                groupe_de_bin[b] = gi;
            }
        }
    }
    let transitoire = detecter_transitoire(&a, &est_tonal, m, hop, sr);
    let (clic, hf0) = match &transitoire {
        Some((c, _, h)) => (c.clone(), *h),
        None => (vec![false; t], m),
    };

    // Accumulateurs de masques (fraction de |X| attribuée à chaque source / à la voix).
    let zeros = || vec![vec![0.0_f64; m]; t];
    let mut mt: Vec<Vec<Vec<f64>>> = (0..tonales.len()).map(|_| zeros()).collect();
    let mut mbb = zeros();
    let mut mtr = zeros();
    let mut mvoix = zeros();

    for i in 0..t {
        for k in 0..m {
            let av = a[i][k];
            if av <= 1e-12 {
                continue; // |X|=0 → tout masque y est nul (rien à attribuer)
            }
            let s = av.min(pl[k]); // partie stationnaire
            let e = (av - s).max(0.0); // excès
            let mut attribue = 0.0;
            if est_tonal[k] {
                let g = groupe_de_bin[k];
                mt[g][i][k] = s / av; // la raie ne prend QUE le niveau stationnaire
                attribue += s / av;
            } else {
                mbb[i][k] = s / av; // le souffle = le plancher des bins non-tonals
                attribue += s / av;
            }
            if !est_tonal[k] && clic[i] && k >= hf0 {
                mtr[i][k] = e / av; // l'excès HF d'une trame-clic = le transitoire (laisse intacte la voix LF)
                attribue += e / av;
            }
            mvoix[i][k] = (1.0 - attribue).max(0.0); // le reste = la voix
        }
    }

    // Énergie d'un masque sur le spectrogramme du mélange A (= ce que la source pèse).
    let e_de = |mask: &[Vec<f64>]| -> f64 {
        let mut e = 0.0;
        for i in 0..t {
            for k in 0..m {
                let v = mask[i][k] * a[i][k];
                e += v * v;
            }
        }
        e
    };
    let e_total: f64 = a.iter().flat_map(|r| r.iter()).map(|&x| x * x).sum::<f64>().max(1e-30);

    // Candidats : chaque masque détecté, avec son énergie et son seuil de pertinence.
    struct Cand {
        nature: &'static str,
        desc: String,
        masque: Vec<Vec<f64>>,
        e: f64,
        seuil: f64, // un clic périodique est gardé même peu énergétique ; tonale/large bande = gabarit large
    }
    let mut cands: Vec<Cand> = Vec::new();
    for (gi, g) in tonales.iter().enumerate() {
        let masque = std::mem::take(&mut mt[gi]);
        let e = e_de(&masque);
        cands.push(Cand { nature: "tonale", desc: format!("tonale ~{:.0} Hz ({} harm.)", g.f0_hz, g.n_harm), masque, e, seuil: BB_FRAC_MIN });
    }
    {
        let e = e_de(&mbb);
        cands.push(Cand { nature: "large bande", desc: "large bande (souffle)".to_string(), masque: mbb, e, seuil: BB_FRAC_MIN });
    }
    if let Some((_, periode_s, _)) = transitoire {
        let e = e_de(&mtr);
        cands.push(Cand { nature: "transitoire périodique", desc: format!("transitoire périodique ~{:.2} s", periode_s), masque: mtr, e, seuil: 1e-5 });
    }

    // Garder les pertinents ; REPLIER les autres dans la voix (l'invariant de partition reste exact : aucune énergie
    // ne disparaît — une source trop faible redevient « de la voix »).
    let mut gardes: Vec<Cand> = Vec::new();
    for c in cands {
        if c.e / e_total >= c.seuil {
            gardes.push(c);
        } else {
            for i in 0..t {
                for k in 0..m {
                    mvoix[i][k] = (mvoix[i][k] + c.masque[i][k]).min(1.0);
                }
            }
        }
    }
    let e_voix = e_de(&mvoix).max(1e-30);
    let mut sources: Vec<Source> = gardes
        .into_iter()
        .map(|c| Source {
            nature: c.nature,
            descriptif: c.desc,
            energie_frac: c.e / e_total,
            bulle_pct: 100.0 * (c.e / e_voix).sqrt(),
            masque: c.masque,
        })
        .collect();
    // Bruit 1 = le plus fort.
    sources.sort_by(|x, y| y.energie_frac.partial_cmp(&x.energie_frac).unwrap());

    Separation { sources, voix: mvoix, n, hop }
}

// ---- Banc `jeu separe` : mélange à VÉRITÉ-TERRAIN connue, séparation MESURÉE ------------------------------------

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

/// Voix synthétique intermittente (~3 syllabes/s), enveloppe à RAMPES douces (~20 ms) : plus réaliste qu'un on/off
/// carré, et surtout sans le faux-onset large bande d'une marche (qui imiterait un transitoire).
fn voix_syllabique(sr: f64, n_ech: usize) -> Vec<f32> {
    use std::f64::consts::PI;
    let r = 0.06; // largeur de rampe en fraction de cycle (cycle = 1/3 s → ~20 ms)
    let enveloppe = |cyc: f64| -> f64 {
        if cyc < r {
            0.5 * (1.0 - (PI * cyc / r).cos()) // montée
        } else if cyc < 0.65 - r {
            1.0 // pleine
        } else if cyc < 0.65 {
            0.5 * (1.0 + (PI * (cyc - (0.65 - r)) / r).cos()) // descente
        } else {
            0.0 // silence (laisse voir le bruit)
        }
    };
    (0..n_ech)
        .map(|k| {
            let t = k as f64 / sr;
            let env = enveloppe((t * 3.0).fract());
            let s: f64 = (1..=5).map(|h| (1.0 / h as f64) * (2.0 * PI * 165.0 * h as f64 * t).sin()).sum();
            (0.40 * env * s) as f32 // un peu en retrait pour laisser les bruits AUDIBLES (et garder du headroom)
        })
        .collect()
}

/// Les trois bruits de la vérité-terrain (séparés exprès → on sait ce que chaque masque DEVRAIT capter).
/// Niveaux RÉALISTES : un vrai ventilo / souffle GÊNE autant que la voix → le retrait doit s'ENTENDRE.
fn bruits_seuls(sr: f64, n_ech: usize) -> Vec<(&'static str, Vec<f32>)> {
    use std::f64::consts::PI;
    let t = |k: usize| k as f64 / sr;
    let ventilo: Vec<f32> = (0..n_ech)
        .map(|k| (0.20 * ((2.0 * PI * 120.0 * t(k)).sin() + 0.5 * (2.0 * PI * 240.0 * t(k)).sin())) as f32)
        .collect();
    let mut rng = Rng(0xBADCAFE);
    let souffle: Vec<f32> = (0..n_ech).map(|_| 0.12 * rng.next_f32()).collect();
    let clics: Vec<f32> = (0..n_ech)
        .map(|k| {
            let periode = (sr * 0.5) as usize;
            let phase = k % periode.max(1);
            let env = (-(phase as f64) / (sr * 0.004)).exp();
            (0.6 * env * (2.0 * PI * 2000.0 * t(k)).sin()) as f32
        })
        .collect();
    vec![("ventilo tonal", ventilo), ("souffle", souffle), ("clics", clics)]
}

/// Vérité-terrain pour les TESTS (déterministe, sans fichier) : voix SYNTHÉTIQUE + les trois bruits.
#[cfg(test)]
fn composantes(sr: f64, n_ech: usize) -> (Vec<f32>, Vec<(&'static str, Vec<f32>)>) {
    (voix_syllabique(sr, n_ech), bruits_seuls(sr, n_ech))
}

/// Charge une VRAIE voix (TTS espeak-ng, ou un enregistrement) depuis `voix_wav/voix_source.wav` si présent,
/// rééchantillonnée au banc et normalisée — sinon None (retombe sur le buzz synthétique). C'est CE qui rend le
/// retrait AUDIBLE : une voix large bande et modulée est perceptivement distincte d'un ventilo grave, alors que le
/// buzz 165 Hz le MASQUAIT (d'où « aucune différence » à l'oreille au 1er essai — bug repéré par l'utilisateur).
fn charger_voix(sr: f64) -> Option<Vec<f32>> {
    let (sig, sr_in) = super::spectro::lire_wav("voix_wav/voix_source.wav").ok()?;
    let mut v = super::spectro::reechantillonner(&sig, sr_in, sr);
    let peak = v.iter().fold(0.0_f32, |m, &x| m.max(x.abs())).max(1e-6);
    let g = 0.6 / peak; // crête ~0.6 : la voix domine bien, avec du headroom (la parole est peaky → RMS bas)
    for x in v.iter_mut() {
        *x *= g;
    }
    Some(v)
}

fn somme(parts: &[&[f32]]) -> Vec<f32> {
    let n = parts.iter().map(|p| p.len()).max().unwrap_or(0);
    (0..n).map(|k| parts.iter().map(|p| p.get(k).copied().unwrap_or(0.0)).sum()).collect()
}

/// Énergie d'un composant SEUL passé dans un masque (réutilise la décomposition rigoureuse de denoise.rs : le même
/// masque appliqué à chaque composant de vérité-terrain dit combien de CHAQUE vraie source il capte).
fn energie_masquee(comp: &[f32], masque: &[Vec<f64>], n: usize, hop: usize) -> f64 {
    let win = hann(n);
    let m = n_bins_uniques(n);
    let sp = stft(comp, n, hop, &win);
    let mut e = 0.0;
    for (t, fx) in sp.iter().enumerate() {
        for k in 0..m {
            let g = masque.get(t).and_then(|r| r.get(k)).copied().unwrap_or(0.0);
            e += g * g * (fx[k].re * fx[k].re + fx[k].im * fx[k].im);
        }
    }
    e
}

fn energie_totale(comp: &[f32], n: usize, hop: usize) -> f64 {
    let win = hann(n);
    let m = n_bins_uniques(n);
    stft(comp, n, hop, &win)
        .iter()
        .flat_map(|fx| (0..m).map(|k| fx[k].re * fx[k].re + fx[k].im * fx[k].im))
        .sum()
}

/// Écrit un WAV PCM 16-bit mono (std-only) → l'utilisateur ÉCOUTE chaque bruit isolé pour de vrai.
fn ecrire_wav(chemin: &str, signal: &[f32], sr: u32) -> std::io::Result<()> {
    use std::io::Write;
    let data_len = (signal.len() * 2) as u32;
    let mut buf: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
    buf.extend_from_slice(b"RIFF");
    buf.extend_from_slice(&(36 + data_len).to_le_bytes());
    buf.extend_from_slice(b"WAVE");
    buf.extend_from_slice(b"fmt ");
    buf.extend_from_slice(&16u32.to_le_bytes());
    buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
    buf.extend_from_slice(&1u16.to_le_bytes()); // mono
    buf.extend_from_slice(&sr.to_le_bytes());
    buf.extend_from_slice(&(sr * 2).to_le_bytes()); // byte rate
    buf.extend_from_slice(&2u16.to_le_bytes()); // block align
    buf.extend_from_slice(&16u16.to_le_bytes()); // bits/échantillon
    buf.extend_from_slice(b"data");
    buf.extend_from_slice(&data_len.to_le_bytes());
    for &x in signal {
        buf.extend_from_slice(&((x.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::File::create(chemin)?.write_all(&buf)
}

/// Point d'entrée `jeu separe` (`jeu separe wav` écrit aussi les WAV à écouter).
pub fn run_separe(arg: &str) {
    let (sr, n, hop) = (16000.0_f64, 512usize, 256usize);
    let (voix, voix_reelle) = match charger_voix(sr) {
        Some(v) => (v, true),
        None => (voix_syllabique(sr, (sr * 3.0) as usize), false),
    };
    let n_ech = voix.len();
    let comps = bruits_seuls(sr, n_ech);
    let melange = somme(&[&voix, &comps[0].1, &comps[1].1, &comps[2].1]);
    let sep = analyser(&melange, n, hop, sr);

    println!("🔊  SÉPARATION DE SOURCES — le calcul énumère les bruits, les ISOLE, n'en retire QUE les cochés");
    println!(
        "    {} Hz · STFT {} · voix {} + ventilo(120/240 Hz) + souffle + clics(2/s) · white-box, zéro IA\n",
        sr as u32,
        n,
        if voix_reelle { "RÉELLE (voix_wav/voix_source.wav)" } else { "synthétique" }
    );

    // 1) Les bruits ÉNUMÉRÉS + l'audition isolée (SIR = combien de la vraie source capte chaque masque).
    println!("   Bruits détectés (audition = on entend SEULEMENT ce bruit) :");
    println!("   {:<3} {:<34} {:>8} {:>9} {:>22}", "#", "nature (descripteur objectif)", "énergie", "bulle", "isolation (capte)");
    for (i, s) in sep.sources.iter().enumerate() {
        // À quoi correspond cette source ? celle des 3 vraies dont le masque capte le plus d'énergie.
        let mut best = ("?", 0.0_f64);
        let mut e_cible = 0.0;
        let mut e_autres = 0.0;
        for (nom, c) in &comps {
            let e = energie_masquee(c, &s.masque, n, hop);
            if e > best.1 {
                best = (nom, e);
            }
        }
        for (nom, c) in &comps {
            let e = energie_masquee(c, &s.masque, n, hop);
            if *nom == best.0 {
                e_cible += e;
            } else {
                e_autres += e;
            }
        }
        // la voix qui fuit dans l'isolation compte aussi comme « autre »
        e_autres += energie_masquee(&voix, &s.masque, n, hop);
        let sir = 10.0 * (e_cible / e_autres.max(1e-30)).log10();
        println!(
            "   {:<3} {:<34} {:>7.0}% {:>8.0}% {:>10} SIR {:>5.1} dB",
            format!("B{}", i + 1),
            s.descriptif,
            100.0 * s.energie_frac,
            s.bulle_pct,
            best.0,
            sir
        );
    }

    // 2) Le RETRAIT sélectif : on coche UN bruit → lui seul baisse, le reste (voix + autres bruits) reste intact.
    println!("\n   Retrait sélectif (Δ dB par vraie source ; ↓ grand = retiré, ≈0 = préservé) :");
    println!("   {:<24} {:>12} {:>12} {:>10} {:>10}", "on coche…", "ventilo", "souffle", "clics", "voix");
    let etiquette = |s: &Source| -> &str {
        match s.nature {
            "tonale" => "ventilo",
            "large bande" => "souffle",
            _ => "clics",
        }
    };
    for (i, s) in sep.sources.iter().enumerate() {
        // Δ dB par vraie source = énergie AVANT / énergie APRÈS, le masque-garde (1 − coché) appliqué au composant.
        let garde = masque_garde(&sep, &[i]);
        let d = |c: &[f32]| 10.0 * (energie_totale(c, n, hop) / energie_masquee(c, &garde, n, hop).max(1e-30)).log10();
        println!(
            "   coche B{} ({:<8}) {:>11.1} {:>12.1} {:>10.1} {:>10.1}",
            i + 1,
            etiquette(s),
            d(&comps[0].1),
            d(&comps[1].1),
            d(&comps[2].1),
            d(&voix)
        );
    }

    // 3) Tout cocher → la voix SEULE (preuve qu'elle survit au nettoyage complet).
    let tous: Vec<usize> = (0..sep.sources.len()).collect();
    let garde = masque_garde(&sep, &tous);
    let dv = 10.0 * (energie_totale(&voix, n, hop) / energie_masquee(&voix, &garde, n, hop).max(1e-30)).log10();
    println!("\n   Tout cocher → voix conservée à {:.1} dB (≈0 = intacte), tous les bruits retirés.", dv);

    if arg == "wav" {
        let dir = "voix_wav";
        let _ = std::fs::create_dir_all(dir);
        let sr_u = sr as u32;
        let _ = ecrire_wav(&format!("{dir}/00_melange.wav"), &melange, sr_u);
        for (i, _s) in sep.sources.iter().enumerate() {
            // l'audition isolée (B{k}_isole) ET le mélange « j'ai coché B{k} » (sans_B{k}) = le résultat in-game.
            let _ = ecrire_wav(&format!("{dir}/B{}_isole.wav", i + 1), &sep.isoler(&melange, i), sr_u);
            let _ = ecrire_wav(&format!("{dir}/sans_B{}.wav", i + 1), &sep.retirer(&melange, &[i]), sr_u);
        }
        let _ = ecrire_wav(&format!("{dir}/voix_seule.wav"), &sep.voix(&melange), sr_u);
        println!("\n🎧  WAV écrits dans ./{dir}/ : mélange, chaque bruit ISOLÉ (B*_isole), le mélange SANS le bruit coché");
        println!("    (sans_B*), et la voix seule — à écouter pour valider l'audition + le retrait sélectif.");
    }

    println!("\n📌 Lecture : chaque bruit est SÉPARÉ et auditionnable AVANT de cocher ; cocher n'enlève QUE lui (la voix");
    println!("   et les bruits gardés restent intacts) ; aucun nom donné par une IA (descripteurs objectifs). White-box,");
    println!("   mesuré, pas à l'oreille. Détail : prive/PLAN_TEST_VOIX.md §1.8 (audition + cases à cocher).");
}

/// Le masque-GARDE (1 − Σ cochés) — partagé par le banc et `retirer`.
fn masque_garde(sep: &Separation, set: &[usize]) -> Vec<Vec<f64>> {
    let frames = sep.voix.len();
    let m = n_bins_uniques(sep.n);
    let mut garde = vec![vec![1.0_f64; m]; frames];
    for &s in set {
        for (t, row) in sep.sources[s].masque.iter().enumerate() {
            for (k, &g) in row.iter().enumerate() {
                garde[t][k] = (garde[t][k] - g).max(0.0);
            }
        }
    }
    garde
}

#[cfg(test)]
mod tests {
    use super::*;

    fn banc() -> (Vec<f32>, Vec<(&'static str, Vec<f32>)>, Separation, usize, usize, f64) {
        let (sr, n, hop, dur) = (16000.0_f64, 512usize, 256usize, 3.0);
        let n_ech = (sr * dur) as usize;
        let (voix, comps) = composantes(sr, n_ech);
        let melange = somme(&[&voix, &comps[0].1, &comps[1].1, &comps[2].1]);
        let sep = analyser(&melange, n, hop, sr);
        (melange, comps, sep, n, hop, sr)
    }

    #[test]
    fn les_trois_bruits_sont_separes() {
        let (_mel, _comps, sep, _n, _hop, _sr) = banc();
        let natures: Vec<&str> = sep.sources.iter().map(|s| s.nature).collect();
        assert!(natures.contains(&"tonale"), "le ventilo tonal doit être une source : {:?}", natures);
        assert!(natures.contains(&"large bande"), "le souffle doit être une source : {:?}", natures);
        assert!(natures.contains(&"transitoire périodique"), "les clics doivent être une source : {:?}", natures);
    }

    #[test]
    fn partition_de_lunite_isoler_tout_plus_voix_redonne_loriginal() {
        // Invariant fondateur : Σ masques + voix = 1 → la somme des isolations + la voix == le mélange (intérieur).
        let (mel, _comps, sep, n, _hop, _sr) = banc();
        let mut rec = sep.voix(&mel);
        for i in 0..sep.sources.len() {
            let iso = sep.isoler(&mel, i);
            for (r, v) in rec.iter_mut().zip(&iso) {
                *r += v;
            }
        }
        // Intérieur seulement (les bords sont atténués par la fenêtre, comme le test STFT de fft.rs).
        let err: f64 = (n..mel.len() - n).map(|k| (mel[k] - rec[k]).abs() as f64).fold(0.0, f64::max);
        assert!(err < 1e-3, "isoler(toutes)+voix doit reconstruire l'original, erreur max = {}", err);
    }

    #[test]
    fn la_periode_des_clics_est_retrouvee() {
        let (_mel, _comps, sep, _n, _hop, _sr) = banc();
        let s = sep.sources.iter().find(|s| s.nature == "transitoire périodique").expect("source clics");
        // descriptif "transitoire périodique ~0.50 s" : on vérifie ~0.5 s (clics à 2/s).
        let periode: f64 = s.descriptif.split('~').nth(1).unwrap().trim_end_matches(" s").trim().parse().unwrap();
        assert!((periode - 0.5).abs() < 0.1, "période clics ≈ 0.5 s, trouvé {}", periode);
    }

    #[test]
    fn cocher_le_ventilo_le_retire_en_preservant_le_reste() {
        // LE test du concept : on coche le SEUL bruit tonal → il chute fort, souffle/clics/voix restent ≈ intacts.
        let (mel, comps, sep, n, hop, _sr) = banc();
        let i_ton = sep.sources.iter().position(|s| s.nature == "tonale").expect("source tonale");
        let garde = masque_garde(&sep, &[i_ton]);
        let d = |c: &[f32]| 10.0 * (energie_totale(c, n, hop) / energie_masquee(c, &garde, n, hop).max(1e-30)).log10();
        let (dvent, dsouf, dclic, dvoix) = (d(&comps[0].1), d(&comps[1].1), d(&comps[2].1), d(&mel.iter().zip(&comps[0].1).zip(&comps[1].1).zip(&comps[2].1).map(|(((m,v),s),c)| m-v-s-c).collect::<Vec<f32>>()));
        assert!(dvent > 3.0, "le ventilo coché doit chuter nettement : {} dB", dvent);
        assert!(dsouf < 1.0, "le souffle non coché doit être préservé : {} dB", dsouf);
        assert!(dclic < 1.0, "les clics non cochés doivent être préservés : {} dB", dclic);
        assert!(dvoix < 2.0, "la voix doit être préservée : {} dB", dvoix);
    }
}
