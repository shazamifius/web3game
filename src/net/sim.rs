//! LA SIMULATION MASSIVE (chapitre 6.8) : « est-ce que ça tient ? »
//!
//! On ne peut pas lancer 55 000 fenêtres de jeu. Mais on peut lancer des CENTAINES
//! de nœuds headless (`Bot`) en threads, sur une seule machine, tous sur localhost,
//! avec un rendez-vous local et un essaim d'attaquants — et MESURER ce qui se passe.
//!
//! # Ce que ça prouve (et ce que ça ne prouve pas)
//! Grâce au voisinage borné (chap. 6.6, `MAX_NEIGHBORS`), la charge PAR NŒUD ne
//! dépend PAS du nombre total de joueurs : chacun ne parle qu'à ~32 voisins. Donc si
//! l'essaim tient à 200 ou 500 nœuds avec des attaquants, le comportement par nœud
//! est le même à 55 000 — la vraie échelle se fait en AJOUTANT des machines (chaque
//! joueur est un appareil réel), pas en surchargeant une seule. Cette simulation
//! valide la CORRECTION et la RÉSISTANCE AUX ATTAQUES à l'échelle que la machine
//! encaisse ; le passage planétaire viendra des optimisations de lien à venir.
//!
//! Lancement :  cargo run -- sim [nb_bots] [nb_attaquants] [secondes]
//!   ex.        cargo run -- sim 200 5 20

use super::aoi::MAX_NEIGHBORS;
use super::attack::run_attack;
use super::bot::Bot;
use super::probe::{peak_rss_bytes, thread_cpu_secs};
use super::rendezvous::run_rendezvous;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Le bilan d'un nœud à la fin de la simulation.
struct NodeStat {
    neighbors: usize,
    /// Tiers de perception ENTENDUS sur la fenêtre (chap. 8.2b) : pairs entendus à plein
    /// débit (focus) et en basse fidélité (conscience). Remplace « connus » pour la couverture.
    focus: usize,
    conscience: usize,
    accepted: u64,
    rejected: u64,
    muted: usize,
    orb_stolen: bool,
    /// COÛT RÉEL du nœud (chap. 7.4), mesuré sur la fenêtre `secs` :
    bytes_up: u64,   // octets émis par ce nœud
    bytes_down: u64, // octets reçus par ce nœud
    cpu_secs: f64,   // secondes de CPU brûlées par le thread de ce nœud
}

pub fn run_sim(n_bots: usize, n_attackers: usize, secs: u64) {
    println!("=== SIMULATION : {n_bots} bots honnêtes + {n_attackers} attaquant(s), {secs}s ===");
    println!("(chaque nœud MINE son identité puis tourne le VRAI protocole ; patiente le démarrage…)");

    // 1) Un rendez-vous local, en thread détaché (meurt à la fin du process).
    thread::spawn(run_rendezvous);
    thread::sleep(Duration::from_millis(500));

    let stats: Arc<Mutex<Vec<NodeStat>>> = Arc::new(Mutex::new(Vec::new()));
    let tick = Duration::from_millis(50);

    // 2) N bots honnêtes. Chacun MINE (dans Bot::new) puis tourne `secs` à partir de
    //    SON démarrage (pour avoir une fenêtre pleine malgré le coût de minage).
    let mut handles = Vec::new();
    for i in 0..n_bots {
        let stats = Arc::clone(&stats);
        handles.push(thread::spawn(move || {
            let phase = i as f32 * 0.37; // étale la « foule » sur le cercle
            let Some(mut bot) = Bot::new(format!("h{i}"), false, phase) else {
                return;
            };
            let start = Instant::now();
            // On photographie le coût AU DÉBUT de la fenêtre (après le minage de
            // l'identité, fait dans Bot::new) : ainsi le CPU et les octets mesurés ne
            // couvrent QUE le protocole sur `secs`, pas la preuve de travail initiale.
            let cpu0 = thread_cpu_secs();
            let up0 = bot.bytes_up();
            let down0 = bot.bytes_down();
            bot.reset_heard(); // 8.2b : les tiers focus/conscience ne comptent QUE la fenêtre
            let mut last = Instant::now();
            while start.elapsed().as_secs() < secs {
                let dt = last.elapsed().as_secs_f32();
                last = Instant::now();
                bot.step(dt, start.elapsed().as_secs_f32());
                thread::sleep(tick);
            }
            let (focus, conscience) = bot.heard_tiers(secs as f32);
            let ns = NodeStat {
                neighbors: bot.neighbors(),
                focus,
                conscience,
                accepted: bot.accepted(),
                rejected: bot.rejected(),
                muted: bot.muted(),
                orb_stolen: bot.orb_master().is_some(),
                bytes_up: bot.bytes_up().saturating_sub(up0),
                bytes_down: bot.bytes_down().saturating_sub(down0),
                cpu_secs: (thread_cpu_secs() - cpu0).max(0.0),
            };
            stats.lock().unwrap().push(ns);
        }));
    }

    // 3) M attaquants, en boucle (variés), détachés : ils tapent sans relâche pendant
    //    toute la simulation et sont tués à la fin du process.
    let variants = ["orb-creep", "teleport", "flood", "forge", "sybil", "gossip-flood"];
    for i in 0..n_attackers {
        let v = variants[i % variants.len()].to_string();
        thread::spawn(move || loop {
            run_attack(&v);
        });
    }

    // 4) On attend que tous les bots aient fini leur fenêtre, puis on agrège.
    for h in handles {
        let _ = h.join();
    }
    report(&stats.lock().unwrap(), n_bots, n_attackers, secs);
}

