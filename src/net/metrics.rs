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
        Some("install") => run_agent_install(false),
        Some("uninstall") => run_agent_install(true),
        Some("recv") => {
            ensure_rendezvous_from_file();
            run_agent_recv(secs)
        }
        Some("loop") => {
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

    let links = link_stats_by_peer(bot.take_link_arrivals(), 16.0);
    report_freshness("", &samples, &links);
}

/// Construit le rapport par pair (un JSON), l'IMPRIME, et RENVOIE les lignes (pour l'upload). Partagé
/// par `recv` et `loop`. `ts` = horodatage époque (vide pour une mesure unique). La FRAÎCHEUR vient du
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
            let verdict = if p95 <= 500.0 { "vivant" } else { "MORT(>500ms)" };
            // Qualité de lien (perte/gigue/ré-ordre) si on a chiffré des arrivées pour ce pair.
            let quality = match links.get(id) {
                Some(s) => format!(
                    "\"recv\":{},\"expected\":{},\"loss_pct\":{:.1},\"reorder_pct\":{:.1},\"jitter_ms\":{:.1},",
                    s.received, s.expected, s.loss_pct * 100.0, s.reorder_pct * 100.0, s.jitter_ms
                ),
                None => String::new(),
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
fn heartbeat_json(ts: u64, host: &str, ver: &str, ev: &str) -> String {
    format!("{{\"ts\":{ts},\"host\":\"{host}\",\"ver\":\"{ver}\",\"ev\":\"{ev}\"}}")
}

/// Envoie un battement de cœur au collecteur (POST /heartbeat). Best-effort : un échec est silencieux
/// (serveur muet → on n'insiste pas, l'agent ne se bloque jamais). C'est l'observabilité « qui est en
/// ligne, quand » SANS lancer de simulation — juste savoir quels PC sont dispo et à quelles heures.
fn send_heartbeat(host_addr: &str, port: u16, ev: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let body = heartbeat_json(ts, &host_label(), &agent_version(), ev);
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
}
impl Default for Campaign {
    fn default() -> Self {
        Campaign { window: 30, mode: Mode::Idle, session: 0 }
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
                _ => {}
            }
        }
    }
    c
}

/// GET HTTP/1.0 BINAIRE, SANS dépendance (std seulement) — sert au fetch de campagne ET au
/// téléchargement du nouveau binaire (auto-update). Renvoie le CORPS (octets) sur 200, sinon None.
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
    Some(resp[pos + 4..].to_vec())
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
        if uninstall {
            let _ = std::process::Command::new("schtasks").args(["/delete", "/tn", tn, "/f"]).status();
            let _ = std::fs::remove_file(&vbs);
            println!("[install] auto-démarrage RETIRÉ (tâche {tn}).");
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
        // C'est ce que la tâche exécute (via `wscript`), au lieu de l'exe directement (qui ouvre une
        // console). Dep-free (juste un fichier texte). `0` = caché, `False` = ne pas attendre.
        let vbs_body = format!(
            "Set s = CreateObject(\"WScript.Shell\")\r\ns.Run \"\"\"{}\"\" agent loop\", 0, False\r\n",
            dest.to_string_lossy()
        );
        let _ = std::fs::write(&vbs, vbs_body);
        let tr = format!("wscript.exe \"{}\"", vbs.to_string_lossy());
        let st = std::process::Command::new("schtasks")
            .args(["/create", "/tn", tn, "/tr", &tr, "/sc", "onlogon", "/f"])
            .status();
        match st {
            Ok(s) if s.success() => {
                println!("[install] ✅ DÉMARRAGE AUTO installé — tâche « {tn} » à chaque ouverture de session (fenêtre cachée).");
                println!("[install] dossier : {}", dir.to_string_lossy());
                // Démarrage immédiat, AUSSI caché (via le shim) → pas de terminal qui s'ouvre.
                let _ = std::process::Command::new("wscript.exe").arg(&vbs).spawn();
                println!("[install] agent démarré (en tâche de fond, sans fenêtre).");
            }
            _ => eprintln!("[install] échec de la création de la tâche planifiée."),
        }
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
            "[Unit]\nDescription=web3 agent de mesure\n\n[Service]\nExecStart={} agent loop\nWorkingDirectory={}\nRestart=always\n\n[Install]\nWantedBy=default.target\n",
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

/// La décision du pote face à la demande de session. `Decline` est le DÉFAUT SÛR : tout ce qui n'est
/// pas un « oui » franc (refus, fermeture, délai dépassé, pas d'outil de dialogue) = on ne mesure pas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Consent {
    Accept,
    Decline,
}

