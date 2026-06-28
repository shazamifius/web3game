//! L'AGENT DE MESURE (v0) — le « mètre étalon » du *« est-ce vivant ? »*.
//!
//! Notre banc headless ne sait PAS mesurer la seule chose qui décide « vivant vs mort » :
//! la **fraîcheur ressentie** d'un avatar distant sur un VRAI lien (perte, gigue, NAT) —
//! c'est le doute D27 (« la forteresse vide »). Cet agent remplace l'œil humain (« ça
//! saccade », « il a 4 s de retard ») par des **chiffres**, plus précis et reproductibles.
//!
//! Tout part d'un fait : chaque état porte un `seq` MONOTONE (l'anti-rejeu, déjà là). Du
//! point de vue d'un observateur, la suite des `seq` reçus suffit à TOUT déduire d'un lien,
//! sans 3D et sans humain :
//!   - **perte** = trous dans les `seq` ;
//!   - **ré-ordonnancement** = un `seq` qui recule ;
//!   - **gigue (jitter)** = irrégularité des intervalles d'arrivée ;
//!   - **fraîcheur** = l'ÂGE du dernier état connu, échantillonné dans le temps (la grandeur
//!     reine : cible ≤ 500 ms = jouable ; au-delà = mort).
//!
//! **Statut.** La logique de mesure est prouvée par tests déterministes ET branchée sur de VRAIS
//! pairs : `agent recv`/`loop` rejoignent le rendez-vous, et chaque état accepté est journalisé
//! `(recv_ms, seq)` (cf. `Bot::take_link_arrivals`). À chaque fenêtre, `link_stats` chiffre alors
//! **perte / ré-ordre / gigue** par pair, à côté de la fraîcheur — l'instrument ne dit plus seulement
//! « est-ce vivant ? » mais **POURQUOI** (paquets perdus ? gigue ? ré-ordre ?). Ce qu'il ne fait PAS
//! encore (honnêteté) : le score « robotique » (l'ampleur des corrections de dead-reckoning), qui a
//! besoin du modèle d'interpolation d'Unreal → il arrivera avec le branchement sidecar.

use super::bot::Bot;
use super::crypto::PeerId;
use std::time::Instant;

/// Un événement d'ARRIVÉE d'état distant, vu par un observateur : QUAND on l'a reçu
/// (ms depuis le début de la mesure) et le `seq` de l'émetteur (monotone).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Arrival {
    pub recv_ms: f64,
    pub seq: u64,
}

/// Les statistiques de LIEN d'une paire (observateur ← émetteur), indépendantes du moteur 3D.
/// C'est « ce que l'œil dirait », chiffré.
#[derive(Clone, Debug, Default)]
pub(crate) struct LinkStats {
    pub received: usize,   // nombre de paquets reçus
    pub expected: u64,     // attendus sur la plage de seq (max − min + 1) — RÉFÉRENCE PLEIN DÉBIT
    pub loss_pct: f64,     // perte APPARENTE : 1 − reçus / attendus (inclut le bridage AoI !)
    pub real_loss_pct: f64, // perte RÉELLE : relative à la cadence INFÉRÉE (hors bridage volontaire)
    pub cadence_step: u64, // pas de seq inféré entre deux envois reçus (1 = plein débit ; 10 ≈ bridé 2 Hz)
    pub reorder_pct: f64,  // fraction d'arrivées dont le seq recule
    pub jitter_ms: f64,    // écart absolu moyen des intervalles inter-arrivées
    pub fresh_p50_ms: f64, // FRAÎCHEUR (âge du dernier état connu) — médiane
    pub fresh_p95_ms: f64, // p95
    pub fresh_max_ms: f64, // pire cas
}

/// Le p-ième centile d'un tableau DÉJÀ TRIÉ (rang le plus proche). `p` ∈ [0, 100].
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let rank = (p / 100.0 * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[rank.min(sorted.len() - 1)]
}

/// Les ÂGES de fraîcheur : on balaie le temps par pas de `tick_ms` (la cadence à laquelle
/// l'observateur « regarde »), et à chaque instant on note l'âge du DERNIER état reçu. Une
/// dent de scie : 0 juste après une arrivée, qui monte jusqu'à la suivante. `arrivals` doit
/// être trié par `recv_ms` croissant.
fn freshness_ages(arrivals: &[Arrival], tick_ms: f64) -> Vec<f64> {
    if arrivals.len() < 2 || tick_ms <= 0.0 {
        return vec![0.0];
    }
    let t0 = arrivals[0].recv_ms;
    let t_end = arrivals[arrivals.len() - 1].recv_ms;
    let mut ages = Vec::new();
    let mut idx = 0usize;
    let mut t = t0;
    while t <= t_end + 1e-9 {
        while idx + 1 < arrivals.len() && arrivals[idx + 1].recv_ms <= t {
            idx += 1;
        }
        ages.push(t - arrivals[idx].recv_ms);
        t += tick_ms;
    }
    ages
}

/// Calcule les stats de lien à partir des arrivées brutes et de la cadence d'observation.
/// `tick_ms` = tous les combien l'observateur « regarde » (typiquement le pas de rendu, ~16 ms).
pub(crate) fn link_stats(arrivals: &[Arrival], tick_ms: f64) -> LinkStats {
    if arrivals.is_empty() {
        return LinkStats::default();
    }
    // On travaille sur une copie triée par instant de réception (robuste si non trié).
    let mut by_time = arrivals.to_vec();
    by_time.sort_by(|a, b| a.recv_ms.partial_cmp(&b.recv_ms).unwrap_or(std::cmp::Ordering::Equal));

    let received = by_time.len();
    let min_seq = by_time.iter().map(|a| a.seq).min().unwrap();
    let max_seq = by_time.iter().map(|a| a.seq).max().unwrap();
    let expected = max_seq - min_seq + 1;
    let loss_pct = (1.0 - received as f64 / expected as f64).max(0.0);

    // VRAIE PERTE vs BRIDAGE VOLONTAIRE (l'enquête « inspecteur Eve », 28 juin).
    // L'émetteur incrémente son seq à plein débit (SEND_HZ, 20/s) pour TOUS, mais n'émet vers un
    // pair LOINTAIN qu'à CONSCIENCE_HZ (2/s) par l'AoI : ce pair voit seq 1,11,21… → `loss_pct`
    // (vs seq global) le compte « perdu » alors que RIEN ne l'est. On INFÈRE le pas de cadence
    // (médiane des sauts de seq consécutifs, robuste : une vraie perte fait un saut ~double) et on
    // mesure la perte RELATIVE à cette cadence : saut ≈ 1 pas = normal ; ≈ 2 pas = 1 envoi vraiment perdu.
    let mut by_seq: Vec<u64> = by_time.iter().map(|a| a.seq).collect();
    by_seq.sort_unstable();
    by_seq.dedup();
    let step_gaps: Vec<u64> = by_seq.windows(2).map(|w| w[1] - w[0]).collect();
    let (cadence_step, real_loss_pct) = if step_gaps.len() < 2 {
        (1, 0.0) // pas assez d'arrivées pour inférer une cadence → on ne prétend rien
    } else {
        let mut sorted_gaps = step_gaps.clone();
        sorted_gaps.sort_unstable();
        let base = sorted_gaps[sorted_gaps.len() / 2].max(1); // médiane (≥1) = le pas de cadence
        let mut slots = 0u64; // nb de créneaux d'émission attendus À CETTE CADENCE
        let mut missing = 0u64; // créneaux manquants = vraies pertes
        for &g in &step_gaps {
            let k = ((g as f64 / base as f64).round() as u64).max(1);
            slots += k;
            missing += k - 1;
        }
        let rl = if slots > 0 { missing as f64 / slots as f64 } else { 0.0 };
        (base, rl)
    };

    // Ré-ordonnancement : un seq qui recule par rapport à l'arrivée précédente (en temps).
    let reorders = by_time.windows(2).filter(|w| w[1].seq < w[0].seq).count();
    let reorder_pct = if received > 1 {
        reorders as f64 / (received - 1) as f64
    } else {
        0.0
    };

    // Gigue : écart absolu moyen des intervalles inter-arrivées autour de leur moyenne.
    let gaps: Vec<f64> = by_time.windows(2).map(|w| w[1].recv_ms - w[0].recv_ms).collect();
    let jitter_ms = if gaps.is_empty() {
        0.0
    } else {
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        gaps.iter().map(|g| (g - mean).abs()).sum::<f64>() / gaps.len() as f64
    };

    // Fraîcheur : distribution des âges.
    let mut ages = freshness_ages(&by_time, tick_ms);
    ages.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    LinkStats {
        received,
        expected,
        loss_pct,
        real_loss_pct,
        cadence_step,
        reorder_pct,
        jitter_ms,
        fresh_p50_ms: percentile(&ages, 50.0),
        fresh_p95_ms: percentile(&ages, 95.0),
        fresh_max_ms: ages.last().copied().unwrap_or(0.0),
    }
}

/// Le rapport d'une paire en JSON (à la main : on n'ajoute pas de dépendance pour ça).
/// C'est le format que les agents enverront au collecteur (un objet par paire observée).
pub(crate) fn report_json(observer: &str, target: &str, s: &LinkStats) -> String {
    format!(
        "{{\"observer\":\"{observer}\",\"target\":\"{target}\",\"received\":{},\"expected\":{},\
         \"loss_pct\":{:.2},\"real_loss_pct\":{:.2},\"cadence_step\":{},\"reorder_pct\":{:.2},\"jitter_ms\":{:.1},\
         \"fresh_p50_ms\":{:.1},\"fresh_p95_ms\":{:.1},\"fresh_max_ms\":{:.1}}}",
        s.received,
        s.expected,
        s.loss_pct * 100.0,
        s.real_loss_pct * 100.0,
        s.cadence_step,
        s.reorder_pct * 100.0,
        s.jitter_ms,
        s.fresh_p50_ms,
        s.fresh_p95_ms,
        s.fresh_max_ms,
    )
}

/// L'agent (v0). Sans argument → DÉMO (flux synthétiques, le format de rapport). `recv [secs]` →
/// mesure LIVE : on rejoint le rendez-vous comme un vrai nœud et on chiffre la fraîcheur des pairs.
/// Réglages que l'AGENT doit avoir QUOI QU'IL ARRIVE (il vit sur des liens CGNAT) : le repli relais
/// et la difficulté PoW du réseau. On les pose SI ABSENTS → un agent persistant lancé par le shim
/// (Windows) ou le service (Linux), SANS l'environnement du `.bat`, fonctionne quand même (relais +
/// identité valides). N'écrase JAMAIS un réglage explicite de l'utilisateur.
fn ensure_agent_env() {
    if std::env::var("RELAY_FALLBACK").is_err() {
        unsafe { std::env::set_var("RELAY_FALLBACK", "1") };
    }
    if std::env::var("POW_BITS").is_err() {
        unsafe { std::env::set_var("POW_BITS", "18") };
    }
}

pub fn run_agent(mode: Option<&str>, secs: u64) {
    match mode {
        Some("install") => run_agent_install(false),
        Some("uninstall") => run_agent_install(true),
        Some("recv") => {
            ensure_agent_env();
            ensure_rendezvous_from_file();
            run_agent_recv(secs)
        }
        Some("loop") => {
            ensure_agent_env();
            ensure_rendezvous_from_file();
            run_agent_loop(secs)
        }
        _ => run_agent_demo(),
    }
}

