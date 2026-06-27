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
    println!("[agent] session {} — mesure VISIBLE en cours (fenêtre {}s)…", start.session, start.window);
    session_window_open(start.session);
    send_heartbeat(cfg_host, CONFIG_PORT, "session");
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
        if win_start.elapsed().as_secs() >= window {
            measure_n += 1;
            let links = link_stats_by_peer(bot.take_link_arrivals(), 16.0);
            let lines = report_freshness(&epoch_secs().to_string(), &samples, &links);
            for l in &lines {
                let _ = http_post(cfg_host, CONFIG_PORT, "/upload", l); // brique 3
            }
            session_log_write(&session_summary_line(measure_n, &samples, &links)); // journal VISIBLE
            samples.clear();
            send_heartbeat(cfg_host, CONFIG_PORT, "alive"); // présence pendant la session
            // La session a-t-elle été ARRÊTÉE / changée côté serveur ? (serveur muet → on continue.)
            if let Some(c) = http_get(cfg_host, CONFIG_PORT, "/campaign").map(|b| parse_campaign(&b)) {
                if c.mode != Mode::Simulate || c.session != start.session {
                    println!("[agent] session {} terminée — retour au repos.", start.session);
                    session_log_write("Mesures terminees. La fenetre se ferme toute seule. Merci de ton aide !");
                    std::thread::sleep(std::time::Duration::from_millis(800));
                    session_window_close();
                    send_heartbeat(cfg_host, CONFIG_PORT, "idle");
                    return SessionEnd::Normal;
                }
                window = c.window; // la fenêtre peut être ajustée à chaud
            }
            if maybe_self_update(cfg_host, CONFIG_PORT) {
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
fn run_agent_loop(window: u64) {
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
            done.insert(campaign.session); // une session par identifiant
            println!("[agent] session {} demandée — démarrage VISIBLE (fenêtre + mesure).", campaign.session);
            if let SessionEnd::Updated = run_measure_session(&cfg_host, campaign) {
                return;
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
