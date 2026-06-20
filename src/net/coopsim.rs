//! LE BANC LÉGER COOPÉRATIF (dette D25/D20) : mesurer la foule au-delà du plafond ~1500.
//!
//! # Le mur que ça enlève
//! `sim`/`crowd` lancent **un OS-thread par bot** + un `thread::sleep` PAR bot : au-delà de
//! ~1200-1500 nœuds sur ce PC (12 cœurs), on sur-souscrit les cœurs et c'est la SIMU (pas le
//! protocole) qui étouffe. Conséquence : « D22 = échelle 5000+ » LITTÉRAL n'était pas prouvable.
//!
//! # L'idée (T0.1 = ce squelette)
//! On step TOUS les nœuds dans **UNE seule boucle, un seul thread**, **sans `thread::sleep` par
//! bot** (un seul sleep cadence toute la marée). On retire ainsi le vrai goulot documenté (la
//! sur-souscription d'OS-threads), pas le protocole. Les nœuds gardent leur VRAIE prise UDP sur
//! `lo` : ce banc ne triche pas sur le protocole, il ne change que l'ORDONNANCEMENT.
//!
//! # Ce que ce squelette NE fait PAS (honnêteté, cf. PLAN_AUTONOME.md)
//! - Il n'a PAS de bus mémoire : il reste sur l'UDP de `lo`. Si le mur devient l'UDP lui-même
//!   (descripteurs de fichiers, coût syscall) AVANT 50k, c'est un constat honnête — et le bus
//!   mémoire (qui exigerait de rendre `Socket` permutable = toucher le cœur) ira en FILE
//!   UTILISATEUR, pas codé en aveugle.
//! - Sa FIDÉLITÉ n'est PAS encore prouvée : T0.2 (prochain pas) exigera qu'il REPRODUISE les
//!   chiffres connus du vrai `crowd` à ~1000 nœuds AVANT toute extrapolation. Tant que T0.2 n'est
//!   pas vert, ne RIEN conclure de ce banc.
//!
//! Lancement :  cargo run -- coopsim [nb_bots] [secondes]

use super::bot::Bot;
use super::rendezvous::run_rendezvous;
use std::thread;
use std::time::{Duration, Instant};

/// Lance `n_bots` nœuds honnêtes dans UN thread coopératif pendant `secs`, puis rapporte la
/// convergence (voisinage moyen, états échangés, perception par résumé). Pas d'attaquant ici : ce
/// banc sert d'abord à MESURER l'échelle (T0), la résistance aux attaques restant prouvée par `sim`.
pub fn run_coopsim(n_bots: usize, secs: u64) {
    println!("=== BANC LÉGER COOPÉRATIF (D25) : {n_bots} bots, {secs}s, 1 thread, 0 sleep/bot ===");
    println!("(chaque nœud MINE son identité puis tourne le VRAI protocole sur UDP/lo ;");
    println!(" minage SÉQUENTIEL ici → patiente le démarrage, ou baisse POW_BITS pour les gros runs)");

    // 1) Un rendez-vous local, en thread détaché (meurt à la fin du process), comme `run_sim`.
    thread::spawn(run_rendezvous);
    thread::sleep(Duration::from_millis(500));

    // 2) On construit les N bots SÉQUENTIELLEMENT (chacun mine dans Bot::new). On garde le Vec :
    //    ce sont eux qu'on stepra tour à tour dans l'unique boucle.
    let t_build = Instant::now();
    let mut bots: Vec<Bot> = Vec::with_capacity(n_bots);
    for i in 0..n_bots {
        let phase = i as f32 * 0.37; // étale la « foule » sur le cercle, comme `run_sim`
        if let Some(bot) = Bot::new(format!("c{i}"), false, phase) {
            bots.push(bot);
        }
    }
    println!(
        "  {} nœuds prêts (minage + prises) en {:.1}s.",
        bots.len(),
        t_build.elapsed().as_secs_f32()
    );
    if bots.is_empty() {
        println!("⚠ Aucun nœud n'a pu s'ouvrir (prise/identité) — rien à mesurer.");
        return;
    }

    // 3) LA BOUCLE COOPÉRATIVE : on step CHAQUE bot une fois par tick, puis UN seul sleep cadence
    //    toute la marée. C'est ici que disparaît la sur-souscription d'OS-threads de `sim`.
    // On photographie les octets AVANT la fenêtre (les prises comptent déjà le trafic du HELLO) →
    // le débit mesuré ne couvre QUE la fenêtre `secs`, comparable au `crowd` (invariant débit plat).
    let up0: u64 = bots.iter().map(|b| b.bytes_up()).sum();
    let down0: u64 = bots.iter().map(|b| b.bytes_down()).sum();
    let tick = Duration::from_millis(50);
    let start = Instant::now();
    let mut last = Instant::now();
    while start.elapsed().as_secs() < secs {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();
        for bot in bots.iter_mut() {
            bot.step(dt, now);
        }
        thread::sleep(tick); // UN sleep pour tout le monde (pas un par bot)
    }

    // 4) Bilan de convergence — la preuve T0.1 = les nœuds se découvrent ET s'échangent des états.
    let n = bots.len() as f32;
    let avg_neighbors = bots.iter().map(|b| b.neighbors() as f32).sum::<f32>() / n;
    let total_accepted: u64 = bots.iter().map(|b| b.accepted()).sum();
    let avg_summary = bots.iter().map(|b| b.summary_perceived() as f32).sum::<f32>() / n;
    let max_summary = bots.iter().map(|b| b.summary_perceived()).max().unwrap_or(0);
    // Débit par nœud sur la fenêtre (mêmes compteurs de prise que `crowd` → comparable pour T0.2).
    let up = bots.iter().map(|b| b.bytes_up()).sum::<u64>().saturating_sub(up0) as f32;
    let down = bots.iter().map(|b| b.bytes_down()).sum::<u64>().saturating_sub(down0) as f32;
    let kos_up = up / 1000.0 / n / secs as f32;
    let kos_down = down / 1000.0 / n / secs as f32;
    println!("-------- CONVERGENCE (banc léger, 1 thread) --------");
    println!("Pairs CONNUS moyen       : {avg_neighbors:.1}/nœud (table, bornée MAX_KNOWN — PAS le focus ~32).");
    println!("États acceptés (total)   : {total_accepted} (≥1 ⇒ échange de bout en bout PROUVÉ)");
    println!(
        "Perception par RÉSUMÉ     : moy {avg_summary:.0}, max {max_summary} occupants via 1 flux (foule {})",
        bots.len()
    );
    println!("Débit par nœud (fenêtre) : ↑{kos_up:.1} / ↓{kos_down:.1} Ko/s (invariant : doit rester PLAT quand N grandit)");
    println!("===========================================");
    if total_accepted > 0 && avg_neighbors >= 1.0 {
        println!("✅ Le banc coopératif mono-thread DÉLIVRE (découverte + échange d'états).");
        println!("   ⚠ T0.2 : FIDÈLE seulement à BAS N. À N≈1000, ce banc perçoit ~2× MOINS que `crowd 1000`");
        println!("   (un thread sérialise N nœuds → dilate le temps mural ; l'UDP réel interdit un dt fixe).");
        println!("   → NE PAS extrapoler vers 50k. Banc fidèle haute échelle = bus mémoire = change le cœur (file utilisateur).");
    } else {
        println!("⚠ Pas de convergence : rallonge `secondes`, ou le rendez-vous n'a pas amorcé.");
    }
}