/// v0 — démonstration du MÈTRE ÉTALON sur deux flux synthétiques (un bon lien, un mauvais).
/// Montre le format de rapport qu'on récoltera. Prochain pas : nourrir `link_stats` avec les
/// VRAIES arrivées d'un pair (rendez-vous + émetteur à trajectoire connue), sur tes box.
fn run_agent_demo() {
    // BON lien : 20 Hz réguliers, aucune perte.
    let bon: Vec<Arrival> = (0..100).map(|i| Arrival { recv_ms: i as f64 * 50.0, seq: i }).collect();

    // MAUVAIS lien : ~10 % de perte, de la gigue, un ré-ordonnancement.
    let mut mauvais: Vec<Arrival> = Vec::new();
    let mut t = 0.0;
    for i in 0..100u64 {
        t += 50.0 + ((i % 7) as f64 - 3.0) * 12.0; // intervalles irréguliers (gigue)
        if i % 10 == 3 {
            continue; // un paquet sur dix perdu
        }
        mauvais.push(Arrival { recv_ms: t, seq: i });
    }
    if mauvais.len() > 20 {
        // un vrai ré-ordre : on échange les INSTANTS d'arrivée de deux paquets (le seq plus
        // grand arrive avant le plus petit), sans toucher aux seq → recul détecté après tri.
        let (t15, t16) = (mauvais[15].recv_ms, mauvais[16].recv_ms);
        mauvais[15].recv_ms = t16;
        mauvais[16].recv_ms = t15;
    }

    // EXTRÊME lien : 90 % de perte (1 paquet sur 10 passe).
    let mut extreme_90: Vec<Arrival> = Vec::new();
    let mut t_ex = 0.0;
    for i in 0..100u64 {
        t_ex += 50.0;
        if i % 10 != 0 {
            continue; // 90 % de perte (seul 1 paquet sur 10 passe)
        }
        extreme_90.push(Arrival { recv_ms: t_ex, seq: i });
    }

    let tick = 16.0; // l'observateur « regarde » à ~60 Hz (pas de rendu)
    println!("agent v0 — mètre étalon (flux synthétiques ; cible fraîcheur ≤ 500 ms)");
    println!("{}", report_json("moi", "lien_bon", &link_stats(&bon, tick)));
    println!("{}", report_json("moi", "lien_mauvais", &link_stats(&mauvais, tick)));
    println!("{}", report_json("moi", "lien_extreme_90pct", &link_stats(&extreme_90, tick)));
}

/// MESURE LIVE (v0) : un VRAI nœud qui rejoint le rendez-vous, écoute `secs` secondes, et chiffre
/// la FRAÎCHEUR de chaque pair entendu (âge du dernier état reçu). C'est le « est-ce vivant » sur un
/// vrai lien — l'angle mort du banc headless (D27). L'émetteur en face = un simple `bot`. Rendez-vous
/// = 127.0.0.1 par défaut (sinon `RENDEZVOUS_ADDR=ip:port` pour le cross-machine). Un JSON par pair.
fn run_agent_recv(secs: u64) {
    let mut bot = match Bot::new("agent", false, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[agent] réseau indisponible (le rendez-vous est-il joignable ?).");
            return;
        }
    };
    println!("[agent] mesure LIVE pendant {secs}s — fraîcheur des pairs (cible ≤ 500 ms = vivant)");

    // Fenêtre de CHAUFFE : on ne chiffre pas la cérémonie de connexion (découverte + perçage),
    // qui pollue la traîne. On mesure le RÉGIME établi, pas le démarrage.
    const WARMUP_S: f64 = 3.0;
    println!("[agent] (les {WARMUP_S:.0} 1res s sont exclues — chauffe découverte/perçage)");

    let start = Instant::now();
    let mut last = Instant::now();
    let mut samples: std::collections::HashMap<PeerId, Vec<f64>> = std::collections::HashMap::new();
    while start.elapsed().as_secs() < secs {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        let now = start.elapsed().as_secs_f32();
        bot.step(dt, now);
        if start.elapsed().as_secs_f64() >= WARMUP_S {
            for (id, age) in bot.peer_freshness_ms() {
                samples.entry(id).or_default().push(age);
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let links = link_stats_by_peer(bot.take_link_arrivals(), 16.0);
    report_freshness("", &samples, &links);
}

/// Construit le rapport par pair (un JSON), l'IMPRIME, et RENVOIE les lignes (pour l'upload). Partagé
/// par `recv` et `loop`. `ts` = horodatage époque (vide pour une mesure unique). La FRAÎCHEUR vient du
/// VERDICT de vivacité, CADENCE-CONSCIENT (« inspecteur Eve » saison 2, 28 juin). Un seuil plat de
/// 500 ms MENT : un pair en palier CONSCIENCE (bridé ~2 Hz EXPRÈS, le « filet ») est « stale » SANS
/// être mort. Trois états honnêtes, en lisant la RÉCEPTION réelle de la fenêtre :
///  • `recv == 0` (pair connu mais AUCUNE arrivée) → SILENCIEUX : le vrai suspect (relais / inclusivité) ;
///  • cadence BRIDÉE (≥ 4×) OU perte RÉELLE faible (≤ 20 %) → LOINTAIN basse fidélité = VIVANT : il
///    délivre fiablement ce qu'il promet à sa cadence, juste lointain — PAS mort ;
///  • réellement lossy (perte RÉELLE élevée, ex. CGNAT non perçable) ET en retard → vraiment MORT.
///
/// Levier A (29 juin) : ne plus DIFFAMER un lien `real_loss~0`. La session 112 a montré que ~moitié des
/// `MORT(>500ms)` étaient des liens SAINS (recv=expected, real_loss 0, bas débit) que le verdict tuait à
/// tort en ne lisant QUE la fraîcheur brute. Les deux populations sont nettes (sain ≈ 0 % vs CGNAT lossy
/// 50-81 %) → tout seuil dans [10,40] % les sépare ; on prend 20 % (marge confortable).
const REAL_LOSS_MORT_PCT: f64 = 0.20;

fn liveness_verdict(fresh_p95_ms: f64, recv: usize, cadence_step: u64, real_loss_pct: f64) -> &'static str {
    if fresh_p95_ms <= 500.0 {
        "vivant"
    } else if recv == 0 {
        "MORT(silencieux)"
    } else if cadence_step >= 4 || real_loss_pct <= REAL_LOSS_MORT_PCT {
        "lointain(basse-fidelite)"
    } else {
        "MORT(>500ms)"
    }
}

/// sondage par tick (`samples`, le verdict éprouvé en réel) ; perte/ré-ordre/gigue viennent du journal
/// d'arrivées `links` (chiffré par `link_stats` à partir des `seq`) → l'instrument complet, pas juste
/// « est-ce vivant » mais « POURQUOI » (paquets perdus ? gigue ? ré-ordre ?). `links` peut être vide
/// (pair entendu mais aucune arrivée chiffrée) → on ne sort alors que la fraîcheur, jamais de crash.
fn report_freshness(
    ts: &str,
    samples: &std::collections::HashMap<PeerId, Vec<f64>>,
    links: &std::collections::HashMap<PeerId, LinkStats>,
) -> Vec<String> {
    let tsf = if ts.is_empty() { String::new() } else { format!("\"ts\":{ts},") };
    let mut lines = Vec::new();
    if samples.is_empty() {
        lines.push(format!("{{{tsf}\"note\":\"aucun pair vu\"}}"));
    } else {
        for (id, ages) in samples {
            let mut a = ages.clone();
            a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            let p95 = percentile(&a, 95.0);
            // VERDICT CADENCE-CONSCIENT : on lit la RÉCEPTION réelle de la fenêtre (pas juste un seuil
            // plat de 500 ms qui mentirait sur un pair bridé EXPRÈS). cf. `liveness_verdict`.
            let link = links.get(id);
            let recv = link.map(|s| s.received).unwrap_or(0);
            let cadence = link.map(|s| s.cadence_step).unwrap_or(0);
            let real_loss = link.map(|s| s.real_loss_pct).unwrap_or(0.0);
            let verdict = liveness_verdict(p95, recv, cadence, real_loss);
            // Qualité de lien : `recv` apparaît TOUJOURS (0 = silence VISIBLE → on voit le vrai
            // problème), avec perte/gigue/cadence quand on a chiffré des arrivées pour ce pair.
            let quality = match link {
                Some(s) => format!(
                    "\"recv\":{},\"expected\":{},\"loss_pct\":{:.1},\"real_loss_pct\":{:.1},\"cadence_step\":{},\"reorder_pct\":{:.1},\"jitter_ms\":{:.1},",
                    s.received, s.expected, s.loss_pct * 100.0, s.real_loss_pct * 100.0, s.cadence_step, s.reorder_pct * 100.0, s.jitter_ms
                ),
                None => "\"recv\":0,".to_string(),
            };
            lines.push(format!(
                "{{{tsf}\"observer\":\"agent\",\"target\":\"{}\",\"samples\":{},{}\"fresh_p50_ms\":{:.0},\
                 \"fresh_p95_ms\":{:.0},\"fresh_max_ms\":{:.0},\"verdict\":\"{}\"}}",
                id.short(), a.len(), quality, percentile(&a, 50.0), p95, a.last().copied().unwrap_or(0.0), verdict
            ));
        }
    }
    for l in &lines {
        println!("{l}");
    }
    lines
}

/// Chiffre les stats de LIEN (perte/gigue/ré-ordre/fraîcheur-paquet) par pair à partir du journal
/// d'arrivées drainé du bot. `tick_ms` = cadence d'observation (pas de rendu). Pairs sans arrivée
/// suffisante → ignorés (link_stats par défaut, peu informatif). Pur (testable, sans réseau).
fn link_stats_by_peer(
    arrivals: std::collections::HashMap<PeerId, Vec<Arrival>>,
    tick_ms: f64,
) -> std::collections::HashMap<PeerId, LinkStats> {
    arrivals
        .into_iter()
        .filter(|(_, v)| v.len() >= 2) // <2 arrivées = aucune perte/gigue chiffrable
        .map(|(id, v)| (id, link_stats(&v, tick_ms)))
        .collect()
}

/// POST HTTP/1.0 minimaliste (upload des résultats), SANS dépendance. true si envoyé.
fn http_post(host: &str, port: u16, path: &str, body: &str) -> bool {
    use std::io::{Read, Write};
    let timeout = std::time::Duration::from_secs(5);
    let mut stream = match std::net::TcpStream::connect((host, port)) {
        Ok(s) => s,
        Err(_) => return false,
    };
    let _ = stream.set_write_timeout(Some(timeout));
    let _ = stream.set_read_timeout(Some(timeout));
    let req = format!(
        "POST {path} HTTP/1.0\r\nHost: {host}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    if stream.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut sink = Vec::new();
    let _ = stream.read_to_end(&mut sink); // on lit (et ignore) l'accusé
    true
}

/// Port HTTP où le serveur sert la CAMPAGNE (sur la même machine que le rendez-vous).
const CONFIG_PORT: u16 = 24001;

/// Un nom de machine LISIBLE pour la présence (« le PC de X »), sans dépendance : `COMPUTERNAME`
/// sous Windows, `/etc/hostname` sinon, repli `HOSTNAME` puis « inconnu ». Nettoyé (pas de guillemet
/// ni de saut de ligne) pour rester un JSON sûr. Aucune info sensible — juste de quoi repérer les
/// créneaux de dispo (« mardi 16 h, tout le monde est là »).
fn host_label() -> String {
    let raw = std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/hostname").ok())
        .or_else(|| std::env::var("HOSTNAME").ok())
        .unwrap_or_default();
    let cleaned: String = raw.trim().chars().filter(|c| *c != '"' && *c != '\n' && *c != '\r').take(64).collect();
    if cleaned.is_empty() { "inconnu".to_string() } else { cleaned }
}

/// La ligne de PRÉSENCE (battement de cœur), JSON fait main. PUR (testable) : un horodatage, le nom
/// du PC, la version de l'agent, l'événement. Volontairement MINUSCULE → coût réseau négligeable au
/// repos (« peu de connexion hors simulation »). Le serveur l'empile dans `presence.ndjson`.
fn heartbeat_json(ts: u64, host: &str, ver: &str, ev: &str, diag: &str) -> String {
    format!("{{\"ts\":{ts},\"host\":\"{host}\",\"ver\":\"{ver}\",\"ev\":\"{ev}\"{diag}}}")
}

/// Envoie un battement de cœur au collecteur (POST /heartbeat). Best-effort : un échec est silencieux
/// (serveur muet → on n'insiste pas, l'agent ne se bloque jamais). C'est l'observabilité « qui est en
/// ligne, quand » SANS lancer de simulation — juste savoir quels PC sont dispo et à quelles heures.
fn send_heartbeat(host_addr: &str, port: u16, ev: &str) {
    send_heartbeat_diag(host_addr, port, ev, "");
}

/// Battement ENRICHI (observabilité, 28 juin) : `diag` = champs JSON supplémentaires (ex.
/// `,"peers":3,"recv":120,"sent":5`) collés tels quels avant la `}`. Sert à VOIR à distance ce qu'un
/// agent ami fait vraiment (combien de pairs il voit, combien d'états il reçoit, combien il uploade) —
/// fini la chasse aux captures d'écran. `diag` vide = battement simple. Best-effort, jamais bloquant.
fn send_heartbeat_diag(host_addr: &str, port: u16, ev: &str, diag: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = heartbeat_json(ts, &host_label(), &agent_version(), ev, diag);
    let _ = http_post(host_addr, port, "/heartbeat", &body);
}

/// Le MODE de la campagne. `Idle` (DÉFAUT) = repos : l'agent ne se connecte PAS au P2P, il ne fait
/// qu'un battement de cœur léger → quasi zéro réseau/CPU, le pote n'est jamais dérangé. `Simulate` =
/// une session de mesure est DEMANDÉE → l'agent demande le CONSENTEMENT (popup) avant de mesurer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Idle,
    Simulate,
}

/// La CAMPAGNE : ce que l'agent doit faire, décidé CENTRALEMENT (je l'édite sur le serveur, les
/// agents suivent — brique 2). Format « clé=valeur » par ligne → zéro dépendance JSON (fait-main).
/// `session` = identifiant de session : tant qu'il ne change pas, on ne re-demande PAS le consentement
/// (une question par session). Pour relancer une demande, je bumpe `session` sur le serveur.
#[derive(Clone, Copy, Debug)]
struct Campaign {
    window: u64,
    mode: Mode,
    session: u64,
    bots: usize,
    /// COUCHE 2 — allume l'AoI BILATÉRALE le temps de la session (`aoi=1`). Défaut `false` → émission
    /// byte-pour-byte. Permet de PROUVER la couche 2 dehors par un simple flip serveur, sans rebuild.
    aoi: bool,
}
impl Default for Campaign {
    fn default() -> Self {
        Campaign { window: 30, mode: Mode::Idle, session: 0, bots: 1, aoi: false }
    }
}

/// Parse une campagne « clé=valeur ». ROBUSTE : tout champ absent/illisible garde le défaut, et on
/// ignore l'inconnu → l'agent ne casse JAMAIS sur une config foireuse (self-sufficient). `mode`
/// inconnu → `idle` (le repos = le choix le PLUS sûr pour le pote, jamais de mesure surprise).
fn parse_campaign(body: &str) -> Campaign {
    let mut c = Campaign::default();
    for line in body.lines() {
        if let Some((k, v)) = line.trim().split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            match k {
                "window" => {
                    if let Ok(n) = v.parse::<u64>() {
                        c.window = n.clamp(5, 3600);
                    }
                }
                "mode" => {
                    c.mode = if v.eq_ignore_ascii_case("simulate") { Mode::Simulate } else { Mode::Idle };
                }
                "session" => {
                    if let Ok(n) = v.parse::<u64>() {
                        c.session = n;
                    }
                }
                "bots" => {
                    if let Ok(n) = v.parse::<usize>() {
                        c.bots = n.clamp(1, 1000);
                    }
                }
                "aoi" => {
                    c.aoi = matches!(v, "1" | "true"); // couche 2 ON le temps de la session
                }
                _ => {}
            }
        }
    }
    c
}

