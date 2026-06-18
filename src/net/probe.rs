//! LA SONDE SYSTÈME (chapitre 7.4) : mesurer le COÛT RÉEL d'un nœud.
//!
//! Pour extrapoler honnêtement vers 55 000 joueurs (doute D19), il ne suffit pas de
//! savoir que « ça tient » : il faut chiffrer ce qu'UN nœud consomme. On mesure deux
//! choses, via le pseudo-système de fichiers `/proc` de Linux (zéro dépendance, zéro
//! `unsafe`) :
//!
//!   - le **temps CPU du thread courant** (`cpu_secs`) — combien de secondes de
//!     processeur CE nœud a réellement brûlé. Honnête MAIS à lire avec la tête :
//!     en simulation mono-PC sur `localhost`, l'envoi UDP ne coûte pas un vrai trajet
//!     réseau (pas de carte, pas de pile distante) ; ce chiffre mesure donc surtout la
//!     LOGIQUE + la CRYPTO (signer/vérifier Ed25519), pas le coût réseau d'un vrai
//!     déploiement. C'est quand même la part qui passe le plus mal à l'échelle.
//!
//!   - la **RAM crête du PROCESSUS** (`peak_rss_bytes`) — `VmHWM`, le plus haut niveau
//!     de mémoire résidente atteint. C'est une valeur GLOBALE (tous les threads + le
//!     rendez-vous + les attaquants + le code partagé + l'allocateur), PAS une mesure
//!     par nœud : un seul tas est partagé entre tous les threads d'un process, donc on
//!     NE PEUT PAS attribuer proprement la RAM à un thread. Diviser par le nombre de
//!     nœuds donne une moyenne grossière, à présenter comme telle — jamais comme une
//!     mesure exacte (ce serait la rustine qu'on s'interdit).

use std::fs;

/// Cadence des « tics » d'horloge du noyau (USER_HZ), unité de `utime`/`stime` dans
/// `/proc`. Vaut 100 sur la quasi-totalité des Linux (dont NixOS) → 1 tic = 10 ms.
/// On l'assume au lieu d'appeler `sysconf(_SC_CLK_TCK)` (qui exigerait `libc`) ; si une
/// cible exotique utilisait une autre valeur, seul l'affichage CPU serait à l'échelle.
const CLK_TCK: f64 = 100.0;

/// Temps CPU (utilisateur + système) consommé par le THREAD courant, en secondes.
/// Lu dans `/proc/thread-self/stat` (le thread appelant), donc utilisable tel quel
/// depuis chaque thread-nœud de la simulation. Renvoie `0.0` si la lecture échoue
/// (cible non-Linux, format inattendu) — la simulation continue, on n'a juste pas le
/// chiffre CPU.
pub(crate) fn thread_cpu_secs() -> f64 {
    let Ok(stat) = fs::read_to_string("/proc/thread-self/stat") else {
        return 0.0;
    };
    // Le champ 2 (« comm ») peut contenir espaces ET parenthèses : on coupe APRÈS la
    // DERNIÈRE `)` pour fiabiliser le découpage, puis on prend les champs suivants.
    let Some(after) = stat.rsplit_once(')').map(|(_, rest)| rest.trim()) else {
        return 0.0;
    };
    let f: Vec<&str> = after.split_whitespace().collect();
    // Après la `)`, l'index 0 = champ 3 (state). utime = champ 14 → index 11 ;
    // stime = champ 15 → index 12. (Cf. proc(5).)
    let utime: f64 = f.get(11).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    let stime: f64 = f.get(12).and_then(|s| s.parse().ok()).unwrap_or(0.0);
    (utime + stime) / CLK_TCK
}

/// RAM crête du PROCESSUS entier (high-water mark `VmHWM`), en octets. Survit à la fin
/// des threads (c'est un maximum historique), donc lisible APRÈS l'agrégation. Renvoie
/// `0` si indisponible. ⚠ Valeur GLOBALE, pas par nœud (cf. en-tête du module).
pub(crate) fn peak_rss_bytes() -> u64 {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmHWM:") {
            // Format : « VmHWM:\t   12345 kB ».
            if let Some(kb) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) {
                return kb * 1024;
            }
        }
    }
    0
}
