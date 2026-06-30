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
//! # Ce que ce squelette NE fait PAS (honnêteté)
//! - Il n'a PAS de bus mémoire : il reste sur l'UDP de `lo`. Si le mur devient l'UDP lui-même
//!   (descripteurs de fichiers, coût syscall) AVANT 50k, c'est un constat honnête — et le bus
//!   mémoire (qui exigerait de rendre `Socket` permutable = toucher le cœur) ira en FILE
//!   UTILISATEUR, pas codé en aveugle.
//! - Sa FIDÉLITÉ n'est PAS encore prouvée : T0.2 (prochain pas) exigera qu'il REPRODUISE les
//!   chiffres connus du vrai `crowd` à ~1000 nœuds AVANT toute extrapolation. Tant que T0.2 n'est
//!   pas vert, ne RIEN conclure de ce banc.
//!
//! Lancement :  cargo run -- coopsim [nb_bots] [secondes]

use super::aoi::{dist2, keep_nearest, within_radius, MAX_NEIGHBORS};
use super::bot::Bot;
use super::control::{decode_hello, encode_welcome};
use super::crypto::{pow_bits, PeerId};
use super::rendezvous::run_rendezvous;
use super::transport::{new_bus, Socket};
use super::wire::RENDEZVOUS_PORT;
use std::collections::HashMap;
use std::net::SocketAddr;
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

/// Un RENDEZ-VOUS minimal sur BUS mémoire (dette D25) : il RÉUTILISE les vraies fonctions de décision
/// du rendez-vous UDP (`decode_hello`, PoW, AoI `keep_nearest`, `encode_welcome`) ; seule
/// l'ORCHESTRATION est locale.
/// ⚠ BUS_DOUTE — cette orchestration DUPLIQUE celle de `rendezvous.rs` (risque de divergence D2 connu).
/// Les DÉCISIONS sont partagées, mais si on durcit le vrai rendez-vous (ex. rate-limit T1.2 / éviction),
/// ce banc ne le reflète PAS (il n'a ni attaquant ni client mort). À terme : extraire un cœur commun.
struct BusRendezvous {
    socket: Socket,
    clients: HashMap<SocketAddr, (PeerId, (f32, f32))>,
    hue: u16,
}

impl BusRendezvous {
    /// Traite les HELLO reçus ce tick et renvoie à chacun son WELCOME (roster des plus proches).
    fn step(&mut self) {
        for (from, bytes) in self.socket.poll() {
            let Some((px, pz, id)) = decode_hello(&bytes) else {
                continue;
            };
            if !id.has_pow(pow_bits()) {
                continue; // même garde PoW que le vrai rendez-vous
            }
            let pos = (px, pz);
            self.clients.insert(from, (id, pos));
            // Voisinage borné : les MAX_NEIGHBORS plus proches dans le rayon (mêmes helpers AoI).
            let cands: Vec<((PeerId, SocketAddr), f32)> = self
                .clients
                .iter()
                .filter(|(a, (_, p))| **a != from && within_radius(*p, pos))
                .map(|(a, (i, p))| ((*i, *a), dist2(*p, pos)))
                .collect();
            let roster = keep_nearest(cands, MAX_NEIGHBORS);
            let _ = self.socket.send_to(from, &encode_welcome(self.hue, &roster));
        }
    }
}