/// GET HTTP/1.0 BINAIRE, SANS dépendance (std seulement) — sert au fetch de campagne ET au
/// téléchargement du nouveau binaire (auto-update). Renvoie le CORPS (octets) sur 200, sinon None.
/// Lit l'en-tête `Content-Length` (insensible à la casse) dans le bloc d'en-têtes HTTP.
/// None s'il est absent → l'appelant garde l'ancien comportement (corps tel quel).
fn parse_content_length(headers: &[u8]) -> Option<usize> {
    let text = std::str::from_utf8(headers).ok()?;
    for line in text.split("\r\n") {
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                return v.trim().parse::<usize>().ok();
            }
        }
    }
    None
}

fn http_get_bytes(host: &str, port: u16, path: &str) -> Option<Vec<u8>> {
    use std::io::{Read, Write};
    let timeout = std::time::Duration::from_secs(10); // un binaire est gros → marge
    let mut stream = std::net::TcpStream::connect((host, port)).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp).ok()?;
    let pos = resp.windows(4).position(|w| w == b"\r\n\r\n")?; // fin des en-têtes
    if !resp.starts_with(b"HTTP/1.0 200") && !resp.starts_with(b"HTTP/1.1 200") {
        return None; // 404 ou autre → on ne renvoie rien
    }
    let body = &resp[pos + 4..];
    // INTÉGRITÉ (anti-brick sur lien bas débit, ex. mobile instable) : si le serveur
    // ANNONCE une taille (Content-Length), on EXIGE un corps complet. Un téléchargement
    // coupé en route → None : on garde l'ancien binaire qui tourne et on réessaiera,
    // plutôt que d'installer un .exe TRONQUÉ (qui briquerait l'agent en silence).
    if let Some(declared) = parse_content_length(&resp[..pos]) {
        if body.len() < declared {
            return None; // tronqué → refus net
        }
        return Some(body[..declared].to_vec()); // exactement la taille annoncée
    }
    Some(body.to_vec())
}

/// GET texte (campagne) — enveloppe de `http_get_bytes`. None si le serveur ne répond pas
/// (l'agent garde alors sa config courante → self-sufficient).
fn http_get(host: &str, port: u16, path: &str) -> Option<String> {
    http_get_bytes(host, port, path).map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// Si `RENDEZVOUS_ADDR` n'est pas dans l'environnement, on le lit dans `serveur.txt` à côté de
/// l'exe. INDISPENSABLE à l'auto-démarrage : le service/tâche n'a pas l'env du `.bat` → l'agent se
/// configure SEUL depuis le fichier. (Appelé tôt, mono-thread → set_var sûr.)
fn ensure_rendezvous_from_file() {
    if std::env::var("RENDEZVOUS_ADDR").is_ok() {
        return;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if let Ok(s) = std::fs::read_to_string(parent.join("serveur.txt")) {
                let addr = s.trim();
                if !addr.is_empty() {
                    // SÛR : appelé au tout début de l'agent, mono-thread, avant tout spawn réseau.
                    unsafe { std::env::set_var("RENDEZVOUS_ADDR", addr) };
                }
            }
        }
    }
}

/// Brique 4 — DÉMARRAGE AUTO au boot. `agent install` copie l'agent (+ son `serveur.txt`) dans un
/// dossier STABLE et l'enregistre pour qu'il se lance seul à l'ouverture de session (Windows : tâche
/// planifiée `schtasks` ; Linux : service `systemd --user`, Restart=always). `agent uninstall` retire.
/// Dep-free : on appelle les outils système. Après l'install, l'agent est démarré tout de suite.
fn run_agent_install(uninstall: bool) {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => {
            eprintln!("[install] exe introuvable.");
            return;
        }
    };
    let serveur_txt = exe.parent().map(|p| p.join("serveur.txt"));

    #[cfg(windows)]
    {
        let tn = "web3-agent";
        let base = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
        let dir = std::path::Path::new(&base).join("web3-agent");
        let vbs = dir.join("start.vbs");
        // Auto-démarrage SANS ADMIN : le dossier « Démarrage » de l'utilisateur. Contrairement à
        // `schtasks /sc onlogon` (qui crée une tâche racine → exige l'élévation → « Accès refusé »
        // chez un ami sans droits admin, 28 juin), déposer un shim ici est TOUJOURS autorisé.
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".into());
        let startup_vbs = std::path::Path::new(&appdata)
            .join("Microsoft\\Windows\\Start Menu\\Programs\\Startup\\web3-agent.vbs");
        if uninstall {
            let _ = std::process::Command::new("schtasks").args(["/delete", "/tn", tn, "/f"]).status();
            let _ = std::fs::remove_file(&vbs);
            let _ = std::fs::remove_file(&startup_vbs);
            println!("[install] auto-démarrage RETIRÉ (tâche {tn} + dossier Démarrage).");
            return;
        }
        let _ = std::fs::create_dir_all(&dir);
        let dest = dir.join("jeu.exe");
        let _ = std::fs::copy(&exe, &dest);
        if let Some(s) = serveur_txt {
            if s.exists() {
                let _ = std::fs::copy(&s, dir.join("serveur.txt"));
            }
        }
        // CALME : un shim VBScript qui lance l'agent FENÊTRE CACHÉE (style 0) → plus de gros terminal.
        // Dep-free (juste un fichier texte). `0` = caché, `False` = ne pas attendre.
        let vbs_body = format!(
            "Set s = CreateObject(\"WScript.Shell\")\r\ns.Run \"\"\"{}\"\" agent loop\", 0, False\r\n",
            dest.to_string_lossy()
        );
        let _ = std::fs::write(&vbs, &vbs_body);
        // 1) Auto-start ROBUSTE = dossier Démarrage (aucun admin requis).
        let startup_ok = std::fs::write(&startup_vbs, &vbs_body).is_ok();
        // 2) BONUS best-effort = tâche planifiée (si on a les droits). Son échec n'est PLUS bloquant.
        let tr = format!("wscript.exe \"{}\"", vbs.to_string_lossy());
        let _ = std::process::Command::new("schtasks")
            .args(["/create", "/tn", tn, "/tr", &tr, "/sc", "onlogon", "/f"])
            .status();
        if startup_ok {
            println!("[install] ✅ DÉMARRAGE AUTO installé (dossier Démarrage, sans admin requis).");
        } else {
            eprintln!("[install] ⚠ auto-démarrage non posé, mais l'agent va tourner pour cette session.");
        }
        println!("[install] dossier : {}", dir.to_string_lossy());
        // 3) TOUJOURS démarrer l'agent maintenant (même si l'auto-start a échoué) → il se connecte tout de suite.
        let _ = std::process::Command::new("wscript.exe").arg(&vbs).spawn();
        println!("[install] agent démarré (en tâche de fond, sans fenêtre).");
    }

    #[cfg(unix)]
    {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let svc_dir = std::path::Path::new(&home).join(".config/systemd/user");
        let svc = svc_dir.join("web3-agent.service");
        if uninstall {
            let _ = std::process::Command::new("systemctl").args(["--user", "disable", "--now", "web3-agent"]).status();
            let _ = std::fs::remove_file(&svc);
            println!("[install] auto-démarrage RETIRÉ (service web3-agent).");
            return;
        }
        let data = std::path::Path::new(&home).join(".local/share/web3-agent");
        let _ = std::fs::create_dir_all(&data);
        let dest = data.join("jeu");
        let _ = std::fs::copy(&exe, &dest);
        if let Some(s) = serveur_txt {
            if s.exists() {
                let _ = std::fs::copy(&s, data.join("serveur.txt"));
            }
        }
        let _ = std::fs::create_dir_all(&svc_dir);
        let unit = format!(
            "[Unit]\nDescription=web3 agent de mesure\n\n[Service]\nExecStart={} agent loop\nWorkingDirectory={}\nRestart=always\nNice=19\nCPUSchedulingPolicy=idle\n\n[Install]\nWantedBy=default.target\n",
            dest.to_string_lossy(),
            data.to_string_lossy()
        );
        if std::fs::write(&svc, unit).is_err() {
            eprintln!("[install] impossible d'écrire le service systemd.");
            return;
        }
        let _ = std::process::Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        let st = std::process::Command::new("systemctl").args(["--user", "enable", "--now", "web3-agent"]).status();
        match st {
            Ok(s) if s.success() => {
                println!("[install] ✅ DÉMARRAGE AUTO installé (service systemd --user web3-agent, Restart=always).");
            }
            _ => eprintln!("[install] service écrit mais `systemctl --user enable` a échoué (essaie `loginctl enable-linger`)."),
        }
    }
}

