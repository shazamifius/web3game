# COMMENT LANCER & TESTER

> Lancer le cœur headless (rendezvous/sidecar/bot/sim), tester un vrai mauvais réseau (`tc netem`), tester les NAT.

## Tester dans des conditions réseau réelles, sur une seule machine

**Une seule machine suffit** pour confronter le réseau au réel : Linux sait simuler une mauvaise connexion.

- **`tc netem`** (sur l'interface loopback `lo`) ajoute **latence, jitter, perte,
  ré-ordonnancement** à TOUT le trafic localhost. On lance `sim` derrière, et les
  centaines de nœuds se parlent soudain « comme sur Internet ». (`tools/sim-netem.sh`
  l'applique puis le retire proprement.)
- **`tc tbf`** limite le débit (pour simuler les « quelques Ko/s »).
- **`ip netns`** (namespaces réseau) crée de « fausses machines » isolées derrière de
  « faux NAT » sur le même PC — c'est déjà ce que fait `tools/test-nat.sh`.

Donc : **pas besoin de 2 machines** pour confronter au réel. Une seule + netem =
un labo réseau complet, qui transforme les tests « localhost » en vraie preuve.

---

## Comment lancer le cœur (headless)

Le binaire `jeu` n'embarque plus de fenêtre 3D : c'est le **cœur réseau headless**.
La présentation 3D vit dans Unreal, qui se branche au mode `sidecar`. Le projet se
construit dans un environnement reproductible (`nix-shell`) — se placer **dans le
dossier du projet d'abord** :

```fish
cd web3game
```

**Le pont vers Unreal** (le cas normal). On lance le **rendez-vous** (l'annuaire), puis
le **sidecar** auquel Unreal se connecte (socket locale `127.0.0.1:47800`) :

```fish
nix-shell --run "cargo run -- rendezvous"   # terminal 1  (l'annuaire — à lancer en premier)
nix-shell --run "cargo run -- sidecar"      # terminal 2  (le pont ; lancer Unreal ensuite)
```

**Tester le réseau sans Unreal** (clients headless + bancs de mesure) :

```fish
nix-shell --run "cargo run -- bot alice"        # un client headless (le vrai protocole, sans 3D)
nix-shell --run "cargo run -- sim 50 3 15"      # 50 nœuds + 3 attaquants, 15 s, rapport agrégé
nix-shell --run "cargo run -- relay-test 6"     # banc déterministe du relais NAT (deux sens)
nix-shell --run "cargo run -- crowd 200"        # foule dense (couverture de perception)
```

**Caractériser un lien, et prouver la redondance** (les bancs réseau, sans Unreal) :

```fish
nix-shell --run "cargo run -- natcheck"     # sonde de lien : type de NAT (STUN), latence, gigue
nix-shell --run "cargo run -- losscheck"    # nature de la perte : aléatoire vs congestion (rafale à débit croissant)
nix-shell --run "cargo run -- phase1"       # banc DÉTERMINISTE : redondance K vs perte (modèle à graine fixe)
./tools/netem-bench.sh 30                    # banc RÉEL : redondance face à une vraie perte de 30 % (tc netem)
```

> `netem-bench.sh` est notable : il injecte une **vraie** perte réseau (`tc netem`) **sans aucun sudo** et **sans
> toucher la machine**, grâce à un **espace réseau jetable** (`unshare -rn`). Il affiche, pour K = 1…4 copies, la
> perte résiduelle mesurée — à comparer à la prédiction `pᴷ`.

**Mesurer le « vivant » et la pertinence** (bancs déterministes, graine fixe — voir le
[chantier « vivant »](chantier-vivant.md) et le doute D29) :

```fish
nix-shell --run "cargo run -- vivant"       # le mouvement perçu est-il vivant ? (fidélité / fraîcheur / à-coups)
nix-shell --run "cargo run -- aoi"          # pertinence sociale vs proximité (sélection + allocation)
nix-shell --run "cargo run -- aoi-live"     # le même, bout-en-bout sur le vrai transport (message signé)
```

**Les bancs de la voix** (chantier en pause — le code reste rejouable, tout est déterministe) :

```fish
nix-shell --run "cargo run -- voix"         # transport voix : latence bouche-à-oreille, jitter buffer
nix-shell --run "cargo run -- codec"        # codec fait main (courbe débit/qualité) ; « codec p » = perceptuel
nix-shell --run "cargo run -- micro"        # débruitage « étude du micro »
nix-shell --run "cargo run -- separe"       # séparation de sources (chaque bruit isolé, retrait au choix)
```

**Voir le réseau seul, en texte** (sans la 3D, pour observer les paquets) :

```fish
nix-shell --run "cargo run -- net-demo a"
nix-shell --run "cargo run -- net-demo b"
```

**Tester le hole punching à travers de vrais NAT** (sur un seul PC, deux box
simulées en namespaces réseau) :

```fish
nix-shell --run "cargo build"     # compiler d'abord (hors sudo)
sudo ./tools/test-nat.sh          # monte 2 NAT + 2 clients, observe les trous s'ouvrir
sudo ./tools/test-nat.sh --clean  # nettoyer un essai interrompu
```

> Le mode `nat-test` (lancé par le script) rejoue le scénario réseau en texte, car
> les fenêtres 3D ne peuvent pas tourner dans un namespace sans écran.

**Développer avec relance automatique** (le cœur se recompile et redémarre à chaque
sauvegarde — confort de dev, via `cargo-watch`) :

```fish
nix-shell --run "cargo watch -x 'run -- rendezvous'"  # terminal 1 (annuaire)
nix-shell --run "cargo watch -x 'run -- sidecar'"     # terminal 2 (le pont Unreal)
```

> *(Les **contrôles** du joueur — ZQSD, souris, saut — vivent côté Unreal, le client de présentation.
> Le binaire `jeu` n'a plus d'entrée clavier.)*

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Comment lancer & tester** — la **dernière** étape des parcours **⚙️ Le code** et **🧭 Tout comprendre**.

**Continuer le fil :**
- ✓ Vous êtes au bout de ces deux parcours — bravo, vous avez fait le tour. Revenez à la 🗺️ [vitrine](../README.md), ou explorez 🔎 [Juger vite](etat-du-projet.md).

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)

---