/// Le BANC BUS MÉMOIRE (dette D25, T0.2-bis) : N nœuds dans UN thread, reliés par le BUS synchrone
/// (`transport::Socket::bus`) au lieu de l'UDP. Comme la livraison est instantanée et déterministe,
/// on avance le temps de simulation par un **dt FIXE sans `sleep`** → `secs` SIM-secondes valent
/// `secs/dt` ticks QUEL QUE SOIT le temps mural. C'est ce qui corrige l'infidélité de `coopsim` (T0.2,
/// où un thread sérialisant N nœuds dilatait le temps réel). But : mesurer DIRECTEMENT l'échelle 5k-50k.
///
/// ⚠ BUS_DOUTE — réseau PARFAIT (0 latence, 0 perte, ordre strict) : ce banc mesure l'ÉCHELLE
/// (perception/débit ∝ N), PAS le réalisme réseau (= rôle de `sim` + `netem`). À VALIDER d'abord par
/// `coopsim-bus N` ≈ `crowd N` aux mêmes N « threadables » (~1000) AVANT toute extrapolation (T0.2-bis).
pub fn run_coopsim_bus(n_bots: usize, secs: u64) {
    println!("=== BANC BUS MÉMOIRE (D25) : {n_bots} bots, {secs}s SIM, dt FIXE, 0 réseau OS ===");
    println!("(livraison synchrone en mémoire → temps-sim découplé du temps mural ; le chemin UDP réel");
    println!(" reste intact — c'est `sim`/`crowd` qui le mesure. ⚠ réseau PARFAIT : mesure l'ÉCHELLE, pas le réseau)");

    let bus = new_bus();
    let rv_addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], RENDEZVOUS_PORT));
    let mut rv = BusRendezvous { socket: Socket::bus(rv_addr, bus.clone()), clients: HashMap::new(), hue: 200 };

    let t_build = Instant::now();
    let mut bots: Vec<Bot> = Vec::with_capacity(n_bots);
    for i in 0..n_bots {
        let phase = i as f32 * 0.37; // étale la « foule », comme `run_sim`
        // ⚠ BUS_DOUTE — adresse SYNTHÉTIQUE unique (clé d'aiguillage du bus, pas une vraie route) ;
        //   `10_000 + i` en u16 → plafonne vers ~55k bots avant collision/débordement (à élargir si besoin).
        let addr = SocketAddr::from(([127, 0, 0, 1], 10_000u16.wrapping_add(i as u16)));
        let socket = Socket::bus(addr, bus.clone());
        bots.push(Bot::new_on(format!("b{i}"), false, phase, socket, rv_addr));
    }
    println!("  {} nœuds prêts (minage + endpoints bus) en {:.1}s.", bots.len(), t_build.elapsed().as_secs_f32());
    if bots.is_empty() {
        println!("⚠ Aucun nœud créé.");
        return;
    }

    // BOUCLE À dt FIXE, SANS `sleep` : c'est ICI que le temps-sim se découple du temps mural.
    let dt = 0.05_f32;
    let ticks = (secs as f32 / dt) as u64;
    let up0: u64 = bots.iter().map(|b| b.bytes_up()).sum();
    let down0: u64 = bots.iter().map(|b| b.bytes_down()).sum();
    let wall = Instant::now();
    let mut now = 0.0_f32;
    // ⚠ BUS_DOUTE — on DÉCALE le démarrage des bots (le bot i ne step qu'à partir du tick i%20, soit
    //   un étalement des arrivées sur ~1 s) pour BRISER le lockstep : sinon tous les timers sont en
    //   phase (vs les threads naturellement décalés du vrai `crowd`), ce qui empêchait le bootstrap
    //   mutuel du perçage à grand N (deadlock N≥700). Décalage harnais-only, plus fidèle au réel.
    const JOIN_SPREAD: u64 = 20;
    // ARRIVÉE PROGRESSIVE OPT-IN (étape A, env `RAMP_S` en secondes SIM). Le mur n°2 (bootstrap lent)
    // a été mesuré dans le scénario IRRÉALISTE « tous à t=0 » (5000 simultanés sur un bus parfait). En
    // déploiement réel, les pairs arrivent ÉTALÉS dans le temps. Sous `RAMP_S>0`, le bot `idx` ne rejoint
    // qu'au tick `idx * ramp_ticks / N` → la foule entière entre LINÉAIREMENT sur `RAMP_S` s, donnant au
    // réseau le temps de se stabiliser entre vagues. But : PROUVER (pas supposer) que la perception monte
    // sans le plateau de bootstrap. Défaut (0) → on garde le `JOIN_SPREAD` ~1 s (binaire identique).
    let ramp_s: f32 = std::env::var("RAMP_S").ok().and_then(|v| v.parse().ok()).unwrap_or(0.0);
    let ramp_ticks = (ramp_s / dt) as u64;
    let nb = bots.len().max(1) as u64;
    let join_tick = |idx: usize| -> u64 {
        if ramp_ticks > 0 { (idx as u64 * ramp_ticks) / nb } else { idx as u64 % JOIN_SPREAD }
    };
    println!("  (arrivée : {})", if ramp_ticks > 0 { format!("PROGRESSIVE sur {ramp_s:.0}s SIM (étape A)") } else { "quasi-simultanée (~1s, baseline)".to_string() });
    // JITTER CONTINU OPT-IN (D25, test (i) protocole vs (ii) lockstep — env `JITTER=1`). Donne à chaque
    // bot un DÉPHASAGE D'HORLOGE CONSTANT propre (réparti ~[0,1s) par un hash déterministe de l'index)
    // ajouté au `now` qu'il voit. Ses timers périodiques (gossip/émission) ne se ré-alignent JAMAIS,
    // contrairement au lockstep du banc — c'est plus FIDÈLE au réel (horloges indépendantes = D13),
    // PAS un tuning : on ne change pas dt (l'échelle de temps), seulement la PHASE. Défaut = 0 (binaire
    // identique → harnais de régression). Si le plateau de bootstrap fond avec jitter → il était un
    // artefact de lockstep ; s'il persiste → propriété réelle du protocole à l'échelle.
    let jitter = matches!(std::env::var("JITTER").as_deref(), Ok("1") | Ok("true"));
    let phase_off = |idx: usize| -> f32 {
        if jitter { (idx.wrapping_mul(2_654_435_761) % 1000) as f32 / 1000.0 } else { 0.0 }
    };
    println!("  (jitter continu des horloges : {})", if jitter { "ON (test (i)/(ii))" } else { "OFF (baseline)" });
    // INSTANTANÉ PÉRIODIQUE (D25, instrumentation) : toutes les ~10 s SIM, on imprime la TRAJECTOIRE
    // (pairs connus, trous ouverts, perception). Tranche « convergence LENTE » (ça monte) de
    // « DEADLOCK » (plat) en UN seul run — lecture seule, ne change rien au protocole.
    let snap_s: f32 = std::env::var("SNAP_S").ok().and_then(|v| v.parse().ok()).unwrap_or(10.0);
    let snap_every = (snap_s / dt) as u64;
    for tick in 0..ticks {
        rv.step(); // le rendez-vous traite les HELLO du tick précédent, renvoie les WELCOME
        for (idx, bot) in bots.iter_mut().enumerate() {
            if tick >= join_tick(idx) {
                bot.step(dt, now + phase_off(idx));
            }
        }
        now += dt;
        if snap_every > 0 && (tick + 1) % snap_every == 0 {
            let nn = bots.len() as f32;
            let kn = bots.iter().map(|b| b.neighbors() as f32).sum::<f32>() / nn;
            let ho = bots.iter().map(|b| b.open_holes().len() as f32).sum::<f32>() / nn;
            let pe = bots.iter().map(|b| b.summary_perceived() as f32).sum::<f32>() / nn;
            let pmax = bots.iter().map(|b| b.summary_perceived()).max().unwrap_or(0);
            if ramp_ticks > 0 {
                // Arrivée progressive : on imprime aussi les ARRIVÉS (déterministe via `join_tick`) et la
                // perception/ARRIVÉ → honnête (les pas-encore-là perçoivent 0 et fausseraient la moyenne globale).
                let arr = (0..bots.len()).filter(|&i| tick >= join_tick(i)).count().max(1);
                let pe_arr = bots.iter().map(|b| b.summary_perceived() as f32).sum::<f32>() / arr as f32;
                println!("  [t={:>4.0}s] arrivés {arr:>5} | pairs connus {kn:>6.1} | trous {ho:>6.1} | perception moy/tous {pe:>6.0} moy/arrivé {pe_arr:>6.0} max {pmax}", now);
            } else {
                println!("  [t={:>4.0}s] pairs connus {kn:>6.1} | trous ouverts {ho:>6.1} | perception moy {pe:>6.0} max {pmax}", now);
            }
        }
    }
    let wall_s = wall.elapsed().as_secs_f32();

    // Bilan — MÊMES métriques que `crowd` (perception par résumé, débit/nœud) → comparable (T0.2-bis).
    let n = bots.len() as f32;
    let avg_neighbors = bots.iter().map(|b| b.neighbors() as f32).sum::<f32>() / n;
    let avg_summary = bots.iter().map(|b| b.summary_perceived() as f32).sum::<f32>() / n;
    let max_summary = bots.iter().map(|b| b.summary_perceived()).max().unwrap_or(0);
    let up = bots.iter().map(|b| b.bytes_up()).sum::<u64>().saturating_sub(up0) as f32;
    let down = bots.iter().map(|b| b.bytes_down()).sum::<u64>().saturating_sub(down0) as f32;
    println!("-------- BANC BUS : {ticks} ticks = {secs}s SIM joués en {wall_s:.1}s mural --------");
    println!("Pairs CONNUS moyen    : {avg_neighbors:.1}/nœud (si ~0 à grand N → la DÉCOUVERTE échoue, pas les résumés)");
    println!("Perception par RÉSUMÉ : moy {avg_summary:.0}, max {max_summary} occupants via 1 flux (foule {})", bots.len());
    println!("Débit par nœud        : ↑{:.1} / ↓{:.1} Ko/s", up / 1000.0 / n / secs as f32, down / 1000.0 / n / secs as f32);
    println!("=> T0.2-bis : compare à `crowd {}`. Si proche → banc bus FIDÈLE → extrapolation 5k-50k permise.", bots.len());
    report_graph_structure(&bots);
    report_summary_rejections(&bots);
}