/// La version courante de l'agent, lue dans `version.local` à côté de l'exe (« 0 » si absent → au
/// 1er lancement l'agent adopte la version du serveur). Découple la version du build → testable.
fn agent_version() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(|d| d.join("version.local")))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "0".to_string())
}

/// Un binaire exécutable PLAUSIBLE ? (magie ELF `\x7fELF` ou PE `MZ`, + taille mini) — garde-fou
/// anti-binaire tronqué/corrompu AVANT de l'installer. Jamais d'échange sur un téléchargement douteux.
fn looks_like_exe(b: &[u8]) -> bool {
    b.len() > 50_000 && (b.starts_with(&[0x7f, b'E', b'L', b'F']) || b.starts_with(b"MZ"))
}

/// AUTO-UPDATE (brique 5) : si le serveur annonce une version != la mienne, je télécharge le nouveau
/// binaire de MA plateforme, je le VÉRIFIE, je l'échange ATOMIQUEMENT, j'écris ma nouvelle version et
/// je me RELANCE (mêmes arguments). Renvoie true si une relance est lancée → l'appelant DOIT sortir.
/// 100 % dep-free. SÛR par construction : tout échec/incohérence → on garde l'ancien binaire qui tourne.
fn maybe_self_update(host: &str, port: u16) -> bool {
    let server_ver = match http_get(host, port, "/version") {
        Some(v) => v.trim().to_string(),
        None => return false, // serveur muet → on ne touche à RIEN
    };
    let my_ver = agent_version();
    if server_ver.is_empty() || server_ver == my_ver {
        return false; // déjà à jour
    }
    let plat = if cfg!(windows) { "jeu-windows" } else { "jeu-linux" };
    let bytes = match http_get_bytes(host, port, &format!("/{plat}")) {
        Some(b) => b,
        None => return false,
    };
    if !looks_like_exe(&bytes) {
        eprintln!("[agent] MAJ {server_ver} REFUSÉE : binaire douteux ({} o) → on garde l'actuel.", bytes.len());
        return false;
    }
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return false,
    };
    let dir = match exe.parent() {
        Some(d) => d.to_path_buf(),
        None => return false,
    };
    let tmp = dir.join("jeu.new");
    if std::fs::write(&tmp, &bytes).is_err() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755));
    }
    // Windows ne peut pas écraser un exe EN COURS → on renomme d'abord l'ancien (autorisé).
    #[cfg(windows)]
    {
        let _ = std::fs::rename(&exe, dir.join("jeu.old"));
    }
    if std::fs::rename(&tmp, &exe).is_err() {
        eprintln!("[agent] MAJ : échange impossible → on garde l'ancien binaire.");
        return false;
    }
    let _ = std::fs::write(dir.join("version.local"), &server_ver);
    println!("[agent] ⬆️  AUTO-UPDATE {my_ver} → {server_ver} : binaire échangé, relance…");
    let args: Vec<String> = std::env::args().skip(1).collect();
    let _ = std::process::Command::new(&exe).args(&args).spawn();
    true
}

/// L'horodatage époque (secondes), 0 si l'horloge est cassée.
fn epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ===================== FENÊTRE DE SESSION (transparence) =====================
// Modèle décidé avec l'utilisateur : une session NE BLOQUE PAS sur un consentement (les potes ne sont
// quasi jamais devant l'écran). Elle DÉMARRE TOUTE SEULE (ils ont accepté en installant) mais s'affiche
// dans une fenêtre VISIBLE qui montre les logs EN DIRECT, avec deux boutons : « Quitter la session »
// (déconnexion propre ~8 s, on prévient tout le monde, le PC se range au repos → re-dispo à la session
// SUIVANTE) et « Réduire » (continue en fond, pour quand le pote veut son PC malgré le ventilo). Si le
// pote ne fait rien, ça continue et se range tout seul à la fin. Coordination agent↔fenêtre par 3
// fichiers dans TEMP (dep-free) :
//   - web3_session.log    : l'agent écrit, la fenêtre affiche (tail).
//   - web3_session.active : présent pendant la session ; la fenêtre se ferme quand il disparaît.
//   - web3_quit.flag      : la fenêtre l'écrit au clic « Quitter » ; l'agent le lit → déconnexion propre.

fn session_log_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_session.log") }
fn session_active_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_session.active") }
fn session_quit_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_quit.flag") }

/// Ajoute une ligne au journal de session (que la fenêtre affiche). BORNÉ aux ~120 dernières lignes →
/// le fichier ne gonfle pas sur une longue session.
fn session_log_write(line: &str) {
    let p = session_log_file();
    let mut lines: Vec<String> =
        std::fs::read_to_string(&p).map(|s| s.lines().map(str::to_string).collect()).unwrap_or_default();
    lines.push(line.to_string());
    let n = lines.len();
    if n > 120 {
        lines.drain(0..n - 120);
    }
    let _ = std::fs::write(&p, lines.join("\n") + "\n");
}

/// Une ligne de journal AMICALE par fenêtre de mesure : combien de pairs, la pire fraîcheur (p95) et la
/// pire perte vues — pour que le pote comprenne en un coup d'œil « ce qui se passe ».
fn session_summary_line(
    n: u64,
    samples: &std::collections::HashMap<PeerId, Vec<f64>>,
    links: &std::collections::HashMap<PeerId, LinkStats>,
) -> String {
    let k = samples.len();
    let worst_p95 = samples
        .values()
        .map(|v| {
            let mut a = v.clone();
            a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
            percentile(&a, 95.0)
        })
        .fold(0.0_f64, f64::max);
    let worst_loss = links.values().map(|s| s.loss_pct).fold(0.0_f64, f64::max) * 100.0;
    let etat = if k == 0 {
        "personne en face pour l'instant"
    } else if worst_p95 <= 500.0 {
        "le reseau repond bien"
    } else {
        "le reseau est un peu lent"
    };
    format!("Mesure {n} : {k} machine(s) en vue - delai max {worst_p95:.0} ms, perte max {worst_loss:.0}% ({etat}).")
}

/// Ouvre la fenêtre de session : nettoie un éventuel flag « quitter » périmé, (re)crée le journal avec
/// un en-tête, pose le marqueur ACTIF, puis (Windows) lance la fenêtre WinForms en parallèle de la mesure.
fn session_window_open(session: u64) {
    let _ = std::fs::remove_file(session_quit_file());
    let _ = std::fs::write(
        session_log_file(),
        format!("=== web3 : mesure du reseau (projet de jeu video) - session #{session} ===\nL'outil demarre. Merci de ton aide !\n"),
    );
    let _ = std::fs::write(session_active_file(), "1");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000; // pas de console CMD derrière la fenêtre WinForms
        let ps = WINDOWS_SESSION_PS1
            .replace("__LOG__", &session_log_file().to_string_lossy())
            .replace("__ACTIVE__", &session_active_file().to_string_lossy())
            .replace("__QUIT__", &session_quit_file().to_string_lossy());
        let path = std::env::temp_dir().join("web3_session.ps1");
        // BOM UTF-8 → PowerShell 5.1 lit correctement les accents ; CREATE_NO_WINDOW + -WindowStyle
        // Hidden → AUCUNE console CMD ne s'ouvre, seule la fenêtre WinForms reste visible. spawn (pas
        // .output()) : la fenêtre vit À CÔTÉ de la mesure, sans la bloquer.
        if std::fs::write(&path, format!("\u{feff}{ps}")).is_ok() {
            let _ = std::process::Command::new("powershell")
                .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-WindowStyle", "Hidden", "-STA", "-File"])
                .arg(&path)
                .creation_flags(CREATE_NO_WINDOW)
                .spawn();
        }
    }
    #[cfg(unix)]
    {
        // Sans session graphique (serveur, service headless) → pas de fenêtre, la mesure tourne quand
        // même (le pote la verra au prochain affichage / via la présence). On ne lance la fenêtre QUE
        // s'il y a un affichage.
        let has_display =
            std::env::var("DISPLAY").is_ok() || std::env::var("WAYLAND_DISPLAY").is_ok();
        if has_display {
            let sh = LINUX_SESSION_SH
                .replace("__LOG__", &session_log_file().to_string_lossy())
                .replace("__ACTIVE__", &session_active_file().to_string_lossy())
                .replace("__QUIT__", &session_quit_file().to_string_lossy());
            let path = std::env::temp_dir().join("web3_session.sh");
            if std::fs::write(&path, &sh).is_ok() {
                // Pas de zenity garanti sur NixOS → on affiche les logs dans un TERMINAL (le plus
                // dispo l'emporte). Chaque terminal a sa façon de lancer une commande : on essaie
                // dans l'ordre, le premier qui se lance gagne.
                let terms: [(&str, &[&str]); 6] = [
                    ("kitty", &["bash"]),
                    ("foot", &["bash"]),
                    ("konsole", &["-e", "bash"]),
                    ("gnome-terminal", &["--", "bash"]),
                    ("alacritty", &["-e", "bash"]),
                    ("xterm", &["-e", "bash"]),
                ];
                for (term, pre) in terms {
                    if std::process::Command::new(term).args(pre).arg(&path).spawn().is_ok() {
                        break;
                    }
                }
            }
        }
    }
}

