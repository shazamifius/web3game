//! LA FENÊTRE DE SESSION (transparence) — extraite de l'agent (`metrics.rs`) pour l'isoler de la
//! logique de mesure. Modèle décidé avec l'utilisateur : une session NE BLOQUE PAS sur un consentement
//! (les potes ne sont quasi jamais devant l'écran). Elle DÉMARRE TOUTE SEULE (ils ont accepté en
//! installant) mais s'affiche dans une fenêtre VISIBLE qui montre les logs EN DIRECT, avec deux boutons :
//! « Quitter la session » (déconnexion propre ~8 s → re-dispo à la session SUIVANTE) et « Réduire »
//! (continue en fond). Si le pote ne fait rien, ça continue et se range tout seul. Coordination
//! agent↔fenêtre par 3 fichiers dans TEMP (dep-free) :
//!   - web3_session.log    : l'agent écrit, la fenêtre affiche (tail).
//!   - web3_session.active : présent pendant la session ; la fenêtre se ferme quand il disparaît.
//!   - web3_quit.flag      : la fenêtre l'écrit au clic « Quitter » ; l'agent le lit → déconnexion propre.

use super::crypto::PeerId;
use super::linkstats::{percentile, LinkStats};

fn session_log_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_session.log") }
fn session_active_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_session.active") }
fn session_quit_file() -> std::path::PathBuf { std::env::temp_dir().join("web3_quit.flag") }

/// Ajoute une ligne au journal de session (que la fenêtre affiche). BORNÉ aux ~120 dernières lignes →
/// le fichier ne gonfle pas sur une longue session.
pub(crate) fn session_log_write(line: &str) {
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
pub(crate) fn session_summary_line(
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
pub(crate) fn session_window_open(session: u64) {
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
pub(crate) fn session_quit_requested() -> bool {
    session_quit_file().exists()
}

/// Ferme la fenêtre de session : retire le marqueur ACTIF (la fenêtre se ferme d'elle-même au prochain
/// tick) et le flag « quitter ».
pub(crate) fn session_window_close() {
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
pub(crate) enum SessionEnd {
    Normal,
    Updated,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