/// DIAGNOSTIC D'INGESTION DES RÉSUMÉS (D25, demandé le 20 juin) : POURQUOI les résumés sont
/// acceptés/rejetés, agrégé sur tous les nœuds. Tranche deux hypothèses du mur à 5000 :
///   - `reçus ≈ 0` → ce n'est PAS D26 : la DÉCOUVERTE/PERÇÉE ne livre pas les résumés.
///   - `reçus ≫ 0` mais rejet dominé par `émetteur≠hôte` → suspect D26 couche 1 CONFIRMÉ (à
///     découverte sparse, chaque nœud calcule un hôte attendu différent → rejette du légitime).
/// Lecture SEULE : ne change rien au comportement.
fn report_summary_rejections(bots: &[Bot]) {
    let mut recus = 0u64;
    let (mut acc, mut host, mut sig, mut stale, mut full) = (0u64, 0u64, 0u64, 0u64, 0u64);
    for b in bots {
        let s = b.summary_stats();
        recus += s.received();
        acc += s.accepted;
        host += s.rej_host;
        sig += s.rej_sig;
        stale += s.rej_stale;
        full += s.rej_full;
    }
    let pct = |x: u64| if recus > 0 { x as f32 / recus as f32 * 100.0 } else { 0.0 };
    println!("-------- INGESTION DES RÉSUMÉS (pourquoi accepté/rejeté, tous nœuds) --------");
    println!("Résumés REÇUS (total) : {recus}");
    println!("  ✅ acceptés         : {acc} ({:.0}%)", pct(acc));
    println!("  ❌ émetteur≠hôte    : {host} ({:.0}%)   ← la dette D26 couche 1 (vue locale de l'hôte)", pct(host));
    println!("  ❌ sceau invalide   : {sig} ({:.0}%)", pct(sig));
    // Le compteur `rej_stale` est partagé, mais sa SIGNIFICATION dépend du mode (honnêteté de mesure,
    // 8.3★ étape C-diag) : sous DENSITY_MAX on rejette « count ≤ max déjà vu » (pas plus DENSE), sinon
    // « seq ≤ existant » (pas plus FRAIS). On affiche le bon libellé pour ne pas se mé-relire plus tard.
    let stale_label = if crate::net::link::density_corrob_mode() {
        "count pas plus frais de CE signataire (anti-rejeu par seq/signataire)"
    } else if crate::net::link::density_max_mode() {
        "pas plus DENSE (count ≤ max vu — normal sous DENSITY_MAX, redondance épidémique)"
    } else {
        "même hôte, seq ≤ existant"
    };
    println!("  ❌ pas plus frais   : {stale} ({:.0}%)   ({stale_label})", pct(stale));
    println!("  ❌ table pleine     : {full} ({:.0}%)   (MAX_CELLS)", pct(full));
    if recus == 0 {
        println!("=> 0 résumé reçu : le mur est la DÉCOUVERTE/PERÇÉE, PAS l'ingestion (D26 hors de cause ici).");
    } else if crate::net::link::density_corrob_mode() {
        println!("=> CORROB : taxe émetteur≠hôte = 0 % (pas d'élection) ; densité = Σ (Q-ième plus grand count /signataire) → anti-inflation.");
    } else if crate::net::link::density_max_mode() {
        println!("=> DENSITY_MAX : taxe émetteur≠hôte attendue à 0 % (contrôle d'hôte relâché) ; densité = Σ counts (MAX/cellule).");
    } else if host >= acc && host >= sig && host >= stale {
        println!("=> Rejet dominé par émetteur≠hôte : suspect D26 couche 1 (vues locales divergentes) PLAUSIBLE.");
    }
}