/// La FENÊTRE DE SESSION Linux = un terminal qui affiche le journal en direct (les chemins sont
/// substitués dans le script). Transparence (logs) + « tape q pour quitter et libérer ton PC »
/// (écrit le flag) ; fermer la fenêtre = cacher (la session continue). Se ferme quand le marqueur
/// ACTIF disparaît. POSIX-friendly mais utilise `read -t` (bash) → on le lance avec `bash`.
#[cfg(unix)]
const LINUX_SESSION_SH: &str = r#"LOG='__LOG__'
ACTIVE='__ACTIVE__'
QUIT='__QUIT__'
while [ -f "$ACTIVE" ]; do
  clear
  echo "=================================================================="
  echo "  web3 - mesure du reseau (projet de jeu video)"
  echo "  Merci de ton aide ! Cet outil mesure la qualite du reseau."
  echo "  Il tourne tout seul, tu n'as rien a faire."
  echo ""
  echo "  Besoin de ton ordinateur ? Tape  q  puis Entree pour quitter et"
  echo "  liberer CETTE machine (ca n'arrete que ton PC ; les mesures"
  echo "  continuent ailleurs). Aucun souci pour partir."
  echo "=================================================================="
  tail -n 15 "$LOG" 2>/dev/null
  echo "------------------------------------------------------------------"
  echo "[q]+Entree = quitter et liberer mon PC   |   fermer la fenetre = cacher"
  if read -t 1 -r key; then
    case "$key" in
      q|Q) printf 'quit' > "$QUIT"; echo "Deconnexion en cours... (~8 s)"; sleep 3; exit 0;;
    esac
  fi
done
echo "Mesures terminees. Merci de ton aide !"
sleep 1
"#;

/// Le pote a-t-il cliqué « Quitter la session » ? (présence du flag écrit par la fenêtre.)
fn session_quit_requested() -> bool {
    session_quit_file().exists()
}

/// Ferme la fenêtre de session : retire le marqueur ACTIF (la fenêtre se ferme d'elle-même au prochain
/// tick) et le flag « quitter ».
fn session_window_close() {
    let _ = std::fs::remove_file(session_active_file());
    let _ = std::fs::remove_file(session_quit_file());
}

/// Le script PowerShell de la FENÊTRE DE SESSION Windows. `__LOG__`/`__ACTIVE__`/`__QUIT__` → chemins.
/// Journal en direct (TextBox noir/vert, 1 Hz) + textes français clairs (rôle de l'outil, ce que fait
/// « Quitter »). « Quitter » écrit le flag ; « Mettre de côté » RANGE la fenêtre dans une icône de la
/// zone de notification (NotifyIcon) — léger, non invasif, on la rouvre en double-cliquant l'icône. Se
/// ferme quand le marqueur ACTIF disparaît. (Écrit avec BOM UTF-8 → accents OK ; lancée sans console.)
#[cfg(windows)]
const WINDOWS_SESSION_PS1: &str = r#"$log = '__LOG__'
$active = '__ACTIVE__'
$quit = '__QUIT__'
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form = New-Object System.Windows.Forms.Form
$form.Text = 'web3 - mesure du reseau'
$form.Width = 600
$form.Height = 470
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$hdr = New-Object System.Windows.Forms.Label
$hdr.Text = "Bonjour, et merci de ton aide. Cet outil mesure la qualite du reseau pour un projet de jeu video. Il travaille en ce moment : tu n'as rien a faire."
$hdr.SetBounds(14, 12, 565, 44)
$info = New-Object System.Windows.Forms.Label
$info.Text = "Si tu as besoin de ton ordinateur ou si l'outil te derange, tu peux l'arreter avec << Quitter >>. Cela n'arrete que TON ordinateur ; les mesures continuent ailleurs. Je suis en train de mesurer, mais aucun souci si tu dois t'en aller."
$info.SetBounds(14, 58, 565, 56)
$box = New-Object System.Windows.Forms.TextBox
$box.Multiline = $true
$box.ReadOnly = $true
$box.ScrollBars = 'Vertical'
$box.BackColor = 'Black'
$box.ForeColor = 'Lime'
$box.Font = New-Object System.Drawing.Font('Consolas', 9)
$box.SetBounds(14, 120, 565, 248)
$btnQuit = New-Object System.Windows.Forms.Button
$btnQuit.Text = 'Quitter (liberer mon ordinateur)'
$btnQuit.SetBounds(14, 378, 285, 46)
$btnHide = New-Object System.Windows.Forms.Button
$btnHide.Text = 'Mettre de cote (petite icone en bas a droite)'
$btnHide.SetBounds(308, 378, 271, 46)
$foot = New-Object System.Windows.Forms.Label
$foot.Text = "Si tu ne fais rien, l'outil continue tranquillement et se ferme tout seul a la fin."
$foot.SetBounds(14, 430, 565, 20)
$form.Controls.AddRange(@($hdr, $info, $box, $btnQuit, $btnHide, $foot))
$notify = New-Object System.Windows.Forms.NotifyIcon
$notify.Icon = [System.Drawing.SystemIcons]::Information
$notify.Text = 'web3 - mesure du reseau en cours'
$notify.Visible = $true
$menu = New-Object System.Windows.Forms.ContextMenuStrip
$miShow = $menu.Items.Add('Afficher la fenetre')
$miQuit = $menu.Items.Add('Quitter (liberer mon ordinateur)')
$notify.ContextMenuStrip = $menu
$doQuit = {
    try { Set-Content -Path $quit -Value 'quit' -ErrorAction SilentlyContinue } catch {}
    $btnQuit.Enabled = $false
    $btnQuit.Text = 'Deconnexion en cours... (~8 s)'
    $form.Show(); $form.WindowState = 'Normal'
}
$doShow = { $form.Show(); $form.WindowState = 'Normal'; $form.Activate(); $form.BringToFront() }
$btnQuit.add_Click($doQuit)
$miQuit.add_Click($doQuit)
$btnHide.add_Click({ $form.Hide() })
$miShow.add_Click($doShow)
$notify.add_DoubleClick($doShow)
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 1000
$timer.add_Tick({
    if (Test-Path $log) {
        try {
            $t = (Get-Content -Path $log -Tail 200 -ErrorAction SilentlyContinue) -join "`r`n"
            if ($box.Text -ne $t) { $box.Text = $t; $box.SelectionStart = $box.Text.Length; $box.ScrollToCaret() }
        } catch {}
    }
    if (-not (Test-Path $active)) { $timer.Stop(); $notify.Visible = $false; $notify.Dispose(); $form.Close() }
})
$timer.Start()
$form.add_FormClosed({ $notify.Visible = $false; $notify.Dispose() })
$form.add_Shown({ $form.Activate(); $form.BringToFront() })
[void]$form.ShowDialog()
"#;

/// Issue d'une session de mesure : terminée normalement, ou interrompue par une AUTO-UPDATE (l'appelant
/// doit alors sortir, le nouveau process prend le relais).
enum SessionEnd {
    Normal,
    Updated,
}

