#!/usr/bin/env bash
#
# test-nat.sh — LE VRAI test du hole punching, sur UN seul PC.
#
# On simule « deux maisons derrière deux box Internet » avec des NAMESPACES RÉSEAU
# (ip netns) : deux machines isolées (alice, bob), chacune derrière son routeur-NAT
# (natA, natB), reliées par un segment « internet » où vit le rendez-vous.
#
#   [alice]──[natA]──┐                       ┌──[natB]──[bob]
#   192.168.1.2      │  (segment internet)   │      192.168.2.2
#                10.0.0.1 ── br-net 10.0.0.254 ── 10.0.0.2
#                              (rendez-vous ici, côté hôte)
#
# Ce que tu DOIS voir :
#   1) alice et bob s'inscrivent au rendez-vous (qui lit leur adresse PUBLIQUE,
#      c.-à-d. celle de leur box après traduction NAT) ;
#   2) une SALVE de PUNCH dans les deux sens : les premiers paquets MEURENT dans la
#      box d'en face (trou pas encore ouvert), les suivants PASSENT ;
#   3) « trou OUVERT » / « données reçues » : la connexion DIRECTE est établie,
#      sans que le rendez-vous relaie quoi que ce soit.
#
# Prérequis :  sudo (création de namespaces) + le binaire compilé.
# Lancement :
#   nix-shell --run "cargo build"          # 1) compiler d'abord (hors sudo)
#   sudo ./tools/test-nat.sh               # 2) lancer le test
#   sudo ./tools/test-nat.sh --clean       # (si besoin) nettoyer un essai interrompu
#
set -euo pipefail

RV_PORT=4000
BR_IP=10.0.0.254          # adresse du rendez-vous, vue depuis le « net »
BIN="./target/debug/jeu"  # binaire compilé par cargo

# --- Nettoyage : on supprime tout ce que le script crée (idempotent) ----------
cleanup() {
  set +e
  # tuer les processus lancés (rendez-vous + clients)
  [ -n "${RV_PID:-}" ] && kill "$RV_PID" 2>/dev/null
  for ns in alice bob natA natB; do
    ip netns pids "$ns" 2>/dev/null | xargs -r kill 2>/dev/null
  done
  # supprimer les namespaces (emporte leurs interfaces veth)
  for ns in alice bob natA natB; do ip netns del "$ns" 2>/dev/null; done
  # supprimer le pont côté hôte et ses veth restantes
  ip link del br-net 2>/dev/null
  for v in br-natA br-natB; do ip link del "$v" 2>/dev/null; done
  set -e
}

if [ "${1:-}" = "--clean" ]; then
  cleanup
  echo "Nettoyage terminé."
  exit 0
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
# chargent au lancement même si le mode nat-test ne les utilise pas). On récupère
# le LD_LIBRARY_PATH de l'environnement nix UNE fois, pour le passer dans les netns.
echo "→ Récupération de l'environnement de bibliothèques (nix-shell)…"
LIBS="$(nix-shell --run 'printf %s "$LD_LIBRARY_PATH"' 2>/dev/null || true)"

cleanup  # au cas où un essai précédent aurait laissé des restes
trap cleanup EXIT

echo "→ Construction de la topologie réseau (2 NAT, 2 clients, 1 segment internet)…"

# --- Le segment « internet » : un pont côté hôte, qui porte le rendez-vous ------
ip link add br-net type bridge
ip addr add "$BR_IP/24" dev br-net
ip link set br-net up

# rp_filter (anti-spoofing du noyau) jetterait nos paquets « asymétriques » : off.
sysctl -wq net.ipv4.conf.all.rp_filter=0 || true

# --- Fonction : monte un NAT + son client derrière -----------------------------
# $1 = nom client (alice/bob)  $2 = nom NAT (natA/natB)
# $3 = IP publique du NAT (10.0.0.X)  $4 = sous-réseau interne (192.168.X)
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
}

make_house alice natA 10.0.0.1 1
make_house bob   natB 10.0.0.2 2

echo "→ Démarrage du rendez-vous (côté hôte, écoute $BR_IP:$RV_PORT)…"
"$BIN" rendezvous &
RV_PID=$!
sleep 1

echo
echo "==================== ALICE & BOB (12 s) ===================="
echo "Regarde : la SALVE de PUNCH, puis « trou OUVERT » / « données reçues »."
echo "==========================================================="
echo

# Chaque client tourne dans SON namespace, derrière SON NAT, et joint le rendez-vous
# par son adresse publique. Les deux en parallèle ; on attend 12 s puis on coupe.
ip netns exec alice env LD_LIBRARY_PATH="$LIBS" RENDEZVOUS_ADDR="$BR_IP:$RV_PORT" \
  timeout 12 "$BIN" nat-test alice &
ip netns exec bob   env LD_LIBRARY_PATH="$LIBS" RENDEZVOUS_ADDR="$BR_IP:$RV_PORT" \
  timeout 12 "$BIN" nat-test bob &
wait

echo
echo "==================== FIN — nettoyage ======================="
# le trap EXIT appelle cleanup()
