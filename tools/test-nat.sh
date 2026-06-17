#!/usr/bin/env bash
#
# test-nat.sh — LE VRAI test du hole punching, sur UN seul PC.
#
# On simule « deux maisons derrière deux box Internet » avec des NAMESPACES RÉSEAU
# (ip netns) : deux machines isolées (alice, bob), chacune derrière son routeur-NAT
# (natA, natB), reliées par un segment « internet » où vit le rendez-vous (rv).
#
#   [alice]──[natA]──┐                         ┌──[natB]──[bob]
#   192.168.1.2      │   (segment internet)    │      192.168.2.2
#                10.0.0.1 ── br-net (pont L2) ── 10.0.0.2
#                                  │
#                               [rv] 10.0.0.254  ← le rendez-vous, dans SON namespace
#
# Pourquoi le rendez-vous dans un namespace (et pas sur l'hôte) ? Deux raisons :
#   - chaque namespace a son PROPRE espace de ports → aucun conflit avec un
#     rendez-vous déjà lancé sur l'hôte (ton cargo-watch peut rester ouvert) ;
#   - chaque namespace a son PROPRE pare-feu (vide) → le pare-feu de NixOS sur
#     l'hôte ne jette pas les paquets venant des « box ».
#
# Ce que tu DOIS voir :
#   1) alice et bob reçoivent un identifiant du rendez-vous (HELLO → WELCOME) ;
#   2) une SALVE de PUNCH dans les deux sens : les premiers paquets MEURENT dans la
#      box d'en face (trou pas encore ouvert), les suivants PASSENT ;
#   3) « trou OUVERT » / « données reçues » : connexion DIRECTE, sans relais.
#
# Deux variantes de NAT (le type de NAT décide si le hole punching réussit) :
#   - défaut : MASQUERADE de Linux = NAT ~SYMÉTRIQUE (un port public différent par
#     destination) → le punch ÉCHOUE. C'est le cas dur (~10 % des box), celui qui
#     justifie le relais TURN (chapitre 5). Tu verras la salve tourner sans ouvrir.
#   - --cone : on force un NAT FULL-CONE (port stable + entrée ouverte) → le punch
#     RÉUSSIT. C'est l'autre moitié des box. Tu verras « trou OUVERT ».
#
# Prérequis :  sudo (création de namespaces) + le binaire compilé.
# Lancement :
#   nix-shell --run "cargo build"          # 1) compiler d'abord (hors sudo)
#   sudo ./tools/test-nat.sh               # 2) cas réaliste (NAT symétrique → échoue)
#   sudo ./tools/test-nat.sh --cone        #    cas full-cone → le punch RÉUSSIT
#   sudo ./tools/test-nat.sh --clean       # (si besoin) nettoyer un essai interrompu
#
set -euo pipefail

RV_PORT=4000
RV_IP=10.0.0.254          # adresse du rendez-vous sur le segment « internet »
BIN="./target/debug/jeu"  # binaire compilé par cargo

# --- Nettoyage : on supprime tout ce que le script crée (idempotent) ----------
cleanup() {
  set +e
  for ns in alice bob natA natB rv; do
    ip netns pids "$ns" 2>/dev/null | xargs -r kill 2>/dev/null
  done
  for ns in alice bob natA natB rv; do ip netns del "$ns" 2>/dev/null; done
  ip link del br-net 2>/dev/null
  for v in br-natA br-natB br-rv; do ip link del "$v" 2>/dev/null; done
  set -e
}

CONE=0
for a in "$@"; do
  case "$a" in
    --clean) cleanup; echo "Nettoyage terminé."; exit 0 ;;
    --cone)  CONE=1 ;;
    *) echo "Option inconnue : $a (attendu : --cone ou --clean)" >&2; exit 1 ;;
  esac
done

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
  echo "→ Mode NAT : FULL-CONE (le hole punching doit RÉUSSIR : « trou OUVERT »)."
else
  echo "→ Mode NAT : SYMÉTRIQUE (cas réaliste : le punch ÉCHOUE → relais au chap. 5)."
fi
echo "→ Construction de la topologie réseau (rendez-vous + 2 NAT + 2 clients)…"

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

# --- Fonction : monte un NAT + son client derrière -----------------------------
# $1 = nom client (alice/bob)  $2 = nom NAT (natA/natB)
# $3 = IP publique du NAT (10.0.0.X)  $4 = numéro de sous-réseau interne (1 ou 2)
make_house() {
  local cli="$1" nat="$2" pubip="$3" subnet="$4"
  local gw="192.168.${subnet}.1" host="192.168.${subnet}.2"

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

  # Lien INTERNE NAT <-> client.
  ip link add int type veth peer name lan
  ip link set int netns "$nat"
  ip link set lan netns "$cli"
  ip netns exec "$nat" ip addr add "$gw/24" dev int
  ip netns exec "$nat" ip link set int up
  ip netns exec "$cli" ip addr add "$host/24" dev lan
  ip netns exec "$cli" ip link set lan up
  ip netns exec "$cli" ip route add default via "$gw"

  # Le NAT route et MASQUERADE le trafic du client vers l'internet (le cœur du NAT).
  ip netns exec "$nat" sysctl -wq net.ipv4.ip_forward=1
  ip netns exec "$nat" sysctl -wq net.ipv4.conf.all.rp_filter=0 || true
  ip netns exec "$nat" iptables -t nat -A POSTROUTING -s "192.168.${subnet}.0/24" -o pub -j MASQUERADE

  if [ "$CONE" = "1" ]; then
    # Mode FULL-CONE : toute entrée UDP sur l'IP publique est renvoyée à l'unique
    # client derrière ce NAT (port conservé). L'adresse publique annoncée par le
    # rendez-vous devient donc joignable par n'importe qui → le hole punching réussit.
    ip netns exec "$nat" iptables -t nat -A PREROUTING -i pub -p udp -j DNAT --to-destination "$host"
  fi
}

make_house alice natA 10.0.0.1 1
make_house bob   natB 10.0.0.2 2

echo "→ Démarrage du rendez-vous (namespace rv, écoute $RV_IP:$RV_PORT)…"
ip netns exec rv env LD_LIBRARY_PATH="$LIBS" "$BIN" rendezvous &
sleep 1

echo
echo "==================== ALICE & BOB (12 s) ===================="
echo "Regarde : la SALVE de PUNCH, puis « trou OUVERT » / « données reçues »."
echo "==========================================================="
echo

# Chaque client tourne dans SON namespace, derrière SON NAT, et joint le rendez-vous
# par son adresse publique. Les deux en parallèle ; on attend 12 s puis on coupe.
ip netns exec alice env LD_LIBRARY_PATH="$LIBS" RENDEZVOUS_ADDR="$RV_IP:$RV_PORT" \
  timeout 12 "$BIN" nat-test alice &
ip netns exec bob   env LD_LIBRARY_PATH="$LIBS" RENDEZVOUS_ADDR="$RV_IP:$RV_PORT" \
  timeout 12 "$BIN" nat-test bob &
wait

echo
echo "==================== FIN — nettoyage ======================="
# le trap EXIT appelle cleanup()