/// UNE SESSION DE MESURE, VISIBLE : on ouvre la fenêtre, on crée le nœud, on mesure fenêtre par fenêtre
/// (fraîcheur + perte/gigue/ré-ordre), on uploade ET on écrit un résumé amical dans le journal visible.
/// À chaque tour on surveille le bouton « Quitter » → déconnexion propre (~8 s, on prévient tout le monde).
/// On RE-LIT la campagne à chaque fenêtre : si le serveur repasse `idle`/change de `session`, on range la
/// fenêtre et on revient au repos. Le nœud P2P n'existe QUE pendant la session.
fn run_measure_session(cfg_host: &str, start: Campaign) -> SessionEnd {
    let mut bot = match Bot::new("agent", false, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[agent] réseau indisponible (le rendez-vous est-il joignable ?) — session annulée.");
            return SessionEnd::Normal;
        }
    };
    if start.aoi {
        bot.enable_aoi_bilateral(); // couche 2 ON le temps de cette session (flip serveur)
    }
    println!(
        "[agent] session {} — mesure VISIBLE en cours (fenêtre {}s, {} bot(s){})…",
        start.session, start.window, start.bots,
        if start.aoi { ", AoI bilatérale ON" } else { "" }
    );
    session_window_open(start.session);
    send_heartbeat(cfg_host, CONFIG_PORT, "session");

    let stop_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut worker_handles = Vec::new();
    if start.bots > 1 {
        println!("[agent] 🚀 Démarrage de {} bots de foule en arrière-plan...", start.bots - 1);
        for i in 1..start.bots {
            let stop = std::sync::Arc::clone(&stop_flag);
            worker_handles.push(std::thread::spawn(move || {
                let phase = i as f32 * 0.37;
                if let Some(mut b) = Bot::new(format!("b_{i}"), false, phase) {
                    if start.aoi {
                        b.enable_aoi_bilateral(); // les bots de foule aussi → vraie réception bornée
                    }
                    let boot = Instant::now();
                    let mut last = Instant::now();
                    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
                        let dt = last.elapsed().as_secs_f32();
                        last = Instant::now();
                        b.step(dt, boot.elapsed().as_secs_f32());
                        std::thread::sleep(std::time::Duration::from_millis(50));
                    }
                }
            }));
        }
    }

    let boot = Instant::now();
    let mut last = Instant::now();
    let mut win_start = Instant::now();
    let mut samples: std::collections::HashMap<PeerId, Vec<f64>> = std::collections::HashMap::new();
    let mut first = true;
    let mut window = start.window;
    let mut measure_n = 0u64;
    loop {
        // Le pote a-t-il cliqué « Quitter la session » ? → DÉCONNEXION PROPRE : on prévient tout le monde
        // (heartbeat « leaving »), on laisse ~8 s pour que le départ soit vu, puis on range et on idle.
        // La session reste « faite » côté boucle → le PC redevient dispo à la session SUIVANTE.
        if session_quit_requested() {
            println!("[agent] « Quitter » demandé par le pote — déconnexion propre (~8 s)…");
            session_log_write("Tu as demande a liberer ton ordinateur. Deconnexion en cours... (~8 s)");
            send_heartbeat(cfg_host, CONFIG_PORT, "leaving");
            stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
            std::thread::sleep(std::time::Duration::from_secs(8)); // laisse les autres voir le départ
            session_log_write("C'est bon, ton ordinateur est libere. Merci ! (ca reprendra a la prochaine session, sans rien faire)");
            std::thread::sleep(std::time::Duration::from_millis(800));
            session_window_close();
            send_heartbeat(cfg_host, CONFIG_PORT, "idle");
            return SessionEnd::Normal;
        }
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        bot.step(dt, boot.elapsed().as_secs_f32()); // horloge CONTINUE entre fenêtres (pas de saut)
        if !first || win_start.elapsed().as_secs_f64() >= 3.0 {
            for (id, age) in bot.peer_freshness_ms() {
                samples.entry(id).or_default().push(age);
            }
        }
        // Vérification fréquente (chaque seconde) de l'arrêt de campagne pour FERMETURE INSTANTANÉE de la fenêtre
        if last.elapsed().as_millis() % 1000 < 10 {
            if let Some(c) = http_get(cfg_host, CONFIG_PORT, "/campaign").map(|b| parse_campaign(&b)) {
                if c.mode != Mode::Simulate || c.session != start.session {
                    println!("[agent] session {} arrêtée à chaud — fermeture immédiate.", start.session);
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    session_window_close();
                    send_heartbeat(cfg_host, CONFIG_PORT, "idle");
                    return SessionEnd::Normal;
                }
            }
        }
        if win_start.elapsed().as_secs() >= window {
            measure_n += 1;
            let links = link_stats_by_peer(bot.take_link_arrivals(), 16.0);
            let lines = report_freshness(&epoch_secs().to_string(), &samples, &links);
            for l in &lines {
                let _ = http_post(cfg_host, CONFIG_PORT, "/upload", l); // brique 3
            }
            session_log_write(&session_summary_line(measure_n, &samples, &links)); // journal VISIBLE
            // OBSERVABILITÉ (28 juin) : on RACONTE au serveur ce qu'on a vu/reçu/envoyé cette fenêtre.
            // `recv=0` à distance = on ne reçoit RIEN (relais-retour mort) ; `peers=0` = on ne découvre
            // personne ; `sent` = mesures uploadées. On voit le vrai état de chaque ami SANS capture d'écran.
            let recv_total: usize = links.values().map(|s| s.received).sum();
            let diag = format!(",\"peers\":{},\"recv\":{},\"sent\":{}", samples.len(), recv_total, lines.len());
            samples.clear();
            send_heartbeat_diag(cfg_host, CONFIG_PORT, "alive", &diag); // présence + diagnostic pendant la session
            if let Some(c) = http_get(cfg_host, CONFIG_PORT, "/campaign").map(|b| parse_campaign(&b)) {
                if c.mode != Mode::Simulate || c.session != start.session {
                    println!("[agent] session {} terminée — retour au repos.", start.session);
                    session_log_write("Mesures terminees. La fenetre se ferme toute seule. Merci de ton aide !");
                    stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    session_window_close();
                    send_heartbeat(cfg_host, CONFIG_PORT, "idle");
                    return SessionEnd::Normal;
                }
                window = c.window; // la fenêtre peut être ajustée à chaud
            }
            if maybe_self_update(cfg_host, CONFIG_PORT) {
                stop_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                session_window_close();
                return SessionEnd::Updated; // le nouveau process reprend
            }
            win_start = Instant::now();
            first = false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// LA BOUCLE DE L'AGENT (calme + TRANSPARENTE). Au REPOS : aucun nœud P2P, juste un battement de cœur
/// léger (présence « qui est en ligne quand ») + relecture de campagne + auto-update. Quand le serveur
/// demande une session (`mode=simulate` + un `session` neuf), on DÉMARRE DIRECTEMENT la mesure dans une
/// fenêtre VISIBLE (pas de popup bloquant : les potes ne sont jamais à l'écran) — le pote peut Quitter ou
/// Réduire quand il veut. Une session par `session` (set `decided`) → après un « Quitter », le PC reste
/// au repos et redevient dispo dès que je bumpe `session`.
/// Usage CPU GLOBAL de la machine (0..100 %), mesuré sur un court échantillon (~120 ms). `None` si la
/// plateforme n'est pas gérée → on ne bride alors RIEN (comportement inchangé). 100 % dep-free.
/// Sert au RESPECT DE L'HÔTE : si le pote est occupé (jeu, simu lourde…), l'agent s'efface.
#[cfg(target_os = "linux")]
fn cpu_busy_pct() -> Option<f64> {
    // /proc/stat ligne « cpu  user nice system idle iowait irq softirq steal … » (jiffies cumulés).
    fn snap() -> Option<(u64, u64)> {
        let s = std::fs::read_to_string("/proc/stat").ok()?;
        let line = s.lines().next()?;
        let v: Vec<u64> = line.split_whitespace().skip(1).filter_map(|x| x.parse().ok()).collect();
        if v.len() < 4 {
            return None;
        }
        let idle = v[3] + v.get(4).copied().unwrap_or(0); // idle + iowait
        let total: u64 = v.iter().sum();
        Some((idle, total))
    }
    let (i0, t0) = snap()?;
    std::thread::sleep(std::time::Duration::from_millis(120));
    let (i1, t1) = snap()?;
    let dt = t1.checked_sub(t0)?;
    if dt == 0 {
        return None;
    }
    let di = i1.saturating_sub(i0);
    Some(((1.0 - di as f64 / dt as f64) * 100.0).clamp(0.0, 100.0))
}

#[cfg(target_os = "windows")]
fn cpu_busy_pct() -> Option<f64> {
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    impl FileTime {
        fn as_u64(self) -> u64 {
            ((self.high as u64) << 32) | self.low as u64
        }
    }
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetSystemTimes(idle: *mut FileTime, kernel: *mut FileTime, user: *mut FileTime) -> i32;
    }
    // kernel INCLUT l'idle → busy = (kernel + user) − idle ; total = kernel + user.
    fn snap() -> Option<(u64, u64)> {
        let (mut i, mut k, mut u) = (FileTime::default(), FileTime::default(), FileTime::default());
        if unsafe { GetSystemTimes(&mut i, &mut k, &mut u) } == 0 {
            return None;
        }
        Some((i.as_u64(), k.as_u64() + u.as_u64()))
    }
    let (i0, t0) = snap()?;
    std::thread::sleep(std::time::Duration::from_millis(120));
    let (i1, t1) = snap()?;
    let dt = t1.checked_sub(t0)?;
    if dt == 0 {
        return None;
    }
    let di = i1.saturating_sub(i0);
    Some(((1.0 - di as f64 / dt as f64) * 100.0).clamp(0.0, 100.0))
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
fn cpu_busy_pct() -> Option<f64> {
    None
}

/// Seuil de RESPECT : au-dessus, la machine du pote est jugée OCCUPÉE → l'agent NE mesure PAS (il
/// reste en simple battement de cœur et réessaiera au calme). « Si 90 % d'usage, on ne lance pas. »
const HOST_BUSY_PCT: f64 = 85.0;

/// Met le process à la PRIORITÉ LA PLUS BASSE (« comme un démon Linux `nice` ») → l'OS ne lui donne
/// que les MIETTES de CPU : il ne dispute jamais un cycle au jeu ou à la simu du pote. Dep-free.
/// (Linux : posé en plus par le service systemd — `Nice=19` + `CPUSchedulingPolicy=idle`.)
#[cfg(target_os = "windows")]
fn lower_own_priority() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> isize;
        fn SetPriorityClass(h: isize, class: u32) -> i32;
    }
    const IDLE_PRIORITY_CLASS: u32 = 0x0000_0040;
    unsafe {
        let _ = SetPriorityClass(GetCurrentProcess(), IDLE_PRIORITY_CLASS);
    }
}
#[cfg(not(target_os = "windows"))]
fn lower_own_priority() {
    // Linux : la priorité basse vient du service systemd (Nice=19 + CPUSchedulingPolicy=idle).
}

fn run_agent_loop(window: u64) {
    lower_own_priority(); // RESPECT : on se met en retrait du CPU dès le départ (« comme Linux »).
    let cfg_host = super::link::rendezvous_addr().ip().to_string();
    let fallback_window = window.max(5);
    println!(
        "[agent] démarré — au REPOS (battement de cœur). En attente d'une session sur \
         http://{cfg_host}:{CONFIG_PORT}/campaign (Ctrl-C pour arrêter)"
    );
    send_heartbeat(&cfg_host, CONFIG_PORT, "start");
    let mut done: std::collections::HashSet<u64> = std::collections::HashSet::new();
    loop {
        let campaign = http_get(&cfg_host, CONFIG_PORT, "/campaign")
            .map(|b| parse_campaign(&b))
            .unwrap_or(Campaign { window: fallback_window, ..Campaign::default() });
        // AUTO-UPDATE : au repos, c'est le moment SÛR pour s'échanger (aucune session en cours).
        if maybe_self_update(&cfg_host, CONFIG_PORT) {
            return; // le nouveau process prend le relais
        }
        if campaign.mode == Mode::Simulate && !done.contains(&campaign.session) {
            // RESPECT DE L'HÔTE : si sa machine est OCCUPÉE (jeu, simu lourde…), on NE lance PAS la
            // mesure — on s'efface et on réessaiera au calme. On ne marque PAS la session « faite »
            // (donc elle se relancera dès que ça se calme). cf. cpu_busy_pct / HOST_BUSY_PCT.
            if let Some(p) = cpu_busy_pct().filter(|&p| p >= HOST_BUSY_PCT) {
                println!("[agent] machine occupée ({p:.0}% CPU ≥ {HOST_BUSY_PCT:.0}%) — je m'efface, je réessaierai au calme.");
                send_heartbeat_diag(&cfg_host, CONFIG_PORT, "busy", &format!(",\"cpu_pct\":{p:.0}"));
            } else {
                done.insert(campaign.session); // une session par identifiant
                println!("[agent] session {} demandée — démarrage VISIBLE (fenêtre + mesure).", campaign.session);
                if let SessionEnd::Updated = run_measure_session(&cfg_host, campaign) {
                    return;
                }
            }
        } else {
            // REPOS : juste la présence (coût réseau négligeable, le pote n'est pas dérangé).
            send_heartbeat(&cfg_host, CONFIG_PORT, "alive");
        }
        std::thread::sleep(std::time::Duration::from_secs(campaign.window.clamp(5, 3600)));
    }
}

/// SERT un DOSSIER en HTTP (côté serveur de la flotte) — dep-free (std uniquement). GET /nom →
/// fichier `dir/nom`, RELU à chaque requête (j'édite, les agents suivent). Sert la `campaign`, la
/// `version`, et les binaires `jeu-linux`/`jeu-windows` (auto-update). Binaire-safe (octets bruts).
/// SÉCURITÉ : un seul niveau, pas de `..` ni de `/` → aucune remontée de dossier.
pub fn run_serve_config(dir: &str, port: u16) {
    use std::io::{Read, Write};
    let listener = match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("serve : impossible d'écouter sur {port} : {e}");
            return;
        }
    };
    println!("campagne + MAJ servies sur 0.0.0.0:{port} depuis « {dir} » (Ctrl-C pour arrêter)");
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let mut scratch = [0u8; 4096]; // plus grand : peut contenir le corps d'un POST d'upload
        let n = stream.read(&mut scratch).unwrap_or(0);
        let req = String::from_utf8_lossy(&scratch[..n]);
        // POST → on AJOUTE le corps à un journal, choisi par le CHEMIN : `/heartbeat` (présence,
        // « qui est en ligne quand ») va dans `presence.ndjson` ; tout le reste (`/upload`, brique 3)
        // dans `uploads.ndjson`. Deux fichiers séparés → la présence ne noie pas les mesures.
        if req.split_whitespace().next() == Some("POST") {
            let path = req.split_whitespace().nth(1).unwrap_or("/");
            let file = if path == "/heartbeat" { "presence.ndjson" } else { "uploads.ndjson" };
            if let Some(p) = req.find("\r\n\r\n") {
                let payload = req[p + 4..].trim();
                if !payload.is_empty() {
                    use std::io::Write as _;
                    if let Ok(mut f) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(format!("{dir}/{file}"))
                    {
                        let _ = writeln!(f, "{payload}");
                    }
                }
            }
            let _ = stream.write_all(b"HTTP/1.0 200 OK\r\nConnection: close\r\n\r\n");
            continue;
        }
        // GET /nom → fichier `dir/nom`
        let name = req.split_whitespace().nth(1).unwrap_or("/").trim_start_matches('/');
        let safe = !name.is_empty() && !name.contains("..") && !name.contains('/') && !name.contains('\\');
        let body = if safe { std::fs::read(format!("{dir}/{name}")).ok() } else { None };
        match body {
            Some(bytes) => {
                let header = format!(
                    "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    bytes.len()
                );
                let _ = stream.write_all(header.as_bytes());
                let _ = stream.write_all(&bytes);
            }
            None => {
                let _ = stream.write_all(b"HTTP/1.0 404 Not Found\r\nConnection: close\r\n\r\n");
            }
        }
    }
}

