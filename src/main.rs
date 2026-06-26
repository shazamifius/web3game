//! web3game — LE CŒUR RÉSEAU P2P, fait main (sans moteur 3D).
//!
//! Depuis la bascule vers Unreal (juin 2026), ce binaire n'embarque plus de fenêtre
//! de jeu : la PRÉSENTATION vit dans Unreal, qui parle au cœur via le **sidecar**
//! (socket locale, cf. `CONTRAT_SIDECAR.md`). Ce binaire expose donc uniquement les
//! outils headless du cœur : le rendez-vous, le sidecar, les bots, les simulations,
//! le programme attaquant et les bancs de mesure. Aucune dépendance Bevy : toute la
//! logique réseau est engine-agnostique (cf. `math::Vec3`, maison).
//!
//! # Modes (premier argument)
//!   rendezvous            l'annuaire (à lancer en premier)
//!   sidecar               le pont vers Unreal (socket locale 127.0.0.1:47800)
//!   bot <nom>             un client headless (le vrai protocole, sans 3D)
//!   agent                 l'agent de MESURE (v0) : fraîcheur / perte / gigue, en chiffres
//!   sim [N] [att] [s]     simulation massive : N nœuds + att attaquants
//!   crowd <N> [s]         foule dense au même endroit (couverture de perception)
//!   coopsim <N> [s]       N nœuds dans un thread coopératif (banc léger)
//!   coopsim-bus <N> [s]   banc bus mémoire (temps-sim découplé du mural)
//!   relay-test [s]        banc déterministe du relais NAT (deux sens)
//!   stars <seed> [secs]   champ d'étoiles déterministe (preuve : 2 runs = sortie identique)
//!   stars-race <s> <n> [t] preuve de convergence du ramassage (2 ordres = même décompte)
//!   attack <type>         le programme attaquant (forge|replay|flood|teleport|…)
//!   net-demo <a|b>        la démo réseau en texte (observer les paquets)
//!   nat-test <nom>        le hole punching en texte (pour les namespaces réseau)

mod math;
mod net;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(String::as_str);

    // BANNER DE BUILD : on imprime de quel commit est ce binaire. Démasque un vieux
    // binaire relancé par erreur → on ne devine plus quelle version tourne.
    println!("web3game build {} (mode: {})", env!("GIT_HASH"), mode.unwrap_or("?"));

    match mode {
        Some("rendezvous") => net::run_rendezvous(),
        Some("sidecar") => net::run_sidecar(),
        Some("net-demo") => net::run_demo(args.get(2).map(String::as_str).unwrap_or("a")),
        Some("attack") => net::run_attack(args.get(2).map(String::as_str).unwrap_or("forge")),
        Some("bot") => net::run_bot(args.get(2).map(String::as_str).unwrap_or("1")),
        Some("agent") => net::run_agent(
            args.get(2).map(String::as_str),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(20),
        ),
        Some("nat-test") => net::run_nat_test(args.get(2).map(String::as_str).unwrap_or("client")),
        Some("sim") => {
            let n_bots = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(50);
            let n_attackers = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3);
            let secs = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(15);
            net::run_sim(n_bots, n_attackers, secs);
        }
        Some("crowd") => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
            let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
            net::run_crowd(n, secs);
        }
        Some("coopsim") => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
            let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
            net::run_coopsim(n, secs);
        }
        Some("coopsim-bus") => {
            let n = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(200);
            let secs = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(15);
            net::run_coopsim_bus(n, secs);
        }
        Some("relay-test") => {
            let secs = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(10);
            net::run_relay_test(secs);
        }
        Some("stars") => net::run_stars(
            args.get(2).map(String::as_str).unwrap_or("1"),
            args.get(3).map(String::as_str).unwrap_or("30"),
        ),
        Some("stars-race") => net::run_stars_race(
            args.get(2).map(String::as_str).unwrap_or("1"),
            args.get(3).map(String::as_str).unwrap_or("4"),
            args.get(4).map(String::as_str).unwrap_or("120"),
        ),
        other => {
            if let Some(m) = other {
                eprintln!("Mode inconnu : « {m} ».");
            }
            eprintln!(
                "Usage : jeu <rendezvous|sidecar|bot|agent|sim|crowd|coopsim|coopsim-bus|\
                 relay-test|stars|stars-race|attack|net-demo|nat-test> [args…]\n\
                 (La présentation 3D vit désormais dans Unreal, branchée au mode `sidecar`.)"
            );
            std::process::exit(2);
        }
    }
}