/// Mappe le CODE DE SORTIE d'un dialogue Unix (zenity/kdialog : 0 = bouton OUI) en décision. PUR.
/// Tout sauf 0 (refus, annulation, timeout=5 de zenity, outil absent) → `Decline` (refus par défaut).
#[cfg_attr(not(unix), allow(dead_code))] // utilisé sur Unix + tests
fn consent_from_unix_status(code: Option<i32>) -> Consent {
    if code == Some(0) { Consent::Accept } else { Consent::Decline }
}

/// Mappe la sortie de la fenêtre WinForms Windows (`DialogResult.ToString()`) en décision. PUR.
/// « Yes » (bouton Accepter) → Accept ; tout le reste (« No », « Cancel », timeout, vide) → Decline.
#[cfg_attr(not(windows), allow(dead_code))] // utilisé sur Windows + tests
fn consent_from_windows_popup(stdout: &str) -> Consent {
    if stdout.trim().eq_ignore_ascii_case("Yes") { Consent::Accept } else { Consent::Decline }
}

/// Le script PowerShell (ASCII) de la fenêtre de consentement Windows. `__SESSION__` est remplacé par
/// le numéro de session. WinForms FORCÉ au premier plan (TopMost + Activate + BringToFront) + minuteur
/// 60 s (pas de réponse → Refuser). Écrit « Yes »/« No » sur stdout. (Remplace `WScript.Shell.Popup`
/// qui s'exécutait mais restait INVISIBLE.)
#[cfg(windows)]
const WINDOWS_CONSENT_PS1: &str = r#"Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$form = New-Object System.Windows.Forms.Form
$form.Text = 'web3 - mesure reseau'
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$form.FormBorderStyle = 'FixedDialog'
$form.MinimizeBox = $false
$form.MaximizeBox = $false
$form.Width = 490
$form.Height = 215
$lbl = New-Object System.Windows.Forms.Label
$lbl.Text = "web3 (R&D entre potes) veut lancer une courte session de mesure reseau (session __SESSION__).`r`nCa prend un peu de reseau pendant la session. Tu peux refuser sans souci.`r`n`r`nLancer la session ?"
$lbl.SetBounds(15, 15, 450, 95)
$yes = New-Object System.Windows.Forms.Button
$yes.Text = 'Accepter'
$yes.DialogResult = [System.Windows.Forms.DialogResult]::Yes
$yes.SetBounds(90, 125, 130, 38)
$no = New-Object System.Windows.Forms.Button
$no.Text = 'Refuser'
$no.DialogResult = [System.Windows.Forms.DialogResult]::No
$no.SetBounds(265, 125, 130, 38)
$form.Controls.Add($lbl)
$form.Controls.Add($yes)
$form.Controls.Add($no)
$form.AcceptButton = $yes
$form.CancelButton = $no
$timer = New-Object System.Windows.Forms.Timer
$timer.Interval = 60000
$timer.add_Tick({ $timer.Stop(); $form.DialogResult = [System.Windows.Forms.DialogResult]::No; $form.Close() })
$timer.Start()
$form.add_Shown({ $form.Activate(); $form.BringToFront() })
$res = $form.ShowDialog()
[Console]::Out.Write($res.ToString())
"#;