#[derive(Clone, Debug)]
struct HeartbeatRecord {
    ts: u64,
    host: String,
    _ver: String,
    ev: String,
}

fn parse_heartbeat(line: &str) -> Option<HeartbeatRecord> {
    let line = line.trim();
    if !line.starts_with('{') || !line.ends_with('}') {
        return None;
    }
    let mut ts = 0u64;
    let mut host = String::new();
    let mut _ver = String::new();
    let mut ev = String::new();

    for part in line[1..line.len() - 1].split(',') {
        if let Some((k, v)) = part.split_once(':') {
            let k = k.trim().trim_matches('"');
            let v = v.trim().trim_matches('"');
            match k {
                "ts" => ts = v.parse().unwrap_or(0),
                "host" => host = v.to_string(),
                "ver" => _ver = v.to_string(),
                "ev" => ev = v.to_string(),
                _ => {}
            }
        }
    }
    if ts > 0 && !host.is_empty() {
        Some(HeartbeatRecord { ts, host, _ver, ev })
    } else {
        None
    }
}

/// AFFICHE LES STATISTIQUES DE PRÉSENCE (machines actives + dispo moyenne par heure).
pub fn run_stats() {
    ensure_rendezvous_from_file();
    let cfg_host = super::link::rendezvous_addr().ip().to_string();
    println!("[stats] Analyse de la présence réseau depuis http://{cfg_host}:{CONFIG_PORT}/presence.ndjson ...");

    let content = std::fs::read_to_string("presence.ndjson")
        .or_else(|_| std::fs::read_to_string("/home/shaza/web3-serve/presence.ndjson"))
        .ok()
        .or_else(|| http_get(&cfg_host, CONFIG_PORT, "/presence.ndjson"));

    let body = match content {
        Some(b) => b,
        None => {
            eprintln!("[stats] Impossible de récupérer presence.ndjson depuis le serveur.");
            return;
        }
    };

    let records: Vec<HeartbeatRecord> = body.lines().filter_map(parse_heartbeat).collect();
    if records.is_empty() {
        println!("[stats] Aucune donnée de présence enregistrée pour l'instant.");
        return;
    }

    let now = epoch_secs();

    let mut latest_by_host: std::collections::HashMap<String, &HeartbeatRecord> = std::collections::HashMap::new();
    for r in &records {
        let entry = latest_by_host.entry(r.host.clone()).or_insert(r);
        if r.ts > entry.ts {
            *entry = r;
        }
    }

    let active_threshold = 120;
    let mut active_hosts: Vec<(&String, u64, &str)> = Vec::new();
    for (host, rec) in &latest_by_host {
        if now >= rec.ts && (now - rec.ts) <= active_threshold && rec.ev != "leaving" {
            active_hosts.push((host, now - rec.ts, &rec.ev));
        }
    }
    active_hosts.sort_by_key(|h| h.1);

    println!("\n==================================================================");
    println!("  💻 STATUT ACTUEL DU RÉSEAU");
    println!("==================================================================");
    if active_hosts.is_empty() {
        println!("  Aucune machine active actuellement (dernier battement > 2 min).");
    } else {
        println!("  {} machine(s) connectée(s) et prête(s) pour une simulation :", active_hosts.len());
        for (host, age, ev) in &active_hosts {
            println!("   • {:<20} (actif il y a {:>3}s, mode: {})", host, age, ev);
        }
    }

    let mut hour_day_hosts: std::collections::HashMap<(u64, u32), std::collections::HashSet<String>> = std::collections::HashMap::new();
    let mut days_seen: std::collections::HashSet<u64> = std::collections::HashSet::new();

    for r in &records {
        let day = r.ts / 86400;
        let hour = ((r.ts % 86400) / 3600) as u32;
        days_seen.insert(day);
        hour_day_hosts.entry((day, hour)).or_default().insert(r.host.clone());
    }

    let n_days = days_seen.len().max(1) as f64;
    let mut hourly_avg = [0.0f64; 24];
    for hour in 0..24 {
        let mut total_hosts_for_hour = 0usize;
        for &day in &days_seen {
            if let Some(hosts) = hour_day_hosts.get(&(day, hour)) {
                total_hosts_for_hour += hosts.len();
            }
        }
        hourly_avg[hour as usize] = total_hosts_for_hour as f64 / n_days;
    }

    let max_avg = hourly_avg.iter().copied().fold(0.0f64, f64::max);

    println!("\n==================================================================");
    println!("  📊 HISTORIQUE ET DISPONIBILITÉ MOYENNE PAR HEURE (UTC)");
    println!("==================================================================");
    for hour in 0..24 {
        let avg = hourly_avg[hour];
        let bar_len = if max_avg > 0.0 { ((avg / max_avg) * 20.0).round() as usize } else { 0 };
        let bar = "█".repeat(bar_len);
        let peak_marker = if max_avg > 0.0 && (avg - max_avg).abs() < 1e-5 { " ⭐ PIC" } else { "" };
        println!("  {:02}h00 - {:02}h59 | {:<20} | {:.1} PC(s){}", hour, hour, bar, avg, peak_marker);
    }
    println!("==================================================================\n");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La perte se lit EXACTEMENT dans les trous de seq, et un seq qui recule = ré-ordre.
    #[test]
    fn perte_et_reordre_se_lisent_dans_les_seq() {
        // seq [0,1,2,4,5] : le 3 manque → 1 perdu sur 6 attendus (16,7 %). Aucun recul.
        let a: Vec<Arrival> = [0u64, 1, 2, 4, 5]
            .iter()
            .enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 50.0, seq })
            .collect();
        let s = link_stats(&a, 50.0);
        assert_eq!(s.received, 5);
        assert_eq!(s.expected, 6);
        assert!((s.loss_pct - 1.0 / 6.0).abs() < 1e-9, "perte = 1/6");
        assert!(s.reorder_pct.abs() < 1e-9, "aucun ré-ordre");

        // seq [0,1,3,2,4] : le 2 arrive APRÈS le 3 → 1 recul sur 4 transitions (25 %).
        let b: Vec<Arrival> = [0u64, 1, 3, 2, 4]
            .iter()
            .enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 50.0, seq })
            .collect();
        let s = link_stats(&b, 50.0);
        assert!((s.reorder_pct - 0.25).abs() < 1e-9, "1 ré-ordre sur 4");
        assert!(s.loss_pct.abs() < 1e-9, "aucune perte ici (0..4 complet)");
    }

    /// ENQUÊTE « inspecteur Eve » (28 juin) : un pair BRIDÉ par l'AoI (2 Hz sur un seq global à
    /// 20 Hz) affiche une `loss_pct` énorme (FAUX : rien n'est perdu, l'émetteur n'a pas envoyé
    /// exprès) mais une `real_loss_pct` ~nulle. Et une VRAIE perte par-dessus se lit, elle, dans
    /// `real_loss_pct`. C'est la correction qui sépare « pas envoyé » de « envoyé puis perdu ».
    #[test]
    fn vraie_perte_distinguee_du_bridage_aoi() {
        // BRIDÉ SANS PERTE : seq 1,11,21,31,41 (cadence 10 = 2 Hz sur 20 Hz). Plein débit aurait
        // « attendu » 41 paquets → loss_pct ~88 % (faux positif). Cadence régulière → real_loss = 0.
        let bride: Vec<Arrival> = [1u64, 11, 21, 31, 41]
            .iter().enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 500.0, seq })
            .collect();
        let s = link_stats(&bride, 50.0);
        assert_eq!(s.cadence_step, 10, "cadence inférée = 10 (le bridage 2 Hz)");
        assert!(s.loss_pct > 0.85, "perte APPARENTE énorme (vs plein débit) : {}", s.loss_pct);
        assert!(s.real_loss_pct.abs() < 1e-9, "AUCUNE vraie perte (rien n'a été perdu) : {}", s.real_loss_pct);

        // BRIDÉ + 1 VRAIE PERTE : seq 1,11,31,41 (le 21 manque). Cadence toujours 10 (médiane) ;
        // le saut de 20 = 2 créneaux → 1 manquant sur 4 créneaux = 25 % de VRAIE perte.
        let perdu: Vec<Arrival> = [1u64, 11, 31, 41]
            .iter().enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 500.0, seq })
            .collect();
        let s = link_stats(&perdu, 50.0);
        assert_eq!(s.cadence_step, 10, "cadence inférée toujours 10");
        assert!((s.real_loss_pct - 0.25).abs() < 1e-9, "1 vraie perte sur 4 créneaux = 25 % : {}", s.real_loss_pct);
    }

    /// La FRAÎCHEUR grandit quand les paquets s'espacent : un lien à 1 paquet/seconde donne
    /// un âge bien pire qu'un lien à 20 Hz — c'est la grandeur « est-ce vivant ».
    #[test]
    fn fraicheur_pire_quand_les_paquets_s_espacent() {
        let serre: Vec<Arrival> = (0..50).map(|i| Arrival { recv_ms: i as f64 * 50.0, seq: i }).collect();
        let lache: Vec<Arrival> = (0..50).map(|i| Arrival { recv_ms: i as f64 * 1000.0, seq: i }).collect();
        let f_serre = link_stats(&serre, 16.0).fresh_p95_ms;
        let f_lache = link_stats(&lache, 16.0).fresh_p95_ms;
        assert!(f_lache > f_serre * 5.0, "un lien lâche est bien moins frais qu'un lien serré");
        assert!(f_serre < 500.0, "20 Hz reste sous le seuil de vivacité (≤ 500 ms)");
    }

    /// Brique 2 — la campagne se parse, ignore l'inconnu, borne les valeurs folles, et NE CASSE
    /// JAMAIS sur une entrée foireuse (self-sufficient : on retombe sur le défaut).
    #[test]
    fn campagne_robuste_borne_et_ignore_l_inconnu() {
        assert_eq!(parse_campaign("window=45\nautre_cle=xyz\n").window, 45);
        assert_eq!(parse_campaign("").window, 30); // vide → défaut
        assert_eq!(parse_campaign("window=99999").window, 3600); // borné haut
        assert_eq!(parse_campaign("window=1").window, 5); // borné bas
        assert_eq!(parse_campaign("window=oops").window, 30); // illisible → défaut
    }

    /// Brique B — le mode/session se parse, et le DÉFAUT le plus SÛR est le REPOS (jamais de mesure
    /// surprise) : campagne vide ou mode inconnu → `Idle`. `simulate` (insensible à la casse) → mesure.
    #[test]
    fn campagne_mode_et_session_defaut_repos() {
        assert_eq!(parse_campaign("").mode, Mode::Idle); // défaut = repos
        assert_eq!(parse_campaign("mode=bidon").mode, Mode::Idle); // inconnu → repos (sûr)
        assert_eq!(parse_campaign("mode=Simulate").mode, Mode::Simulate); // casse ignorée
        assert_eq!(parse_campaign("mode=simulate\nsession=7\n").session, 7);
        assert_eq!(parse_campaign("").session, 0);
        // COUCHE 2 — le flip `aoi` : absent → OFF (byte-pour-byte), `1`/`true` → ON.
        assert!(!parse_campaign("").aoi);
        assert!(!parse_campaign("aoi=0").aoi);
        assert!(parse_campaign("aoi=1").aoi);
        assert!(parse_campaign("aoi=true").aoi);
        // une campagne complète et réaliste :
        let c = parse_campaign("window=20\nmode=simulate\nsession=42\nbots=500\naoi=1\n");
        assert_eq!((c.window, c.mode, c.session, c.bots, c.aoi), (20, Mode::Simulate, 42, 500, true));
    }

    /// Fenêtre de session — la coordination par fichiers (le contrat agent↔fenêtre) : `open` pose le
    /// marqueur ACTIF et nettoie un flag « quitter » périmé ; le journal est BORNÉ ; un clic « Quitter »
    /// se détecte ; `close` retire tout. (Le rendu WinForms, lui, se valide sur un vrai Windows.)
    #[test]
    fn fenetre_session_coordination_fichiers() {
        let _ = std::fs::write(session_quit_file(), "perime"); // flag d'une session précédente
        session_window_open(42);
        assert!(session_active_file().exists(), "le marqueur ACTIF est posé");
        assert!(!session_quit_requested(), "le flag « quitter » périmé est nettoyé à l'ouverture");

        for i in 0..200 {
            session_log_write(&format!("ligne {i}"));
        }
        let content = std::fs::read_to_string(session_log_file()).unwrap();
        assert!(content.lines().count() <= 121, "journal borné (~120 lignes), pas de gonflement");

        std::fs::write(session_quit_file(), "quit").unwrap(); // la fenêtre signale le clic « Quitter »
        assert!(session_quit_requested());

        session_window_close();
        assert!(!session_active_file().exists(), "ACTIF retiré → la fenêtre se ferme");
        assert!(!session_quit_requested(), "flag « quitter » retiré");
        let _ = std::fs::remove_file(session_log_file());
    }

    /// L'instrument COMPLET : à partir des arrivées par pair, on chiffre perte/ré-ordre par lien, et
    /// un pair à <2 arrivées (rien à chiffrer) est écarté → le rapport reste honnête (pas de 0 % faux).
    #[test]
    fn link_stats_by_peer_chiffre_et_ecarte_le_trop_court() {
        use super::super::crypto::PeerId;
        let bon = PeerId::from_bytes([1u8; 32]);
        let perte = PeerId::from_bytes([2u8; 32]);
        let muet = PeerId::from_bytes([3u8; 32]);
        let mut arr: std::collections::HashMap<PeerId, Vec<Arrival>> = std::collections::HashMap::new();
        // bon : 0..10 sans trou ni recul.
        arr.insert(bon, (0..10u64).map(|i| Arrival { recv_ms: i as f64 * 50.0, seq: i }).collect());
        // perte : le seq 3 manque (1 perdu sur 6 attendus).
        arr.insert(perte, [0u64, 1, 2, 4, 5].iter().enumerate()
            .map(|(i, &seq)| Arrival { recv_ms: i as f64 * 50.0, seq }).collect());
        // muet : une seule arrivée → rien à chiffrer, doit être écarté.
        arr.insert(muet, vec![Arrival { recv_ms: 0.0, seq: 0 }]);

        let out = link_stats_by_peer(arr, 16.0);
        assert!(!out.contains_key(&muet), "un pair à 1 arrivée est écarté");
        assert!(out[&bon].loss_pct.abs() < 1e-9, "lien bon = 0 % de perte");
        assert!((out[&perte].loss_pct - 1.0 / 6.0).abs() < 1e-9, "perte lue dans le trou de seq");
    }

    /// PRÉSENCE — le battement de cœur est un JSON minuscule, bien formé, qui PORTE le PC et la version.
    /// (Coût réseau négligeable : c'est l'observabilité « qui est en ligne quand » sans simulation.)
    #[test]
    fn heartbeat_json_bien_forme() {
        let h = heartbeat_json(1782520000, "PC-de-Tom", "871699e", "start", "");
        assert!(h.contains("\"ts\":1782520000"));
        assert!(h.contains("\"host\":\"PC-de-Tom\""));
        assert!(h.contains("\"ver\":\"871699e\""));
        assert!(h.contains("\"ev\":\"start\""));
        assert!(h.starts_with('{') && h.ends_with('}'));

        // OBSERVABILITÉ : les champs diagnostic s'insèrent AVANT la } finale, JSON valide.
        let d = heartbeat_json(1782520000, "PC-de-Tom", "0", "alive", ",\"peers\":3,\"recv\":120,\"sent\":5");
        assert!(d.contains("\"ev\":\"alive\",\"peers\":3,\"recv\":120,\"sent\":5}"));
        assert!(d.starts_with('{') && d.ends_with('}'));
    }

    /// Brique 5 — le garde-fou anti-binaire-corrompu : on n'installe QUE de l'ELF/PE assez gros.
    #[test]
    fn looks_like_exe_rejette_le_louche() {
        let mut elf = vec![0x7f, b'E', b'L', b'F'];
        elf.extend(std::iter::repeat(0u8).take(60_000));
        assert!(looks_like_exe(&elf)); // ELF assez gros → OK
        let mut pe = vec![b'M', b'Z'];
        pe.extend(std::iter::repeat(0u8).take(60_000));
        assert!(looks_like_exe(&pe)); // PE (Windows) assez gros → OK
        assert!(!looks_like_exe(b"404 Not Found")); // page d'erreur → REFUSÉ
        assert!(!looks_like_exe(&[0x7f, b'E', b'L', b'F'])); // ELF mais trop petit → REFUSÉ
        assert!(!looks_like_exe(&vec![0u8; 60_000])); // gros mais pas de magie → REFUSÉ
    }

    /// PRÉSENCE — le parsing de ligne heartbeat extrait correctement ts, host et ev.
    #[test]
    fn parse_heartbeat_bien_forme() {
        let line = "{\"ts\":1782577843,\"host\":\"nixos\",\"ver\":\"7253577\",\"ev\":\"alive\"}";
        let r = parse_heartbeat(line).unwrap();
        assert_eq!(r.ts, 1782577843);
        assert_eq!(r.host, "nixos");
        assert_eq!(r.ev, "alive");
    }

    /// B1 (28 juin) — anti-brick sur lien bas débit : `http_get_bytes` REFUSE un corps tronqué
    /// (plus court que le `Content-Length` annoncé) → on n'installe jamais un binaire incomplet ;
    /// un corps complet passe, coupé EXACTEMENT à la taille annoncée.
    #[test]
    fn telechargement_tronque_refuse_corps_complet_accepte() {
        use std::io::{Read, Write};
        // Le parseur d'en-tête (insensible à la casse, absent → None).
        assert_eq!(parse_content_length(b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\nConnection: close"), Some(5));
        assert_eq!(parse_content_length(b"HTTP/1.0 200 OK\r\nConnection: close"), None);

        // Mini-serveur qui répond UNE fois avec `resp`, puis ferme. Renvoie le port éphémère.
        fn serve_once(resp: &'static [u8]) -> u16 {
            let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
            let port = l.local_addr().unwrap().port();
            std::thread::spawn(move || {
                if let Ok((mut s, _)) = l.accept() {
                    let mut buf = [0u8; 256];
                    let _ = s.read(&mut buf); // on lit la requête, on l'ignore
                    let _ = s.write_all(resp); // puis close en sortant de la closure
                }
            });
            port
        }

        // (a) TRONQUÉ : annonce 10 octets, n'en envoie que 3 → REFUS (None).
        let p_tronque = serve_once(b"HTTP/1.0 200 OK\r\nContent-Length: 10\r\nConnection: close\r\n\r\nabc");
        assert_eq!(
            http_get_bytes("127.0.0.1", p_tronque, "/jeu-linux"),
            None,
            "un corps plus court que Content-Length doit être refusé (anti-brick)"
        );

        // (b) COMPLET : annonce 5, envoie 5 → on reçoit EXACTEMENT ces 5 octets.
        let p_ok = serve_once(b"HTTP/1.0 200 OK\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello");
        assert_eq!(
            http_get_bytes("127.0.0.1", p_ok, "/jeu-linux").as_deref(),
            Some(&b"hello"[..]),
            "un corps complet doit passer, coupé à la taille annoncée"
        );
    }

    /// « Inspecteur Eve » saison 2 (28 juin) : le verdict ne se laisse plus berner par un pair BRIDÉ
    /// (palier conscience), et rend le SILENCE (recv=0) explicite — distinct d'un vrai retard.
    /// Saison 3 (Levier A, 29 juin) : il ne DIFFAME plus un lien `real_loss~0` (sain mais lointain).
    #[test]
    fn verdict_cadence_conscient_trois_etats() {
        // Frais (focus) → vivant, quelle que soit la cadence/perte.
        assert_eq!(liveness_verdict(200.0, 50, 1, 0.5), "vivant");
        // p95 > 500 mais cadence BRIDÉE (~2 Hz conscience) → LOINTAIN basse fidélité, PAS mort.
        assert_eq!(liveness_verdict(900.0, 8, 10, 0.0), "lointain(basse-fidelite)");
        // p95 > 500 et AUCUNE arrivée (recv=0) → SILENCIEUX = le vrai suspect (relais/inclusivité).
        assert_eq!(liveness_verdict(900.0, 0, 0, 0.0), "MORT(silencieux)");

        // ⭐ LEVIER A — le cœur du fix. MÊME entrée (p95 900 ms, recv 40, cadence 1) : seule la PERTE
        // RÉELLE décide. Lien SAIN (real_loss 0 %, il délivre tout ce qu'il promet, juste lointain) →
        // VIVANT lointain, plus jamais « MORT ». Lien vraiment lossy (CGNAT 60 %) → MORT, lui, mérité.
        assert_eq!(liveness_verdict(900.0, 40, 1, 0.0), "lointain(basse-fidelite)");
        assert_eq!(liveness_verdict(900.0, 40, 1, 0.60), "MORT(>500ms)");
        // Le seuil (20 %) sépare net les deux populations observées en live.
        assert_eq!(liveness_verdict(900.0, 40, 1, 0.20), "lointain(basse-fidelite)");
        assert_eq!(liveness_verdict(900.0, 40, 1, 0.21), "MORT(>500ms)");
    }

    /// RESPECT DE L'HÔTE (29 juin) : le capteur de charge CPU répond une valeur SENSÉE (0..100 %) sur
    /// les plateformes gérées → l'agent peut décider de s'effacer quand le pote est occupé.
    #[test]
    fn cpu_busy_pct_est_sense() {
        if let Some(p) = cpu_busy_pct() {
            assert!((0.0..=100.0).contains(&p), "usage CPU attendu dans [0,100], obtenu {p}");
        }
        // None sur plateforme non gérée = acceptable (on ne bride pas → comportement inchangé).
    }
}
