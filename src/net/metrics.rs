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
//! **v0 (ce fichier) = la logique de mesure + le format de rapport, prouvés par un test
//! déterministe.** Ce qu'il ne fait PAS encore (honnêteté) : se brancher sur de vrais pairs
//! (prochain pas), et le score « robotique » (l'ampleur des corrections de dead-reckoning),
//! qui a besoin du modèle d'interpolation → il arrive avec le branchement live.

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
    pub expected: u64,     // attendus sur la plage de seq (max − min + 1)
    pub loss_pct: f64,     // perte : 1 − reçus / attendus
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
         \"loss_pct\":{:.2},\"reorder_pct\":{:.2},\"jitter_ms\":{:.1},\
         \"fresh_p50_ms\":{:.1},\"fresh_p95_ms\":{:.1},\"fresh_max_ms\":{:.1}}}",
        s.received,
        s.expected,
        s.loss_pct * 100.0,
        s.reorder_pct * 100.0,
        s.jitter_ms,
        s.fresh_p50_ms,
        s.fresh_p95_ms,
        s.fresh_max_ms,
    )
}

/// L'agent (v0). Sans argument → DÉMO (flux synthétiques, le format de rapport). `recv [secs]` →
/// mesure LIVE : on rejoint le rendez-vous comme un vrai nœud et on chiffre la fraîcheur des pairs.
pub fn run_agent(mode: Option<&str>, secs: u64) {
    match mode {
        Some("recv") => run_agent_recv(secs),
        Some("loop") => run_agent_loop(secs),
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

    let tick = 16.0; // l'observateur « regarde » à ~60 Hz (pas de rendu)
    println!("agent v0 — mètre étalon (flux synthétiques ; cible fraîcheur ≤ 500 ms)");
    println!("{}", report_json("moi", "lien_bon", &link_stats(&bon, tick)));
    println!("{}", report_json("moi", "lien_mauvais", &link_stats(&mauvais, tick)));
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

    report_freshness("", &samples);
}

/// Imprime un rapport de fraîcheur (un JSON par pair) — partagé par `recv` et `loop`.
/// `ts` = horodatage époque (vide pour une mesure unique ; rempli dans la boucle → série temporelle).
fn report_freshness(ts: &str, samples: &std::collections::HashMap<PeerId, Vec<f64>>) {
    let tsf = if ts.is_empty() { String::new() } else { format!("\"ts\":{ts},") };
    if samples.is_empty() {
        println!("{{{tsf}\"note\":\"aucun pair vu\"}}");
        return;
    }
    for (id, ages) in samples {
        let mut a = ages.clone();
        a.sort_by(|x, y| x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal));
        let p95 = percentile(&a, 95.0);
        let verdict = if p95 <= 500.0 { "vivant" } else { "MORT(>500ms)" };
        println!(
            "{{{tsf}\"observer\":\"agent\",\"target\":\"{}\",\"samples\":{},\"fresh_p50_ms\":{:.0},\
             \"fresh_p95_ms\":{:.0},\"fresh_max_ms\":{:.0},\"verdict\":\"{}\"}}",
            id.short(), a.len(), percentile(&a, 50.0), p95, a.last().copied().unwrap_or(0.0), verdict
        );
    }
}

/// Port HTTP où le serveur sert la CAMPAGNE (sur la même machine que le rendez-vous).
const CONFIG_PORT: u16 = 24001;

/// La CAMPAGNE : ce que l'agent doit faire, décidé CENTRALEMENT (je l'édite sur le serveur, les
/// agents suivent — brique 2). Format « clé=valeur » par ligne → zéro dépendance JSON (fait-main).
#[derive(Clone, Debug)]
struct Campaign {
    window: u64,
}
impl Default for Campaign {
    fn default() -> Self {
        Campaign { window: 30 }
    }
}

/// Parse une campagne « clé=valeur ». ROBUSTE : tout champ absent/illisible garde le défaut, et on
/// ignore l'inconnu → l'agent ne casse JAMAIS sur une config foireuse (self-sufficient).
fn parse_campaign(body: &str) -> Campaign {
    let mut c = Campaign::default();
    for line in body.lines() {
        if let Some((k, v)) = line.trim().split_once('=') {
            if k.trim() == "window" {
                if let Ok(n) = v.trim().parse::<u64>() {
                    c.window = n.clamp(5, 3600);
                }
            }
        }
    }
    c
}