/// DEMANDE LE CONSENTEMENT au pote avant toute session de mesure (popup Accepter/Refuser, ~60 s).
/// Dep-free : on shell-out vers l'outil de dialogue du système. Repli = `Decline` (jamais de mesure
/// non consentie). `WEB3_AGENT_AUTOCONSENT=accept|decline` court-circuite le popup (mes propres PC, et
/// les tests déterministes) — JAMAIS sur les machines des potes (variable non posée → vrai popup).
fn ask_consent(session: u64) -> Consent {
    if let Ok(v) = std::env::var("WEB3_AGENT_AUTOCONSENT") {
        return if v.eq_ignore_ascii_case("accept") { Consent::Accept } else { Consent::Decline };
    }
    #[cfg(windows)]
    {
        // Vraie fenêtre WinForms FORCÉE au premier plan (TopMost + Activate + BringToFront), centrée,
        // avec boutons Accepter/Refuser et un minuteur de 60 s (pas de réponse → Refuser). On l'écrit
        // dans un .ps1 TEMP (ASCII → zéro souci d'encodage en ligne de commande) puis on l'exécute en
        // -STA (nécessaire à WinForms). Le `WScript.Shell.Popup` d'avant S'EXÉCUTAIT mais restait
        // INVISIBLE (prouvé : il bloquait 60 s puis expirait) → on ne pouvait pas accepter.
        let ps = WINDOWS_CONSENT_PS1.replace("__SESSION__", &session.to_string());
        let path = std::env::temp_dir().join("web3_consent.ps1");
        if std::fs::write(&path, &ps).is_err() {
            return Consent::Decline;
        }
        match std::process::Command::new("powershell")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-STA", "-File"])
            .arg(&path)
            .output()
        {
            Ok(o) => return consent_from_windows_popup(&String::from_utf8_lossy(&o.stdout)),
            Err(_) => return Consent::Decline,
        }
    }
    #[cfg(unix)]
    {
        let text = format!(
            "web3 (R&D entre potes) aimerait lancer une courte session de mesure réseau (session {session}).\n\
             Ça consomme un peu de réseau pendant la session. Tu peux refuser sans souci.\n\nLancer la session ?"
        );
        // zenity puis kdialog (l'un des deux est presque toujours là sur un bureau Linux).
        let zen = std::process::Command::new("zenity")
            .args(["--question", "--title=web3 — mesure", &format!("--text={text}"),
                   "--ok-label=Accepter", "--cancel-label=Refuser", "--timeout=60"])
            .status();
        if let Ok(s) = zen {
            return consent_from_unix_status(s.code());
        }
        let kde = std::process::Command::new("kdialog")
            .args(["--yesno", &text, "--title", "web3 — mesure"])
            .status();
        if let Ok(s) = kde {
            return consent_from_unix_status(s.code());
        }
        // Aucun outil de dialogue → on PRÉVIENT (best-effort) et on REFUSE par défaut.
        let _ = std::process::Command::new("notify-send")
            .args(["web3", "Session de mesure demandée mais aucun dialogue dispo → refusée."])
            .status();
        Consent::Decline
    }
}

/// Issue d'une session de mesure : terminée normalement, ou interrompue par une AUTO-UPDATE (l'appelant
/// doit alors sortir, le nouveau process prend le relais).
enum SessionEnd {
    Normal,
    Updated,
}

