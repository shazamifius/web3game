#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# foule-3d.sh — ouvre une FOULE de N clients 3D (+ le rendez-vous) pour vérifier
# À L'ŒIL le rendu à deux tiers du chapitre 8.2c : FOCUS = avatars détaillés (les
# ~8 plus proches), CONSCIENCE = imposteurs LOD bon marché (le reste). But : voir
# qu'on AFFICHE bien PLUS que 64 silhouettes sans chute de FPS → ferme D24.
#
# USAGE :   ./tools/foule-3d.sh [N]        (N = nombre de fenêtres, défaut 40)
# EXEMPLE : ./tools/foule-3d.sh 80
#
# Chaque client ouvre SA fenêtre (app-id Wayland « web3game » → niri peut les ranger).
# Tu n'as PAS besoin de toutes les voir : pilote-en UNE (clic = capture souris, ZQSD/
# flèches, Échap = relâche) et regarde la foule des autres. Ils restent IMMOBILES :
# c'est normal, personne ne les pilote. Ctrl-C dans CE terminal ferme TOUT.
#
# Pré-requis : la même nix-shell que pour jouer (Bevy/Vulkan/Wayland). Le script la
# charge tout seul. Si une fenêtre ne s'ouvre pas, regarde /tmp/foule-c<i>.log.
# ----------------------------------------------------------------------------
set -uo pipefail

N="${1:-40}"
cd "$(dirname "$0")/.." || exit 1

echo "[foule-3d] compilation (release, le rendu d'une foule mérite l'optimisé)…"
nix-shell --run "cargo build --release" || { echo "[foule-3d] build raté."; exit 1; }

# On récupère l'environnement de la nix-shell UNE seule fois (LD_LIBRARY_PATH pour
# Vulkan/Wayland/alsa), puis on lance le binaire DIRECTEMENT → pas de nix-shell par
# fenêtre, donc démarrage rapide même pour 80 clients.
export LD_LIBRARY_PATH="$(nix-shell --run 'printf %s "$LD_LIBRARY_PATH"')"
BIN="target/release/jeu"

PIDS=()
cleanup() {
    echo
    echo "[foule-3d] fermeture du rendez-vous et des $N clients…"
    for p in "${PIDS[@]:-}"; do kill "$p" 2>/dev/null || true; done
}
trap cleanup EXIT INT TERM

echo "[foule-3d] démarrage du rendez-vous (annuaire)…"
"$BIN" rendezvous >/tmp/foule-rv.log 2>&1 &
PIDS+=($!)
sleep 1

echo "[foule-3d] ouverture de $N fenêtres clientes…"
for i in $(seq 1 "$N"); do
    # SCENE=arcade : les fenêtres de foule entrent DIRECTEMENT dans la salle arcade
    # (sautent le hub) → le test de perception/foule n'est pas gêné par l'aiguillage.
    SCENE=arcade "$BIN" client >/tmp/foule-c"$i".log 2>&1 &
    PIDS+=($!)
    sleep 0.15   # petit décalage : on n'inonde pas le rendez-vous d'un coup
done

echo "[foule-3d] $N clients lancés."
echo "[foule-3d] → Pilote UNE fenêtre (clic pour capturer la souris, ZQSD, Échap pour relâcher)."
echo "[foule-3d] → Tu dois voir ~8 avatars DÉTAILLÉS (focus, avec pseudo) + une FOULE d'imposteurs"
echo "[foule-3d]   (silhouettes simples, sans pseudo) — bien plus que 64 au total, sans lag."
echo "[foule-3d] → Ctrl-C ici ferme TOUT."
wait