/// GET HTTP/1.0 minimaliste, SANS dépendance (std seulement). Renvoie le CORPS, ou None si le
/// serveur ne répond pas (l'agent garde alors sa config courante → self-sufficient).
fn http_get(host: &str, port: u16, path: &str) -> Option<String> {
    use std::io::{Read, Write};
    let timeout = std::time::Duration::from_secs(3);
    let mut stream = std::net::TcpStream::connect((host, port)).ok()?;
    stream.set_read_timeout(Some(timeout)).ok()?;
    stream.set_write_timeout(Some(timeout)).ok()?;
    let req = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
    stream.write_all(req.as_bytes()).ok()?;
    let mut resp = String::new();
    stream.read_to_string(&mut resp).ok()?;
    resp.split_once("\r\n\r\n").map(|(_, body)| body.to_string())
}

/// MESURE AUTONOME pilotée par CONFIG CENTRALE (briques 1+2 de l'agent self-suffisant) : un nœud
/// qui RESTE connecté, émet un rapport horodaté à chaque fenêtre, et RE-LIT sa campagne sur le
/// serveur → je change ce qu'il fait À DISTANCE, il suit sans relance. (Suivront : upload des
/// rapports, démarrage auto, auto-update.)
fn run_agent_loop(window: u64) {
    let mut bot = match Bot::new("agent", false, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[agent] réseau indisponible (le rendez-vous est-il joignable ?).");
            return;
        }
    };
    // La campagne est servie sur la MÊME machine que le rendez-vous.
    let cfg_host = super::link::rendezvous_addr().ip().to_string();
    let mut campaign = Campaign { window: window.max(5) };
    if let Some(body) = http_get(&cfg_host, CONFIG_PORT, "/campaign") {
        campaign = parse_campaign(&body);
        println!("[agent] campagne reçue du serveur : window={}s", campaign.window);
    }
    println!(
        "[agent] mesure AUTONOME en boucle — campagne sur http://{cfg_host}:{CONFIG_PORT}/campaign (Ctrl-C pour arrêter)"
    );
    let boot = Instant::now();
    let mut last = Instant::now();
    let mut win_start = Instant::now();
    let mut samples: std::collections::HashMap<PeerId, Vec<f64>> = std::collections::HashMap::new();
    let mut first = true;
    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        bot.step(dt, boot.elapsed().as_secs_f32()); // horloge CONTINUE entre fenêtres (pas de saut)
        if !first || win_start.elapsed().as_secs_f64() >= 3.0 {
            for (id, age) in bot.peer_freshness_ms() {
                samples.entry(id).or_default().push(age);
            }
        }
        if win_start.elapsed().as_secs() >= campaign.window {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            report_freshness(&ts.to_string(), &samples);
            samples.clear();
            // PILOTAGE À DISTANCE : on relit la campagne entre deux fenêtres ; serveur muet → on
            // garde la courante (jamais de blocage).
            if let Some(body) = http_get(&cfg_host, CONFIG_PORT, "/campaign") {
                campaign = parse_campaign(&body);
            }
            win_start = Instant::now();
            first = false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// SERT la campagne en HTTP (côté serveur de la flotte) — dep-free (std uniquement), réponse
/// statique RELUE à chaque requête : j'édite le fichier, les agents voient le changement au tour
/// suivant. Mono-connexion (largement suffisant pour des fetchs de config espacés).
pub fn run_serve_config(file: &str, port: u16) {
    use std::io::{Read, Write};
    let listener = match std::net::TcpListener::bind(("0.0.0.0", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("config : impossible d'écouter sur {port} : {e}");
            return;
        }
    };
    println!("config servie sur 0.0.0.0:{port} depuis « {file} » (Ctrl-C pour arrêter)");
    for stream in listener.incoming() {
        let mut stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
        let mut scratch = [0u8; 1024];
        let _ = stream.read(&mut scratch); // on lit (et ignore) la requête GET
        let body = std::fs::read_to_string(file).unwrap_or_default();
        let resp = format!(
            "HTTP/1.0 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
    }
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
}