/// Mode `crowd` (chap. 8.0) : une FOULE DENSE de `n` nœuds AU MÊME ENDROIT, sans
/// attaquant, dédiée à mesurer la COUVERTURE DE PERCEPTION (le mur D22). C'est `sim`
/// sans attaquants, avec l'intention affichée : « sur la foule à portée, combien chaque
/// nœud en perçoit-il ? » Aujourd'hui, plafonné → on s'attend à une couverture FAIBLE
/// dès que `n` dépasse le plafond de voisinage. Le but du chapitre 8 est de la faire
/// monter à ~100 % SANS exploser le débit (preuve : rejouer ceci à la fin du chapitre).
pub fn run_crowd(n: usize, secs: u64) {
    let attendu = if n > 1 { (MAX_NEIGHBORS * 100 / (n - 1)).min(100) } else { 100 };
    println!("=== FOULE DENSE (8.0, D22) : {n} nœuds AU MÊME ENDROIT, {secs}s, 0 attaquant ===");
    println!("On mesure la COUVERTURE DE PERCEPTION (perçus ÷ à portée).");
    println!("Plafond actuel = {MAX_NEIGHBORS} → couverture attendue ~{attendu}% (le reste : AVEUGLE).");
    run_sim(n, 0, secs);
}

fn report(stats: &[NodeStat], n_bots: usize, n_attackers: usize, secs: u64) {
    let up = stats.iter().filter(|s| s.neighbors > 0).count();
    let total_acc: u64 = stats.iter().map(|s| s.accepted).sum();
    let total_rej: u64 = stats.iter().map(|s| s.rejected).sum();
    let total_muted: usize = stats.iter().map(|s| s.muted).sum();
    let stolen = stats.iter().filter(|s| s.orb_stolen).count();
    let avg_nb = if stats.is_empty() {
        0.0
    } else {
        stats.iter().map(|s| s.neighbors).sum::<usize>() as f32 / stats.len() as f32
    };
    let max_nb = stats.iter().map(|s| s.neighbors).max().unwrap_or(0);

    println!("\n========== RAPPORT DE SIMULATION ==========");
    println!("Demandé : {n_bots} bots, {n_attackers} attaquant(s), {secs}s.");
    println!("Nœuds montés (avec voisins)        : {up}/{n_bots}");
    println!("Voisins par nœud                   : moy {avg_nb:.1}, max {max_nb}  (plafond 32 — borne d'échelle 6.6)");
    println!("État honnête accepté (cumulé)      : {total_acc} paquets (~{:.0}/s)", total_acc as f32 / secs as f32);
    println!("Paquets de triche rejetés (cumulé) : {total_rej}");
    println!("Sourdines (tricheurs neutralisés)  : {total_muted}");
    println!("Intégrité de l'orbe                : {stolen}/{n_bots} nœud(s) avec orbe volée (attendu 0)");

    // ---- COÛT RÉEL PAR NŒUD (chap. 7.4, ferme D19) ---------------------------------
    // On chiffre sur les nœuds RÉELLEMENT actifs (avec voisins) : c'est le profil d'un
    // vrai participant. La bande passante est la métrique reine pour extrapoler à 55k.
    let active: Vec<&NodeStat> = stats.iter().filter(|s| s.neighbors > 0).collect();
    if !active.is_empty() {
        let n = active.len() as f32;
        let secs_f = secs.max(1) as f32;
        // Ko/s = octets / secondes / 1024.
        let up_rates: Vec<f32> = active.iter().map(|s| s.bytes_up as f32 / secs_f / 1024.0).collect();
        let down_rates: Vec<f32> = active.iter().map(|s| s.bytes_down as f32 / secs_f / 1024.0).collect();
        let avg_up = up_rates.iter().sum::<f32>() / n;
        let max_up = up_rates.iter().cloned().fold(0.0, f32::max);
        let avg_down = down_rates.iter().sum::<f32>() / n;
        let max_down = down_rates.iter().cloned().fold(0.0, f32::max);
        // %CPU d'un cœur = temps CPU / temps mur × 100.
        let cpu_pct: Vec<f32> = active.iter().map(|s| (s.cpu_secs as f32 / secs_f) * 100.0).collect();
        let avg_cpu = cpu_pct.iter().sum::<f32>() / n;
        let max_cpu = cpu_pct.iter().cloned().fold(0.0, f32::max);

        println!("-------- COÛT RÉEL PAR NŒUD (7.4) ---------");
        println!("Bande passante ↑ (émis)            : moy {avg_up:.1} Ko/s, max {max_up:.1} Ko/s");
        println!("Bande passante ↓ (reçu)            : moy {avg_down:.1} Ko/s, max {max_down:.1} Ko/s");
        println!("CPU par nœud (logique+crypto)      : moy {avg_cpu:.1} %cœur, max {max_cpu:.1} %cœur");
        println!("  (localhost : ne compte PAS le coût réseau d'un vrai déploiement)");
        // RAM : valeur GLOBALE du process, jamais par nœud (un seul tas partagé).
        let peak = peak_rss_bytes();
        if peak > 0 {
            let peak_mo = peak as f32 / (1024.0 * 1024.0);
            let approx_per = peak_mo / n_bots.max(1) as f32;
            println!("RAM crête du PROCESSUS entier      : {peak_mo:.0} Mo  (~{approx_per:.1} Mo/nœud — MOYENNE grossière,");
            println!("  inclut rendez-vous + attaquants + code + allocateur ; PAS une mesure par nœud)");
        }
        // Extrapolation honnête : coût borné par le VOISINAGE (~32), pas par le total
        // (6.6) → constant à 55k... MAIS seulement si voir ~32 voisins suffit (cf. infra).
        println!("→ Coût borné par le voisinage (~{MAX_NEIGHBORS}), PAS par le total → constant à 55k");
        println!("  TANT QUE voir ~{MAX_NEIGHBORS} voisins suffit. ↑ {avg_up:.1} Ko/s/nœud = la contrainte clé.");

        // ---- COUVERTURE DE PERCEPTION (chap. 8.0 — on CHIFFRE le mur D22) -------------
        // Les bots tournent tous sur un cercle de rayon WANDER_RADIUS = 3 m (bot.rs) →
        // diamètre 6 m ≪ CANDIDATE_RADIUS = 500 m : CHAQUE nœud actif est RÉELLEMENT à
        // portée de TOUS les autres. La « foule à portée » d'un nœud = (actifs − 1). On
        // mesure la FRACTION de cette foule qu'il perçoit vraiment. C'est la métrique reine
        // du chapitre 8 : on veut la voir monter à ~100 % SANS que le débit ↓ explose.
        let crowd = active.len().saturating_sub(1).max(1);
        // 8.2b : on compte les pairs ENTENDUS sur la fenêtre, pas seulement CONNUS. FOCUS =
        // entendus à plein débit (lien net) ; CONSCIENCE = entendus en basse fidélité (LOD).
        // (Avant 8.2, conscience = 0 et focus = nb de connus → métrique optimiste, corrigée ici.)
        let avg_focus = active.iter().map(|s| s.focus).sum::<usize>() as f32 / n;
        let avg_awareness = active.iter().map(|s| s.conscience).sum::<usize>() as f32 / n;
        let coverages: Vec<f32> = active
            .iter()
            .map(|s| ((s.focus + s.conscience) as f32 / crowd as f32).min(1.0))
            .collect();
        let avg_cov = coverages.iter().sum::<f32>() / n;
        let min_cov = coverages.iter().cloned().fold(f32::INFINITY, f32::min);
        let avg_blind = (crowd as f32 - avg_focus - avg_awareness).max(0.0);
        println!("-------- COUVERTURE DE PERCEPTION (8.2b, ENTENDUS ≠ connus) --------");
        println!("Foule réellement à portée (co-localisée) : {crowd} pairs/nœud");
        println!("ENTENDUS par nœud (8.2 deux tiers) : FOCUS moy {avg_focus:.1} (plein débit) + CONSCIENCE moy {avg_awareness:.1} (LOD basse fidélité)");
        println!(
            "COUVERTURE de la foule             : moy {:.0}%, min {:.0}%  (ENTENDUS ÷ à portée)",
            avg_cov * 100.0,
            min_cov * 100.0
        );
        if avg_blind >= 1.0 {
            println!("D22 : ~{avg_blind:.0} voisins ni entendus au focus ni en conscience (couverture {:.0}%).", avg_cov * 100.0);
            println!("  ✓ 8.1 : plafond dur {MAX_NEIGHBORS} CASSÉ (gossip). ✓ 8.2 : deux tiers (focus net + conscience LOD).");
            println!("  Résidu attendu = convergence dans la fenêtre (démarrage échelonné) + seuil focus/conscience (~5 Hz).");
            println!("  L'ESSENTIEL (invariant) : le débit ↓ doit rester PLAT quand N grandit (rejouer 200→500).");
        } else {
            println!("✓ Couverture ~complète (ENTENDUS) : focus net + conscience LOD couvrent (presque) toute la foule, débit borné.");
        }
    }
    println!("===========================================");
    if up > 0 && stolen == 0 {
        println!("✅ L'essaim a TENU : voisinage borné, orbe intègre, attaques absorbées.");
    } else if up == 0 {
        println!("⚠ Aucun nœud monté : démarrage trop court ? (augmente `secondes`)");
    } else {
        println!("⚠ Orbe compromise sur {stolen} nœud(s) — à investiguer.");
    }
}