/// UNE SESSION DE MESURE consentie : on crée le nœud, on mesure fenêtre par fenêtre (fraîcheur +
/// perte/gigue/ré-ordre), on uploade, et on RE-LIT la campagne à chaque fenêtre — si le serveur repasse
/// en `idle` ou change de `session`, on s'arrête et on revient au repos. Le nœud P2P n'existe QUE
/// pendant la session → au repos, zéro trafic de jeu (juste le battement de cœur).
fn run_measure_session(cfg_host: &str, start: Campaign) -> SessionEnd {
    let mut bot = match Bot::new("agent", false, 0.0) {
        Some(b) => b,
        None => {
            eprintln!("[agent] réseau indisponible (le rendez-vous est-il joignable ?) — session annulée.");
            return SessionEnd::Normal;
        }
    };
    println!("[agent] ✅ session {} acceptée — mesure en cours (fenêtre {}s)…", start.session, start.window);
    send_heartbeat(cfg_host, CONFIG_PORT, "session");
    let boot = Instant::now();
    let mut last = Instant::now();
    let mut win_start = Instant::now();
    let mut samples: std::collections::HashMap<PeerId, Vec<f64>> = std::collections::HashMap::new();
    let mut first = true;
    let mut window = start.window;
    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();
        bot.step(dt, boot.elapsed().as_secs_f32()); // horloge CONTINUE entre fenêtres (pas de saut)
        if !first || win_start.elapsed().as_secs_f64() >= 3.0 {
            for (id, age) in bot.peer_freshness_ms() {
                samples.entry(id).or_default().push(age);
            }
        }
        if win_start.elapsed().as_secs() >= window {
            let links = link_stats_by_peer(bot.take_link_arrivals(), 16.0);
            let lines = report_freshness(&epoch_secs().to_string(), &samples, &links);
            for l in &lines {
                let _ = http_post(cfg_host, CONFIG_PORT, "/upload", l); // brique 3
            }
            samples.clear();
            send_heartbeat(cfg_host, CONFIG_PORT, "alive"); // présence pendant la session
            // La session a-t-elle été ARRÊTÉE / changée côté serveur ? (serveur muet → on continue.)
            if let Some(c) = http_get(cfg_host, CONFIG_PORT, "/campaign").map(|b| parse_campaign(&b)) {
                if c.mode != Mode::Simulate || c.session != start.session {
                    println!("[agent] session {} terminée — retour au repos.", start.session);
                    send_heartbeat(cfg_host, CONFIG_PORT, "idle");
                    return SessionEnd::Normal;
                }
                window = c.window; // la fenêtre peut être ajustée à chaud
            }
            if maybe_self_update(cfg_host, CONFIG_PORT) {
                return SessionEnd::Updated; // le nouveau process reprend
            }
            win_start = Instant::now();
            first = false;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

/// LA BOUCLE DE L'AGENT (calme + consentie). Au REPOS : aucun nœud P2P, juste un battement de cœur
/// léger (présence « qui est en ligne quand ») + une relecture de campagne + l'auto-update. Quand le
/// serveur demande une session (`mode=simulate` + un `session` neuf), on demande le CONSENTEMENT au
/// pote (popup) : accepté → on mesure ; refusé/pas de réponse → on reste au repos, et on ne re-demande
/// PAS pour cette même session (une question par `session`). Le pote garde toujours la main.
fn run_agent_loop(window: u64) {
    let cfg_host = super::link::rendezvous_addr().ip().to_string();
    let fallback_window = window.max(5);
    println!(
        "[agent] démarré — au REPOS (battement de cœur). En attente d'une session sur \
         http://{cfg_host}:{CONFIG_PORT}/campaign (Ctrl-C pour arrêter)"
    );
    send_heartbeat(&cfg_host, CONFIG_PORT, "start");
    let mut decided: std::collections::HashSet<u64> = std::collections::HashSet::new();
    loop {
        let campaign = http_get(&cfg_host, CONFIG_PORT, "/campaign")
            .map(|b| parse_campaign(&b))
            .unwrap_or(Campaign { window: fallback_window, ..Campaign::default() });
        // AUTO-UPDATE : au repos, c'est le moment SÛR pour s'échanger (aucune session en cours).
        if maybe_self_update(&cfg_host, CONFIG_PORT) {
            return; // le nouveau process prend le relais
        }
        if campaign.mode == Mode::Simulate && !decided.contains(&campaign.session) {
            decided.insert(campaign.session); // une seule question par session
            println!("[agent] session {} demandée par le serveur — demande de consentement…", campaign.session);
            match ask_consent(campaign.session) {
                Consent::Accept => {
                    if let SessionEnd::Updated = run_measure_session(&cfg_host, campaign) {
                        return;
                    }
                }
                Consent::Decline => {
                    println!("[agent] session {} refusée (ou pas de réponse) — on reste au repos.", campaign.session);
                    send_heartbeat(&cfg_host, CONFIG_PORT, "decline");
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

    /// Brique B — le mode/session se parse, et le DÉFAUT le plus SÛR est le REPOS (jamais de mesure
    /// surprise) : campagne vide ou mode inconnu → `Idle`. `simulate` (insensible à la casse) → mesure.
    #[test]
    fn campagne_mode_et_session_defaut_repos() {
        assert_eq!(parse_campaign("").mode, Mode::Idle); // défaut = repos
        assert_eq!(parse_campaign("mode=bidon").mode, Mode::Idle); // inconnu → repos (sûr)
        assert_eq!(parse_campaign("mode=Simulate").mode, Mode::Simulate); // casse ignorée
        assert_eq!(parse_campaign("mode=simulate\nsession=7\n").session, 7);
        assert_eq!(parse_campaign("").session, 0);
        // une campagne complète et réaliste :
        let c = parse_campaign("window=20\nmode=simulate\nsession=42\n");
        assert_eq!((c.window, c.mode, c.session), (20, Mode::Simulate, 42));
    }

    /// Brique B — le CONSENTEMENT par défaut est le REFUS : seul un « oui » franc (code 0 Unix, « 6 »
    /// Windows) accepte ; refus/annulation/timeout/outil-absent → on ne mesure pas (jamais d'agression).
    #[test]
    fn consentement_refus_par_defaut() {
        assert_eq!(consent_from_unix_status(Some(0)), Consent::Accept);
        assert_eq!(consent_from_unix_status(Some(1)), Consent::Decline); // refus
        assert_eq!(consent_from_unix_status(Some(5)), Consent::Decline); // timeout zenity
        assert_eq!(consent_from_unix_status(None), Consent::Decline); // tué par un signal
        assert_eq!(consent_from_windows_popup("Yes"), Consent::Accept);
        assert_eq!(consent_from_windows_popup("yes\r\n"), Consent::Accept); // trim + casse
        assert_eq!(consent_from_windows_popup("No"), Consent::Decline); // Refuser
        assert_eq!(consent_from_windows_popup("Cancel"), Consent::Decline); // fermé
        assert_eq!(consent_from_windows_popup(""), Consent::Decline); // timeout/rien → refus
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
        let h = heartbeat_json(1782520000, "PC-de-Tom", "871699e", "start");
        assert!(h.contains("\"ts\":1782520000"));
        assert!(h.contains("\"host\":\"PC-de-Tom\""));
        assert!(h.contains("\"ver\":\"871699e\""));
        assert!(h.contains("\"ev\":\"start\""));
        assert!(h.starts_with('{') && h.ends_with('}'));
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
}
