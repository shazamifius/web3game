#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# netem-bench.sh — PREUVE Phase 3 : la redondance divise-t-elle VRAIMENT la perte
# par p sur un lien à perte ALÉATOIRE ? On injecte une perte connue avec `tc netem`
# et on fait passer le VRAI mécanisme (`jeu netem-bench` → encode/decode_state_bundle)
# à travers, pour comparer la perte résiduelle mesurée à la prédiction `p^K`.
#
# ⭐ ZÉRO SUDO. Contrairement à `sim-netem.sh` (qui exige root et dégrade TOUT le
#   localhost de la machine), ce script crée un NETWORK NAMESPACE ROOTLESS via
#   `unshare -rn` : on est « root » DANS ce namespace jetable, donc `tc netem` y
#   est permis sans mot de passe, et le netem ne touche QUE le trafic du banc — le
#   reste de ta machine n'est pas ralenti. Le namespace (et donc le netem) disparaît
#   tout seul à la fin de la commande. Rien à nettoyer.
#
# ⚠ Sens UNIQUE : le banc émet émetteur → récepteur (UNE traversée de `lo`), donc la
#   perte effective ≈ le taux netem (PAS de doublement comme un ping aller-retour).
#
# USAGE :
#   ./tools/netem-bench.sh [perte%] [n_états] [cadence_hz]
#     perte%    = perte aléatoire injectée par netem   (défaut 30)
#     n_états   = nombre d'états émis                   (défaut 4000)
#     cadence   = paquets/s                             (défaut 2000)
#
# EXEMPLES :
#   ./tools/netem-bench.sh 30          # perte 30 % → K=2 doit tomber vers 9 %, K=3 vers 2,7 %
#   ./tools/netem-bench.sh 50 6000     # perte 50 %, 6000 états pour resserrer la stat
# ----------------------------------------------------------------------------
set -uo pipefail

LOSS="${1:-30}"
N="${2:-4000}"
RATE="${3:-2000}"

cd "$(dirname "$0")/.." || exit 1

# --- 1) Vérifier que le netns rootless est permis (sinon : message clair) -----
if ! unshare -rn true 2>/dev/null; then
    echo "[netem-bench] ⚠ unshare -rn refusé sur cette machine (user namespaces désactivés)." >&2
    echo "              Repli : adapter sim-netem.sh (sudo) — mais ici on visait le zéro-sudo." >&2
    exit 1
fi

# --- 2) Construire le binaire (release = cadence stable) ----------------------
echo "[netem-bench] build release…"
if ! cargo build --release 2>&1 | tail -1; then
    echo "[netem-bench] build KO." >&2
    exit 1
fi
BIN="$(pwd)/target/release/jeu"
[ -x "$BIN" ] || { echo "[netem-bench] binaire introuvable : $BIN" >&2; exit 1; }

# --- 3) Netns rootless + netem loss + le banc ---------------------------------
# `limit 100000` : file netem très grande → SEULE la perte (loss%) façonne le résultat,
# jamais un débordement de file (cf. note dans sim-netem.sh).
echo "[netem-bench] netns rootless (AUCUN sudo) + netem loss ${LOSS}% sur lo, puis le banc :"
unshare -rn bash -c "
  set -e
  ip link set lo up
  tc qdisc add dev lo root netem loss ${LOSS}% limit 100000
  echo '[netem-bench] qdisc actif :'
  tc qdisc show dev lo | sed 's/^/        /'
  echo '----------------------------------------------------------------------'
  exec '$BIN' netem-bench ${LOSS} ${N} ${RATE}
"
