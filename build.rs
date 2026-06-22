//! Capture le hash COURT du commit git au moment de la compilation, exposé en `env!("GIT_HASH")`.
//! But : que CHAQUE binaire imprime au démarrage de quel build il s'agit → fini les « est-ce le bon
//! .exe ? » (un vieux téléchargement Windows relancé par erreur est démasqué à la seconde).
//! Robuste : si git est absent (build depuis une archive sans `.git`), on tombe sur « inconnu ».

use std::process::Command;

fn main() {
    let hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "inconnu".to_string());
    println!("cargo:rustc-env=GIT_HASH={hash}");
    // Recompiler le banner quand le commit change (sinon le hash resterait figé entre deux commits).
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");
}
