#!/usr/bin/env bash
#
# test-nat.sh — LE VRAI test du hole punching MULTI-JOUEURS, sur UN seul PC.
#
# On simule « N maisons derrière N box Internet » avec des NAMESPACES RÉSEAU
# (ip netns) : N machines isolées (p1..pN), chacune derrière son routeur-NAT
# (nat1..natN), reliées par un segment « internet » où vit le rendez-vous (rv).
# Chaque joueur tente de percer TOUS les autres → on vise un MESH complet.
#
#   [p1]──[nat1]──┐                              ┌──[nat2]──[p2]
#   192.168.1.2   │      (segment internet)      │   192.168.2.2
#             10.0.0.1 ──── br-net (pont L2) ──── 10.0.0.2   …  10.0.0.N
#                                 │
#                              [rv] 10.0.0.254  ← le rendez-vous, dans SON namespace
#
# Pourquoi le rendez-vous dans un namespace (et pas sur l'hôte) ? Deux raisons :
#   - chaque namespace a son PROPRE espace de ports → aucun conflit avec un
#     rendez-vous déjà lancé sur l'hôte (ton cargo-watch peut rester ouvert) ;
#   - chaque namespace a son PROPRE pare-feu (vide) → le pare-feu de NixOS sur
#     l'hôte ne jette pas les paquets venant des « box ».
#
# Ce que tu DOIS voir, à la fin, dans le RÉSUMÉ :
#   - chaque joueur inscrit au rendez-vous (HELLO → WELCOME) ;
#   - en --cone : chaque joueur ouvre N−1 trous → « MESH COMPLET » (connexions
#     directes deux-à-deux, sans relais) ;
#   - en symétrique (défaut) : 0 trou ouvert → c'est le cas dur qui justifie le
#     relais TURN (chapitre 5).
#
# Deux variantes de NAT (le type de NAT décide si le hole punching réussit) :
#   - défaut : MASQUERADE de Linux = NAT ~SYMÉTRIQUE (un port public différent par
#     destination) → le punch ÉCHOUE. C'est le cas dur (~10 % des box).
#   - --cone : on force un NAT FULL-CONE (port stable + entrée ouverte) → le punch
#     RÉUSSIT. C'est l'autre moitié des box.
#
# Prérequis :  sudo (création de namespaces) + le binaire compilé.
# Lancement :
#   nix-shell --run "cargo build"          # 1) compiler d'abord (hors sudo)
#   sudo ./tools/test-nat.sh               # 2) 3 joueurs, NAT symétrique → échoue
#   sudo ./tools/test-nat.sh 5 --cone      #    5 joueurs, full-cone → MESH complet
#   sudo ./tools/test-nat.sh --clean       # (si besoin) nettoyer un essai interrompu
#
set -euo pipefail

RV_PORT=4000
RV_IP=10.0.0.254          # adresse du rendez-vous sur le segment « internet »
BIN="./target/debug/jeu"  # binaire compilé par cargo
DURATION=14               # secondes que tournent les joueurs
LOGDIR="/tmp/nat-test"    # journaux par joueur (pour le résumé final)

# --- Nettoyage ROBUSTE : on supprime tout ce que le script crée, par MOTIF, sans
#     avoir besoin de connaître N (utile pour --clean après un essai interrompu). --
cleanup() {
  set +e
  # Tuer les process puis supprimer les namespaces p<N>, nat<N> et rv.
  for ns in $(ip netns list 2>/dev/null | awk '{print $1}'); do
    case "$ns" in
      p[0-9]*|nat[0-9]*|rv)
        ip netns pids "$ns" 2>/dev/null | xargs -r kill 2>/dev/null
        ip netns del "$ns" 2>/dev/null ;;
    esac
  done
  # Supprimer le pont « internet » et les pattes publiques br-nat<N> / br-rv.
  ip link del br-net 2>/dev/null
  for v in $(ip -o link show 2>/dev/null | awk -F': ' '{print $2}' | cut -d'@' -f1); do
    case "$v" in br-nat[0-9]*|br-rv) ip link del "$v" 2>/dev/null ;; esac
  done
  set -e
}