// --- Union-find (composantes connexes du graphe de communication) ---
fn uf_find(p: &mut [usize], mut x: usize) -> usize {
    while p[x] != x {
        p[x] = p[p[x]]; // compression de chemin
        x = p[x];
    }
    x
}
fn uf_union(p: &mut [usize], a: usize, b: usize) {
    let (ra, rb) = (uf_find(p, a), uf_find(p, b));
    if ra != rb {
        p[ra] = rb;
    }
}

/// DIAGNOSTIC DE PERCOLATION (D25, demandé par l'utilisateur le 20 juin) : la perception se propage
/// le long des TROUS OUVERTS (un résumé ne relaie qu'aux pairs au trou ouvert). On reconstruit donc
/// le GRAPHE de communication (arête A-B si l'un a le trou de l'autre ouvert) et on mesure sa
/// STRUCTURE — pas un chiffre de perf, mais la RAISON d'un éventuel effondrement : si le réseau se
/// fragmente en grappes qui ne se rencontrent jamais, aucune perception globale n'est possible.
/// Lecture SEULE : ne change rien au comportement.
fn report_graph_structure(bots: &[Bot]) {
    let n = bots.len();
    // Index par identité (les bots sans id — jamais arrivés au WELCOME — comptent comme isolés).
    let mut idx: HashMap<PeerId, usize> = HashMap::new();
    for (i, b) in bots.iter().enumerate() {
        if let Some(id) = b.id() {
            idx.insert(id, i);
        }
    }
    let sans_id = n - idx.len();

    let mut parent: Vec<usize> = (0..n).collect();
    let mut total_holes = 0usize;
    let mut isoles = 0usize; // 0 trou ouvert vers un bot connu de ce banc
    for (i, b) in bots.iter().enumerate() {
        let mut deg = 0usize;
        for h in b.open_holes() {
            if let Some(&j) = idx.get(&h) {
                uf_union(&mut parent, i, j);
                deg += 1;
            }
        }
        total_holes += deg;
        if deg == 0 {
            isoles += 1;
        }
    }
    // Tailles des composantes.
    let mut sizes: HashMap<usize, usize> = HashMap::new();
    for i in 0..n {
        let r = uf_find(&mut parent, i);
        *sizes.entry(r).or_insert(0) += 1;
    }
    let n_comp = sizes.len();
    let mut tailles: Vec<usize> = sizes.into_values().collect();
    tailles.sort_unstable_by(|a, b| b.cmp(a)); // décroissant
    let plus_grande = tailles.first().copied().unwrap_or(0);
    let frac = if n > 0 { plus_grande as f32 / n as f32 * 100.0 } else { 0.0 };
    let avg_deg = if n > 0 { total_holes as f32 / n as f32 } else { 0.0 };

    println!("-------- STRUCTURE DU GRAPHE DE COMMUNICATION (trous ouverts) --------");
    println!("Trous ouverts moyen   : {avg_deg:.1}/nœud  (l'arête = je peux relayer à ce pair)");
    println!("Nœuds ISOLÉS          : {isoles}/{n} (aucun trou ouvert{}).", if sans_id > 0 { format!(", dont {sans_id} jamais arrivés au WELCOME") } else { String::new() });
    println!("Composantes connexes  : {n_comp}  |  PLUS GRANDE = {plus_grande} nœuds ({frac:.0}% du total)");
    let apercu: Vec<usize> = tailles.iter().take(8).copied().collect();
    println!("Tailles (8 premières) : {apercu:?}");
    if frac >= 90.0 {
        println!("=> Un seul grand bloc : le réseau PERCOLE (pas de fragmentation).");
    } else {
        println!("=> ⚠ FRAGMENTÉ : la plus grande grappe ne couvre que {frac:.0}% → percolation INCOMPLÈTE (piste du mur).");
    }
}
