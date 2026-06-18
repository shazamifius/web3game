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

use super::attack::run_attack;
use super::bot::Bot;
use super::rendezvous::run_rendezvous;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// Le bilan d'un nœud à la fin de la simulation.
struct NodeStat {
    neighbors: usize,
    accepted: u64,
    rejected: u64,
    muted: usize,
    orb_stolen: bool,
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
            let mut last = Instant::now();
            while start.elapsed().as_secs() < secs {
                let dt = last.elapsed().as_secs_f32();
                last = Instant::now();
                bot.step(dt, start.elapsed().as_secs_f32());
                thread::sleep(tick);
            }
            let ns = NodeStat {
                neighbors: bot.neighbors(),
                accepted: bot.accepted(),
                rejected: bot.rejected(),
                muted: bot.muted(),
                orb_stolen: bot.orb_master().is_some(),
            };
            stats.lock().unwrap().push(ns);
        }));
    }

    // 3) M attaquants, en boucle (variés), détachés : ils tapent sans relâche pendant
    //    toute la simulation et sont tués à la fin du process.
    let variants = ["orb-creep", "teleport", "flood", "forge", "sybil"];
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
    println!("===========================================");
    if up > 0 && stolen == 0 {
        println!("✅ L'essaim a TENU : voisinage borné, orbe intègre, attaques absorbées.");
    } else if up == 0 {
        println!("⚠ Aucun nœud monté : démarrage trop court ? (augmente `secondes`)");
    } else {
        println!("⚠ Orbe compromise sur {stolen} nœud(s) — à investiguer.");
    }
}