# --- Analyse des arguments : un nombre = N joueurs ; --cone / --clean = options. --
N=3
CONE=0
for a in "$@"; do
  case "$a" in
    --clean) cleanup; echo "Nettoyage terminé."; exit 0 ;;
    --cone)  CONE=1 ;;
    *[!0-9]*) echo "Argument inconnu : $a (attendu : un nombre, --cone ou --clean)" >&2; exit 1 ;;
    *) N="$a" ;;
  esac
done

if [ "$N" -lt 2 ] || [ "$N" -gt 250 ]; then
  echo "N doit être entre 2 et 250 (reçu : $N)." >&2
  exit 1
fi

if [ "$(id -u)" -ne 0 ]; then
  echo "Ce script a besoin de root (création de namespaces) : lance-le avec sudo." >&2
  exit 1
fi

if [ ! -x "$BIN" ]; then
  echo "Binaire introuvable ($BIN). Compile d'abord :  nix-shell --run \"cargo build\"" >&2
  exit 1
fi

# Les binaires compilés sous NixOS ont besoin de leurs bibliothèques (Bevy/wgpu se
# chargent au lancement même si nos modes texte ne les utilisent pas). On récupère
# le LD_LIBRARY_PATH de l'environnement nix UNE fois, pour le passer dans les netns.
echo "→ Récupération de l'environnement de bibliothèques (nix-shell)…"
LIBS="$(nix-shell --run 'printf %s "$LD_LIBRARY_PATH"' 2>/dev/null || true)"

cleanup  # au cas où un essai précédent aurait laissé des restes
trap cleanup EXIT

if [ "$CONE" = "1" ]; then
  echo "→ Mode NAT : FULL-CONE (le hole punching doit RÉUSSIR : MESH complet)."
else
  echo "→ Mode NAT : SYMÉTRIQUE (cas réaliste : le punch ÉCHOUE → relais au chap. 5)."
fi
echo "→ Construction de la topologie : rendez-vous + $N NAT + $N joueurs…"

# --- Le segment « internet » : un pont L2 pur (PAS d'IP côté hôte) -------------
ip link add br-net type bridge
ip link set br-net up
# Les frames qui traversent le pont ne doivent PAS repasser par le pare-feu de
# l'hôte (sinon NixOS pourrait les jeter). Best-effort, selon le noyau.
modprobe br_netfilter 2>/dev/null || true
sysctl -wq net.bridge.bridge-nf-call-iptables=0 2>/dev/null || true

# --- Le rendez-vous, dans son propre namespace, branché sur le pont ------------
ip netns add rv
ip netns exec rv ip link set lo up
ip link add br-rv type veth peer name eth0
ip link set br-rv master br-net up
ip link set eth0 netns rv
ip netns exec rv ip addr add "$RV_IP/24" dev eth0
ip netns exec rv ip link set eth0 up

# --- Fonction : monte la maison i (un NAT + son joueur derrière) ----------------
# $1 = index i  →  joueur p$i, NAT nat$i, IP publique 10.0.0.$i, sous-réseau 192.168.$i.0/24
make_house() {
  local i="$1"
  local cli="p$i" nat="nat$i" pubip="10.0.0.$i"
  local gw="192.168.$i.1" host="192.168.$i.2"

  ip netns add "$nat"
  ip netns add "$cli"
  ip netns exec "$nat" ip link set lo up
  ip netns exec "$cli" ip link set lo up

  # Patte PUBLIQUE du NAT, branchée sur le pont « internet ».
  ip link add "br-$nat" type veth peer name pub
  ip link set "br-$nat" master br-net up
  ip link set pub netns "$nat"
  ip netns exec "$nat" ip addr add "$pubip/24" dev pub
  ip netns exec "$nat" ip link set pub up

  # Lien INTERNE NAT <-> joueur.
  ip link add int type veth peer name lan
  ip link set int netns "$nat"
  ip link set lan netns "$cli"
  ip netns exec "$nat" ip addr add "$gw/24" dev int
  ip netns exec "$nat" ip link set int up
  ip netns exec "$cli" ip addr add "$host/24" dev lan
  ip netns exec "$cli" ip link set lan up
  ip netns exec "$cli" ip route add default via "$gw"

  # Le NAT route et MASQUERADE le trafic du joueur vers l'internet (le cœur du NAT).
  ip netns exec "$nat" sysctl -wq net.ipv4.ip_forward=1
  ip netns exec "$nat" sysctl -wq net.ipv4.conf.all.rp_filter=0 || true
  ip netns exec "$nat" iptables -t nat -A POSTROUTING -s "192.168.$i.0/24" -o pub -j MASQUERADE

  if [ "$CONE" = "1" ]; then
    # Mode FULL-CONE : toute entrée UDP sur l'IP publique est renvoyée à l'unique
    # joueur derrière ce NAT (port conservé). L'adresse publique annoncée par le
    # rendez-vous devient joignable par n'importe qui → le hole punching réussit.
    ip netns exec "$nat" iptables -t nat -A PREROUTING -i pub -p udp -j DNAT --to-destination "$host"
  fi
}

