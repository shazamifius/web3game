#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# sim-netem.sh — fait tourner la SIMULATION MASSIVE derrière une VRAIE mauvaise
# connexion, sur une seule machine, grâce à `tc netem` sur l'interface loopback
# `lo`. On ajoute latence / jitter / perte / ré-ordonnancement à TOUT le trafic
# localhost (donc à tous nos nœuds de simu qui se parlent en local), on lance
# `cargo run --release -- sim …`, puis on RETIRE proprement le netem à la sortie
# (fin normale, Ctrl-C ou kill : le retrait est GARANTI par un `trap`, comme
# tools/sim-cool.sh le fait pour les ventilos).
#
# But (chapitre 7.1 de la feuille de route) : arrêter de « mentir comme
# localhost ». Un netcode parfait sur `lo` peut s'écrouler à 120 ms + 2 % de
# perte (prédiction qui part en vrille, anti-rejeu qui jette des paquets
# re-ordonnés, fausses migrations d'orbe). Ce script est le labo qui le révèle.
#
# ⚠ PIÈGE IMPORTANT — le délai compte DOUBLE sur `lo`.
#   Le trafic loopback traverse `lo` à l'aller ET au retour. Un `delay 60ms`
#   donne donc un PING (aller-retour) d'environ 120 ms. Pour que ce soit lisible,
#   les profils ci-dessous sont exprimés en PING CIBLE (ce que tout le monde a en
#   tête), et on applique en interne delay = ping / 2.
#
# ⚠ netem sur `lo` ralentit TOUT le localhost pendant le run (pas seulement la
#   simu). C'est voulu (toute la simu est en localhost). Le `trap` rend la main
#   à la fin, quoi qu'il arrive.
#
# USAGE :
#   ./tools/sim-netem.sh <profil> [bots] [attaquants] [secondes]
#     profil   = bon | moyen | mauvais   (obligatoire)
#     bots     = nombre de nœuds honnêtes        (défaut 50)
#     attaquants = nombre d'attaquants variés     (défaut 5)
#     secondes = durée de la fenêtre de simu      (défaut 20)
#
# PROFILS (ping cible / perte / jitter) :
#   bon      ~30 ms,  0 % perte               (bonne fibre)
#   moyen    ~120 ms, 2 % perte, jitter        (ADSL / 4G correcte)
#   mauvais  ~250 ms, 5 % perte, jitter + ré-ordonnancement (mobile médiocre)
#
# EXEMPLES :
#   ./tools/sim-netem.sh moyen                 # 50 bots, 5 attaquants, 20 s, ~120 ms
#   ./tools/sim-netem.sh mauvais 300 5 20      # le gros run, sous ~250 ms + 5 %
#   # combiné aux ventilos pour un gros run :
#   ./tools/sim-cool.sh ./tools/sim-netem.sh mauvais 300 5 30
#
# `tc` exige les droits root → le script utilise `sudo` UNIQUEMENT pour netem
# (il demandera ton mot de passe une fois). `cargo`, lui, tourne en utilisateur
# normal — on ne compile/lance JAMAIS le jeu en root.
# ----------------------------------------------------------------------------
set -uo pipefail

IFACE="lo"

# --- 1) Lire le profil et traduire en paramètres netem -----------------------
PROFIL="${1:-}"
if [ -z "$PROFIL" ]; then
    echo "Usage : $0 <bon|moyen|mauvais> [bots] [attaquants] [secondes]" >&2
    exit 2
fi
shift || true

BOTS="${1:-50}"
ATTAQUANTS="${2:-5}"
SECONDES="${3:-20}"

# File netem : nombre max de paquets que la qdisc retient. Le DÉFAUT de `tc netem`
# (1000) plafonne le débit à ~limit/délai — ex. 1000/0,125 s ≈ 8000 paq/s à 125 ms.
# Ce n'est PAS une limite du jeu mais du HARNAIS (découvert au ch. 7.3 : sans ça, le
# profil « mauvais » faussait la mesure du débit). On la met très grande pour que SEULS
# latence / jitter / perte / ré-ordonnancement façonnent le trafic, comme un vrai lien.
NETEM_LIMIT=100000

# Les délais ci-dessous sont des DEMI-pings (ping cible ÷ 2), à cause de l'effet
# « double traversée » de `lo` décrit en tête de fichier.
case "$PROFIL" in
    bon)
        PING=30
        NETEM_ARGS=(limit "$NETEM_LIMIT" delay 15ms 2ms)
        ;;
    moyen)
        PING=120
        NETEM_ARGS=(limit "$NETEM_LIMIT" delay 60ms 10ms distribution normal loss 2%)
        ;;
    mauvais)
        PING=250
        NETEM_ARGS=(limit "$NETEM_LIMIT" delay 125ms 25ms distribution normal loss 5% reorder 25% 50%)
        ;;
    *)
        echo "Profil inconnu : « $PROFIL » (attendu : bon | moyen | mauvais)" >&2
        exit 2
        ;;
esac

# --- 2) Retrait propre du netem (appelé par le trap, quoi qu'il arrive) -------
cleanup() {
    echo ""
    echo "[netem] retrait du netem sur $IFACE (retour à la normale)…"
    sudo tc qdisc del dev "$IFACE" root 2>/dev/null || true
    # Confirme que lo est revenu à la normale (plus de ligne « netem »).
    if sudo tc qdisc show dev "$IFACE" | grep -q netem; then
        echo "[netem] ⚠ ATTENTION : du netem subsiste sur $IFACE — à retirer à la main :"
        echo "        sudo tc qdisc del dev $IFACE root"
    else
        echo "[netem] $IFACE est propre. ✓"
    fi
}
trap cleanup EXIT INT TERM

# --- 3) Demander le mot de passe sudo une fois, d'emblée ---------------------
echo "[netem] (tc a besoin des droits root — mot de passe demandé une fois)"
sudo -v || { echo "[netem] sudo refusé, on arrête."; exit 1; }

# --- 4) Appliquer le netem ----------------------------------------------------
# `replace` pose la règle qu'il y en ait déjà une ou non (idempotent). Sur `lo`
# il n'y a normalement aucune qdisc personnalisée (défaut « noqueue »), donc le
# retrait du §2 rend bien `lo` à son état d'origine.
echo "[netem] profil « $PROFIL » → ping cible ~${PING} ms (delay appliqué = moitié, sur $IFACE)"
echo "[netem] tc qdisc replace dev $IFACE root netem ${NETEM_ARGS[*]}"
if ! sudo tc qdisc replace dev "$IFACE" root netem "${NETEM_ARGS[@]}"; then
    echo "[netem] ⚠ échec de l'application du netem (module sch_netem indisponible ?)." >&2
    exit 1
fi
echo "[netem] règle active :"
sudo tc qdisc show dev "$IFACE" | sed 's/^/        /'

# --- 5) Lancer la simulation (en utilisateur normal, surtout PAS en root) -----
echo "----------------------------------------------------------------------"
echo "[netem] lancement : cargo run --release -- sim $BOTS $ATTAQUANTS $SECONDES"
echo "----------------------------------------------------------------------"
nix-shell --run "cargo run --release -- sim $BOTS $ATTAQUANTS $SECONDES"
status=$?
echo "----------------------------------------------------------------------"
# cleanup() est appelé automatiquement par le trap juste après cette ligne.
exit $status