for i in $(seq 1 "$N"); do
  make_house "$i"
done

echo "→ Démarrage du rendez-vous (namespace rv, écoute $RV_IP:$RV_PORT)…"
ip netns exec rv env LD_LIBRARY_PATH="$LIBS" "$BIN" rendezvous >/dev/null 2>&1 &
sleep 1

rm -rf "$LOGDIR"; mkdir -p "$LOGDIR"
echo
echo "============ $N JOUEURS se percent mutuellement ($DURATION s) ============"
echo "(journaux par joueur dans $LOGDIR ; résumé du mesh à la fin)"
echo

# Chaque joueur tourne dans SON namespace, derrière SON NAT, et joint le rendez-vous
# par son adresse publique. Tous en parallèle ; chacun journalise dans son fichier.
for i in $(seq 1 "$N"); do
  ip netns exec "p$i" env LD_LIBRARY_PATH="$LIBS" RENDEZVOUS_ADDR="$RV_IP:$RV_PORT" \
    timeout "$DURATION" "$BIN" nat-test "p$i" > "$LOGDIR/p$i.log" 2>&1 &
done
wait

# --- RÉSUMÉ du mesh : par joueur, combien de trous ouverts sur N−1 attendus ? ---
echo
echo "==================== RÉSUMÉ DU MESH ========================"
expected=$((N - 1))
total_open=0
total_expected=$((N * (N - 1)))
for i in $(seq 1 "$N"); do
  log="$LOGDIR/p$i.log"
  # « inscrit au rendez-vous » prouve HELLO→WELCOME ; « trou OUVERT » par pair distinct.
  if grep -q "inscrit au rendez-vous" "$log" 2>/dev/null; then reg="✓"; else reg="✗"; fi
  open=$(grep -c "trou OUVERT" "$log" 2>/dev/null || true)  # grep -c imprime déjà 0 si rien
  total_open=$((total_open + open))
  printf "  p%-3s : inscrit %s | trous ouverts %s/%s\n" "$i" "$reg" "$open" "$expected"
done
echo "------------------------------------------------------------"
printf "Total trous ouverts : %s/%s" "$total_open" "$total_expected"
if [ "$CONE" = "1" ]; then
  if [ "$total_open" -eq "$total_expected" ]; then
    echo "  → ✅ MESH COMPLET (full-cone : tout le monde se voit en direct)."
  else
    echo "  → ⚠ mesh INCOMPLET : certains trous ne se sont pas ouverts (à investiguer)."
  fi
else
  if [ "$total_open" -eq 0 ]; then
    echo "  → ✅ attendu en NAT symétrique : 0 trou direct → c'est le rôle du relais (chap. 5)."
  else
    echo "  → ℹ des trous se sont ouverts malgré le NAT symétrique (selon le noyau)."
  fi
fi
echo "============================================================"
echo
echo "Détail d'un joueur :  cat $LOGDIR/p1.log"
# le trap EXIT appelle cleanup()
