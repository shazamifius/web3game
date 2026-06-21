# web3game — une expérience vers un espace social en pair-à-pair

[![tests](https://github.com/shazamifius/web3game/actions/workflows/tests.yml/badge.svg)](https://github.com/shazamifius/web3game/actions/workflows/tests.yml)

> **Statut : R&D personnelle, solo, très expérimentale. Work in progress.**
> Rien n'est prouvé à grande échelle — voir [« Ce qui est prouvé / pas prouvé / reste »](#ce-qui-est-prouvé-ce-qui-ne-lest-pas-ce-qui-reste) plus bas.

Un petit univers social en **vue première personne** où l'on veut connecter des
centaines de joueurs **sans aucun serveur de jeu central**, en pur pair-à-pair
(P2P). Le but n'est pas un jeu de tir : c'est un espace social, comme VRChat.

> ⚠️ **Important** : ce projet est écrit en **Rust + Bevy**, pas en Unreal
> Engine. On a choisi Rust pour tout contrôler nous-mêmes — chaque octet qui
> part sur le réseau est écrit à la main, aucune « boîte noire ». L'architecture
> réseau décrite plus bas (Own+Shields, BFT, AoI…) est de la **logique**, elle
> resterait la même avec n'importe quel moteur 3D.

---

## La nature de ce projet (à lire en premier)

Ce dépôt est **un projet d'exploration et de R&D**, pas (encore) un produit. La
question qu'on se pose vraiment est : **le « web3 » — au sens décentralisé /
pair-à-pair, identité possédée, sans serveur de jeu central — peut-il *réellement*
fonctionner, ou n'est-ce qu'une utopie ?** On ne le prouve pas par des slogans : on
l'écrit en Rust octet par octet, on l'attaque nous-mêmes, et on garde un inventaire
honnête de ce qui tient et de ce qui ne tient pas (voir
[`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)).

**L'objectif à terme (la boussole, pas une échéance) :** monter **un énorme événement
en P2P sans serveur réunissant ~55 000 personnes** — soit sur **Unreal Engine**, soit
sur le moteur actuel (la *logique* réseau reste la même, voir l'avertissement ci-dessus).
**55 000 = la jauge de la plus grande salle de concert au monde**, un nombre jamais réuni
dans un seul espace de jeu (les MMO « shardent », Fortnite met ~100 joueurs par instance).

On n'a pas besoin que les 55 000 interagissent directement entre eux. La **version
réalisable, et déjà un exploit**, c'est la *présence par niveau de détail (LOD)* :
- pouvoir **se voir** les uns les autres à grande échelle ;
- avoir des **mouvements de foule cohérents** (la marée lointaine en avatars agrégés) ;
- pouvoir **discuter avec au moins ~30 personnes autour de soi** (le voisinage net, à
  débit plein).

Si on tient ça, on aura montré que le P2P à cette échelle n'est pas *que* utopique.

---

## Ce qui est prouvé, ce qui ne l'est pas, ce qui reste

Par honnêteté (c'est une règle du projet : la crédibilité vient de là), l'état réel —
sans rien arrondir. Le détail vit dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md).

**✅ Prouvé (mesuré, reproductible, tests à l'appui — 80 tests, 0 warning) :**
- Cœur P2P : identité = clé cryptographique, états signés, anti-usurpation / anti-rejeu.
- **Hole-punching NAT réel** (full-cone, en _namespaces_ réseau).
- Anti-triche : Sybil-framing échoue, gossip-flood absorbé, positions corroborées.
- **Perception de foule à l'échelle** (headless, banc bus mémoire) : densité restaurée à
  **~87 % à 1000 nœuds**, à débit reçu **plat**, **sans « chef » de cellule**.
- Identité persistante (clé locale, comme une clé SSH).

**⚠️ Pas (encore) prouvé / limites assumées :**
- **Pas « 55 000 prouvé »** : mesuré directement jusqu'à ~1000-2000 nœuds ; au-delà = architecture + extrapolation prudente.
- **« Sans serveur » a un astérisque** : l'amorçage passe encore par un _rendez-vous_ (tout le reste est P2P).
- **Positions en clair** : le chiffrement bout-à-bout n'est pas encore là.
- **NAT symétrique non géré** : le hole-punching marche pour les NAT full-cone ; le symétrique demandera un relais.
- **Perception corroborée : sûre contre l'_omission_, pas encore contre l'_inflation_** (un menteur seul peut sur-déclarer une poignée de présences ; la fermeture est identifiée et mesurée par un test).

**🚧 Le prochain gros chantier :** l'inclusivité — pouvoir jouer même sur un lien réseau pauvre.

---

## Comment lancer le jeu

Le projet se construit dans un environnement reproductible (`nix-shell`).
Toujours se placer **dans le dossier du projet d'abord** :

```fish
cd "/home/shaza/Documents/projet web 3"
```

**Jouer en solo** (une seule fenêtre, sans réseau) :

```fish
nix-shell --run "cargo run"
```

**Jouer à plusieurs sur le même PC** (multijoueur local). On lance d'abord le
**serveur de rendez-vous** (l'annuaire), puis autant de clients qu'on veut :

```fish
nix-shell --run "cargo run -- rendezvous"   # terminal 1  (l'annuaire — à lancer en premier)
nix-shell --run "cargo run -- a"            # terminal 2  (un joueur)
nix-shell --run "cargo run -- b"            # terminal 3  (un autre)
nix-shell --run "cargo run -- play"         # terminal 4  (… et autant qu'on veut)
nix-shell --run "cargo run -- weak"         # un client à FAIBLE upload (passe par un relais)
```

Les identifiants sont attribués par le rendez-vous (plus de rôle codé en dur) ;
chaque client prend un port libre tout seul. `a`, `b`, `play`, `client` font
tous la même chose : lancer un client. `weak` lance un client à **faible upload** :
il n'émet plus son état à tous, mais une seule fois à un **parent** (relais) qui le
recopie aux autres à sa place (chapitre 4.1).

> 💡 **Indice de connexion** : la **couleur de la salle** est donnée par le
> serveur de rendez-vous. Deux fenêtres de **même couleur** = connectées au même
> serveur. Une fenêtre d'une **autre couleur** (sa couleur aléatoire de départ)
> = pas connectée. Pratique pour vérifier d'un coup d'œil, sans chercher un
> avatar dans la salle.

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

> Le mode `nat-test` (lancé par le script) rejoue le scénario réseau du jeu en
> texte, car les fenêtres 3D ne peuvent pas tourner dans un namespace sans écran.

**Développer avec relance automatique** (le jeu se recompile et redémarre à
chaque sauvegarde — confort de dev, via `cargo-watch`) :

```fish
nix-shell --run "cargo watch -x 'run -- rendezvous'"  # terminal 1 (annuaire)
nix-shell --run "cargo watch -x 'run -- a'"           # terminal 2
nix-shell --run "cargo watch -x 'run -- b'"           # terminal 3
```

> La fenêtre se ferme/rouvre à chaque reload (la position est donc remise à
> zéro, et un clic gauche recapture la souris). C'est de la relance auto, pas du
> hot-patch : largement suffisant pour régler le netcode.

### Contrôles
| Touche        | Action                          |
|---------------|---------------------------------|
| ZQSD          | se déplacer                     |
| Souris        | regarder autour                 |
| Échap         | libérer la souris               |
| Clic gauche   | recapturer la souris            |

---

## Organisation du code (`src/`)

Principe : **un fichier = une responsabilité** (plein de petits fichiers plutôt
qu'un gros).

```
src/
├── main.rs              point d'entrée, aiguillage des modes (solo / a / b / net-demo)
├── world.rs             la salle (sol, murs, plafond néon, lumière)
├── player.rs            le personnage, la caméra 1re personne, les contrôles
└── net/                 LE RÉSEAU, fait main
    ├── mod.rs           assemble le module et expose l'API publique
    ├── message.rs       le format d'un paquet (PlayerState, encode/decode + signé)
    ├── control.rs       les messages d'annuaire (HELLO / WELCOME)
    ├── crypto.rs        Ed25519 + PeerId (identité = clé) + preuve de travail — boîte noire (5/6.1/6.2)
    ├── anticheat.rs     le « Shield local » : règles de plausibilité physique (chap. 6.3+)
    ├── accuse.rs        accusations signées + quorum : réputation partagée (chap. 6.7)
    ├── aoi.rs           Area of Interest (water-filling : qui reçoit quel débit)
    ├── punch.rs         hole punching (percer les NAT pour une connexion directe)
    ├── orb.rs           l'orbe partagée : objet à maître unique + migration d'hôte
    ├── transport.rs     la prise UDP brute (Socket) — la « connexion »
    ├── skin.rs          la couleur de skin aléatoire
    ├── demo.rs          le mode texte net-demo (observer les paquets)
    ├── attack.rs        le PROGRAMME ATTAQUANT (cargo run -- attack …) — chap. 5 & 6
    ├── bot.rs           le CLIENT HEADLESS (cargo run -- bot …) + brique `Bot` réutilisable (6.0/6.8)
    ├── sim.rs           la SIMULATION MASSIVE (cargo run -- sim N M T) : N nœuds + M attaquants (chap. 6.8)
    ├── natdemo.rs       le mode texte nat-test (hole punching sans 3D, pour netns)
    ├── link.rs          NetLink, la ressource qui relie le réseau au jeu
    └── netcode/         LE RATTRAPAGE DE LATENCE
        ├── mod.rs       assemble le sous-module
        ├── state.rs     instantanés, file par joueur, RÉGLAGES (constantes)
        ├── send.rs      émettre notre état (débit limité + vraie vitesse)
        ├── receive.rs   ranger les paquets reçus (et créer l'avatar)
        ├── nameplates.rs  étiquettes de rôle (texte) au-dessus des avatars
        ├── interpolate.rs   animer chaque image (horloge adaptative + ressort)
        ├── predict.rs   calculer l'état voulu (interpolation ou prédiction)
        └── smooth.rs    le ressort amorti (SmoothDamp) + helpers d'angles
```

**En-tête commun à TOUS les paquets** : octet 0 = `type` (KIND), octet 1 =
`version du protocole` (`PROTO_VERSION`). Un récepteur d'une autre version rejette
le paquet **et le signale** au lieu de le lire de travers — fini le « bonhomme
invisible » de deux binaires désynchronisés. Voir `net/wire.rs`.

Depuis le chapitre 5, tout paquet d'état est **signé** : on émet le corps suivi
d'un **sceau Ed25519 de 64 octets**. Depuis le **chapitre 6.1**, l'identité (`id`)
n'est plus un numéro `u8` mais la **clé publique** de l'émetteur (32 octets),
**portée dans le paquet** : le récepteur vérifie le sceau CONTRE cette clé
embarquée — l'identité s'auto-prouve, **sans aucun annuaire de confiance**. Le
rendez-vous ne peut donc plus mentir sur « qui est qui ».

Un paquet de joueur fait **118 octets** : `type` (1) + `version` (1) + `id`
(**clé, 32**) + `x,y,z` + `vx,vy,vz` + `yaw,pitch` + `r,g,b` (11 × 4 octets) +
`parent` (**clé, 32** ; zéros = autonome) + `seq` (8, anti-rejeu). Signé = 118 +
64 = **182 octets**. Voir `net/message.rs`. Un paquet d'orbe **signé** fait **136
octets** (corps 72 + sceau 64). Le corps de **72 octets** : `type` + `version` +
`owner` (**clé, 32**) + `version d'orbe` + position, vitesse, couleur. Voir
`net/orb.rs`. (`PeerId` = la clé, dans `net/crypto.rs` ; affiché en hexa court.)

**Convention « fichier inactif »** : un fichier qui n'est plus utilisé est
préfixé d'un `_` (ex. `_demo.rs`) et sa ligne `mod` est retirée. Il remonte en
tête de liste et signale d'un coup d'œil qu'il ne sert plus — sans le ranger
dans un sous-dossier. (Le compilateur Rust confirme l'inverse : si un fichier
*est* branché sans warning `unused`, c'est qu'il sert.)

---

## Feuille de route (le cours)

> 📋 **Le plan COMPLET et détaillé est dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)** :
> l'inventaire honnête des 22 doutes/risques (D1→D22), et le programme chapitre par
> chapitre (7→14) pour les fermer, avec la méthode de test « vraie mauvaise connexion
> sur une seule machine » (`tc netem`). Le README ci-dessous reste le résumé.

On avance **en codant pour de vrai**, chapitre par chapitre. On part du plus
simple (deux PC qui se parlent) vers le plus dur (des centaines de joueurs,
anti-triche).

> ### 📍 Où on en est (journal de bord — chapitre 6 « refonte BÉTON »)
> Objectif : **55 000 joueurs en P2P pur, un maximum d'attaquants, et que ça tienne.**
> - **▶ ÉTAT COURANT (20 juin) — D26 couche 1 : le RÉSUMÉ DE CELLULE est AUTHENTIFIÉ.** C'était le seul
>   paquet anonyme : n'importe qui forgeait un résumé pour n'importe quelle cellule, et un `ts = u64::MAX`
>   épinglait le mensonge à vie. Désormais l'hôte **embarque sa clé et SIGNE** son résumé (comme un état
>   joueur), la fraîcheur est un **compteur `seq` par hôte** (plus l'horloge forgeable), et on n'accepte un
>   résumé que si **l'émetteur est bien l'hôte attendu de la cellule** PUIS si le **sceau** tient. → forge
>   anonyme, effacement de région (`count=0`) et épinglage tués (tests red-team + sim `crowd 200` : perception
>   intacte, **max 200 occupants via 1 flux**). **73 tests, 0 warning.** *(Reste la couche 2 = corroboration :
>   un hôte LÉGITIME peut encore mentir sur SA cellule — voir [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md) §0.)*
> - **10.1 : IDENTITÉ PERSISTANTE (tu es le même entre sessions).** Ta clé est
>   minée une fois puis sauvée dans `~/.web3game/<profil>.key` (perms 600, comme une clé SSH) et rechargée au
>   lancement → fini l'identité neuve à chaque fois (ferme **D14**). `NetLink::new` (simu/bots) reste éphémère
>   et intact ; le vrai jeu utilise `new_persistent` (profil = le mode → `a.key` ≠ `b.key`, deux fenêtres
>   distinctes ET stables). Prouvé par test (2e lancement = même identité). **68 tests, 0 warning.** *(Premier
>   pas du plan d'attaque « événement 55K » — voir FEUILLE_DE_ROUTE §0.)*
> - **▶ chapitre 8.3d : la foule dense passe à l'échelle.** Chapitres 0→7 faits,
>   chapitre 9 (confiance dure : Sybil/éclipse/framing) tenu, chapitre 8 presque bouclé. **Dernière étape
>   8.3d :** un VRAI bug de conception trouvé et fermé — les résumés de cellule n'avaient pas d'ordre de
>   FRAÎCHEUR, donc de vieilles copies partielles écrasaient les fraîches (la perception EMPIRAIT avec le
>   temps). Fix : un horodatage `ts` par résumé, l'ingestion ne garde que le plus frais (anti-rejeu jumeau de
>   celui des états). **Prouvé :** la perception CONVERGE vers N (`crowd 500` → 477/500 occupants via O(cellules)
>   flux) à débit ↓ **PLAT (~45 Ko/s)** quand N grandit. **66 tests, 0 warning.** Détail + doutes restants
>   (convergence à 2000, fraîcheur à chiffrer) dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md) §0.
> - **Fait :** chapitres 0→5 ; **6.0** (bot headless + 4 attaques) ; **6.1** (identité
>   = clé) ; **6.2** (anti-Sybil PoW) ; **6.3** (anti-téléport) ; **6.4** (contact orbe) ;
>   **6.5** (DoS borné) ; **6.6** (voisinage borné, O(N·K)) ; **6.7** (réputation
>   partagée : accusations signées + quorum) ; **6.8** (simulation massive : 50 et
>   300 bots + attaquants → ça tient, voisins plafonnés à 32, orbe intègre). Build
>   vert, 35 tests, 0 warning. **CHAPITRE 6 TERMINÉ — les 10 trous fermés/bornés.**
> - **Décisions de direction prises** (détail dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)) :
>   ① on chiffre tout ; ② preuve de travail anti-Sybil réglable ; ③ ordre normal
>   7→8→9→10 ; ④ identité persistante (clé sauvée dans un fichier).
> - **Chapitre 7 en cours (confrontation au réel). 7.1 + 7.2 + 7.3 ✓** : `tools/sim-netem.sh`
>   applique une *vraie* mauvaise connexion (`tc netem` : latence/jitter/perte/ré-ordo sur
>   `lo`, 3 profils `bon|moyen|mauvais`), lance la simu, et retire toujours le netem à la
>   fin. Mesuré (`sim 50 5 30`) : la **sécurité tient partout** (orbe 0/50 volée, attaques
>   neutralisées même à 250 ms + ré-ordonnancement). **7.3** durcit l'anti-rejeu en
>   **fenêtre glissante** (style IPsec/DTLS : tolère le ré-ordo sans rouvrir le rejeu).
>   Honnêteté : on pensait que l'anti-rejeu strict expliquait l'effondrement du débit
>   honnête sous `mauvais` (−70 %) — **c'était faux** (le fix ne récupère que +15 %). La
>   vraie cause était le `limit 1000` par défaut de `tc netem` : **7.3b** le relève à
>   100 000 (file non bloquante) et **prouve** que le débit honnête sous `mauvais` remonte
>   à ~21,3k/s (vs `bon` ~23,3k/s → **−9 % seulement**, ≈ la perte de 5 %). **Le protocole
>   tient sous réseau réel** (250 ms + jitter + perte + ré-ordo) ; le −70 % était un
>   artefact du harnais, pas le jeu. **7.4** chiffre le **coût réel par nœud**
>   (nouveau [`src/net/probe.rs`](src/net/probe.rs)) : bande passante (compteurs d'octets
>   dans la prise) et CPU du thread (`/proc/thread-self/stat`) **réels, par nœud** ; la RAM
>   est donnée **globale** (crête du process) car un seul tas est partagé — on **refuse** une
>   RAM/nœud factice. **7.4b** corrige une erreur d'honnêteté : le 1er chiffre (↑89 Ko/s) était
>   mesuré sur le **mauvais chemin** — le bot émettait naïvement à tous, pas via l'AoI
>   water-filling du **vrai** client. Le bot appelle maintenant les mêmes fonctions
>   qu'[`aoi.rs`](src/net/aoi.rs) → re-mesuré à saturation : **↑ ~34 Ko/s, ↓ ~31 Ko/s, CPU
>   ~0,7 %/cœur, 38 Mo** (~0,27 Mbit/s ↑/joueur, très tenable). **MAIS** 7.4b révèle le vrai mur
>   (**doute D22**) : en **foule dense**, on est **aveugle au-delà de 32 voisins** (plafond dur
>   du rendez-vous) — le water-filling ne peut rien car il n'apprend jamais le 33e. C'est une
>   question d'**architecture** (AoI par vision + découverte décentralisée), pas de réglage →
>   ça mérite un **chapitre dédié**. *Ferme D19, ouvre D22.* **7.5** généralise enfin
>   [`tools/test-nat.sh`](tools/test-nat.sh) au **multi-joueurs** (N maisons `p1..pN` derrière
>   N NAT distincts + résumé du mesh) et a révélé+corrigé un **bug d'instrumentation** ([`natdemo.rs`](src/net/natdemo.rs) :
>   le trou s'ouvrait en silence si les données arrivaient avant le punch). **Preuve NAT réelle
>   FAITE** (sous `sudo`, namespaces + vrais NAT, ~16 s) : `test-nat.sh 3 --cone` → **6/6 MESH
>   COMPLET** ; sans `--cone` (symétrique) → **0/6** (le punch échoue → relais ch.5). Le hole
>   punching multi-joueurs tient donc à travers de vrais NAT, pas juste sur localhost.
>   **Chapitre 7 bouclé.**
>   **PLAN DU CHAPITRE 8 (densité, D22) ÉCRIT — prochaine action = CODER 8.0.** Le mur D22 a
>   maintenant son chapitre dédié, écrit AVANT de coder (règle d'or). Diagnostic net : le plafond
>   est au rendez-vous ([`rendezvous.rs`](src/net/rendezvous.rs) : `keep_nearest(…, 32)`) et le
>   client **écrase** `link.peers` avec ce roster ([`receive.rs`](src/net/netcode/receive.rs)) →
>   **le 33e voisin n'est jamais appris**. La réponse est **architecturale, pas un réglage** (monter
>   le plafond rouvrirait l'O(N²) et noierait le faible — D3) : **séparer le FOCUS** (lien netcode
>   plein, borné ~32) **de la CONSCIENCE** (perception LOD de la foule, NON plafonnée — ton « AoI par
>   vision ») ; **découverte par gossip** (le rendez-vous démoté à un simple amorçage) ; **cellules
>   spatiales + hôte agrégateur** pour tenir l'invariant clé : *réception = O(focus + cellules),
>   indépendante de la taille N de la foule*. L'ancien « Chapitre 8 — Inclusivité » (D3/D4/D5) est
>   **fusionné** dans ce chapitre (même problème vu des deux bouts). **8.0 ✓ FAIT — le mur est
>   chiffré.** Nouveau mode `cargo run -- crowd <N>` : une foule dense de N nœuds au même endroit,
>   qui mesure la **couverture de perception** (perçus ÷ à portée) et les tiers *focus / conscience*.
>   **Prouvé :** `crowd 200` → couverture **16 %** (FOCUS 32 + CONSCIENCE 0), **aveugle à 167** ;
>   débit de référence **↓ 24,8 Ko/s** (le nombre qui devra rester PLAT quand la couverture montera) ;
>   l'essaim tient (orbe 0/200). **8.1 ✓ FAIT — le mur tombe.** Découverte par GOSSIP (nouveau
>   `src/net/gossip.rs`, paquet « cartes de visite ») : le WELCOME **amorce** `link.peers` au lieu
>   de l'écraser, et les pairs s'échangent à bas débit un sous-ensemble divers de pairs connus → la
>   table s'enrichit **sans plafond**. **Mesuré : couverture 16 % → 98 %** à `crowd 200`, et l'INVARIANT
>   est prouvé — le débit ↓ **ne grandit pas** de 200 à 500 nœuds (~plat, CPU ~0,7 %, orbe 0 volée).
>   **8.1b ✓ FAIT — la porte DoS du gossip est fermée (D23).** On avait échangé le plafond de 32
>   contre une porte d'entrée DoS (cartes apprises sans preuve de travail ni corroboration). Quatre
>   défenses en profondeur : PoW exigée sur chaque carte, l'adresse d'un pair connu n'est jamais
>   écrasée par ouï-dire, abandon du perçage spéculatif après ~10 s (avant : à vie → flot réfléchi
>   infini), rate-limit d'apprentissage par source. **Prouvé par un VRAI attaquant** `attack
>   gossip-flood` : **0 perçage réfléchi** vers la cible, tables non polluées ; découverte honnête
>   intacte (`crowd 60` → couverture 100 %), essaim tenu avec l'attaquant actif. **47 tests, 0 warning.**
>   *Dette restante (registre dans la feuille de route) : le vrai jeu 3D plafonne la foule visible à
>   64 (**D24**) ; la métrique compte les pairs connus, pas entendus.* **Prochaine
>   action = 8.2** (AoI à deux tiers : focus net / conscience LOD). **Tout le plan
>   post-chapitre-6 (chapitres
>   7→14 + les 22 doutes D1→D22) est dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)** — la liste
>   ci-dessous n'est qu'un aperçu.
> - **Comment je vérifie (sans GPU, en terminaux) :** `cargo test` + le bot
>   headless. Scénario type : un terminal `cargo run -- rendezvous`, deux
>   `cargo run -- bot alice` / `bot bob`, puis `cargo run -- attack <nom>`. Les
>   bots impriment un « ledger » (acceptés / rejetés / relayés / muets / orbe) qui
>   rend chaque attaque visible — rouge (réussie) aujourd'hui, verte une fois fermée.
> - **Les 10 trous de l'audit** (cible de fermeture entre parenthèses) : 1 plafond
>   255 *(6.1 ✓)*, 2 WELCOME tronqué *(6.6 ✓)*, 3 maillage O(N²) *(6.6 ✓)*, 4 collision
>   d'id *(6.1 ✓)*, 5 rendez-vous menteur *(6.1 ✓)*, 6 Sybil gratuit *(6.2 ✓)*, 7
>   téléport/speed-hack *(6.3 ✓)*, 8 vol d'orbe lent *(6.4 ✓)*, 9 DoS spoofing/mémoire
>   *(6.5 ✓)*, 10 amplification relais *(6.5 ✓)*.

- [x] **Chapitre 0 — Le bac à sable 3D**
      Salle néon, personnage articulé, vue première personne. *(fait)*
- [x] **Chapitre 1 — Transport brut**
      UDP fait main : encoder une position en octets, l'envoyer, la recevoir.
      Deux fenêtres se voient. Skin de couleur aléatoire. Orientation
      (corps + tête) transmise. *(fait)*
- [x] **Chapitre 2 — Netcode : fluidité + prédiction**
      Envoi à 20 paquets/s (au lieu de 60). Chaque position reçue est rangée
      dans une file d'**instantanés** horodatés ; l'avatar est dessiné 100 ms
      dans le passé (**retard d'interpolation**) en glissant entre les deux
      instantanés qui l'entourent. Quand la file est épuisée (paquet en retard
      ou perdu), on **prédit** la suite par extrapolation de la vitesse
      (*dead reckoning*) au lieu de figer l'avatar, puis on **réconcilie** en
      douceur quand le vrai paquet arrive. **Horloge de lecture adaptative**
      (dilatation temporelle « à la Discord ») : chaque avatar a sa propre
      horloge qui avance plus vite quand on est en retard / plus lentement
      quand on risque la disette (±10 % max), pour rattraper en marchant le
      vrai chemin au lieu de téléporter. La **vraie vitesse** de l'émetteur est
      transmise dans le paquet (45 octets) → prédiction non bruitée. La
      réconciliation se fait par **ressort amorti** (SmoothDamp) : rattrape vite,
      sans dépasser. Réglages dans `net.rs` : `INTERP_DELAY`,
      `MAX_EXTRAPOLATION`, `SMOOTH_TIME`, `CATCHUP_GAIN`, `MAX_WARP`. Se teste
      avec `tc netem`. *(fait)*

      > Note de conception : prédiction faite **à la main** (vitesse depuis
      > l'historique), pas par réseau de neurones — la physique de l'inertie
      > humaine suffit sur 100 ms, c'est déterministe, lisible et gratuit en CPU.
      > L'IA serait pertinente pour prédire la *pose du corps*, pas la position.
- [x] **Chapitre 3 — Topologie & passage à l'échelle** *(fait)*
      - [x] **N pairs + rendez-vous** : un serveur d'annuaire (`net/rendezvous.rs`)
        présente les joueurs ; chacun s'inscrit (HELLO), reçoit la liste (WELCOME)
        et envoie son état directement à tous les pairs. Plus de « 2 pairs codés
        en dur » → autant de joueurs qu'on veut.
      - [x] **NAT & hole punching** (`net/punch.rs`) : la box (NAT) jette tout
        paquet entrant non sollicité. Mais ENVOYER ouvre, dans notre box, un
        « trou de retour » sur le port utilisé. Les deux pairs s'envoient donc une
        **salve de PUNCH** l'un vers l'autre : les premiers paquets meurent (trou
        d'en face pas encore ouvert), les suivants passent → connexion **directe**,
        sans relais. Le rendez-vous ne fait que présenter les adresses publiques.
        On répète le PUNCH (toutes les 0,25 s) jusqu'à confirmation : la répétition
        absorbe le décalage de timing. Se teste sur un seul PC avec
        `tools/test-nat.sh` (deux NAT simulés en namespaces réseau). Repli relais
        (TURN/supernœud) pour les NAT symétriques : prévu au chapitre 5.
      - [x] **Interest management par allocation de budget** (`net/aoi.rs`) : on
        ne supprime jamais personne par règle ; on **répartit un budget
        d'émission** (`SEND_BUDGET_HZ`) entre tous les pairs selon leur
        **pertinence** (`relevance_weight` : distance douce + un socle, jamais 0).
        La répartition se fait par **water-filling** (`allocate_rates`) : chaque
        pair reçoit un débit ∝ pertinence, plafonné à `SEND_HZ`, somme ≤ budget ;
        le surplus des pairs satisfaits est redonné aux autres. Conséquences :
        budget non saturé (2 joueurs) → **plein débit pour tous, peu importe la
        distance** ; saturé (foule) → ça se dégrade en douceur, jamais zéro. Le
        wifi entrera plus tard par `SEND_BUDGET_HZ` (bon lien = grand budget). Le
        rendez-vous ne fait plus qu'une borne grossière de candidats. Tests
        unitaires du water-filling dans `aoi.rs`.
- [~] **Chapitre 4 — Autorité & migration d'hôte** *(en cours)*
      - [x] **Orbe partagée** (`net/orb.rs`) : le premier **objet du monde** qui
        n'appartient à personne par naissance. Le **dernier joueur à la toucher**
        en devient le **maître** (l'autorité) : lui seul simule sa physique
        (rebonds sur les 6 parois) et la **diffuse** aux pairs (20 Hz) ; les autres
        recopient. La propriété **saute de main en main** à chaque contact — une
        mini-migration d'autorité déclenchée à la main. Conflits réglés par un
        couple `(version, id)` : version plus haute gagne, à égalité le plus petit
        id l'emporte (`supersedes`) — départage **déterministe**, sans serveur.
      - [x] **Migration d'hôte** (sur l'orbe) : si le maître ne se manifeste plus
        pendant `MASTER_TIMEOUT` (0,5 s ≈ 10 battements manqués), on le présume
        parti et on **élit** son remplaçant de façon **déterministe** (le plus petit
        id, l'ancien maître exclu) : chacun calcule le même gagnant **sans voter**.
        Le nouveau maître **reprend** l'orbe à son dernier état connu et incrémente
        la version → un éventuel **split-brain** (l'ancien maître réapparaît) se
        résout tout seul (sa version est plus basse → il abdique via `supersedes`).
        Se voit en tuant la fenêtre du maître pendant que l'orbe vole.
      - [x] **Relais / « parent »** pour les connexions faibles *(chapitre 4.1)* :
        un client lancé en mode `weak` (faible débit montant) n'émet plus son état
        à tous ses pairs — il l'envoie **une seule fois** à un **parent** (le plus
        petit id joignable), dans un paquet `KIND_RELAY`. Le parent le **recopie**
        en `KIND_STATE` à ses propres voisins. L'**id dans le paquet reste celui de
        l'auteur** (pas du relayeur) → ses voisins le rangent sous SON avatar : le
        parent n'est qu'un **porteur d'octets**. Économie : **1 envoi au lieu de N**
        (le *download* reste direct — on continue de recevoir tout le monde, ce qui
        colle à une vraie 4G : upload faible, download correct). Un parent par recoin
        (≈ 10 joueurs) → des milliers de relais, aucun goulot. ⚠️ Le relais
        **recopie** (transport), il n'**arbitre** pas (autorité) : deux rôles
        distincts (cf. *Own ≠ Relais* plus bas). Sécurité (chap. 5) : le faible
        **signera** son état pour que le parent ne puisse pas le falsifier.
      - [ ] **Shields** (témoins) : vérification **périodique** d'un Own d'objet/zone
        pour empêcher un maître local de tricher (passerelle vers le chapitre 5).

      > **Aide visuelle (debug)** : ces rôles sont invisibles à l'œil, alors une
      > **étiquette texte flotte au-dessus de chaque avatar distant**
      > (`net/netcode/nameplates.rs`, rendu en overlay 2D projeté à l'écran) :
      > `Joueur N — OWN BALLE / TUTEUR / SOUS TUTELLE`. Le rôle voyage dans l'octet
      > `parent` du paquet d'état, donc tout le monde voit qui relaie qui. (On
      > n'étiquette pas son propre corps : on lit les rôles sur les autres, idéalement
      > depuis une 3e fenêtre.)
- [~] **Chapitre 5 — Confiance & anti-triche** *(en cours)*
      - [x] **5.1 — Identité signée (enveloppe scellée).** Chaque session a une paire
        de clés **Ed25519** ; la clé publique est l'identité, diffusée par le
        rendez-vous (dans HELLO/WELCOME). Tout état est **signé** (corps 48 o +
        sceau 64 o = 112 o) et **vérifié** à la réception. Ferme l'**usurpation
        d'identité** (on ne peut plus se faire passer pour un autre `id`) et la
        **falsification par un relais** (le parent porte l'enveloppe scellée
        verbatim, il ne peut plus modifier l'état de son protégé). La crypto vit
        dans un **seul** fichier (`net/crypto.rs`) : la seule « boîte noire »
        assumée du projet — on ne code JAMAIS sa propre crypto.
      - [x] **5.2 — Anti-rejeu.** Compteur `seq` monotone dans le corps signé ; on
        refuse tout paquet de `seq` ≤ au dernier vu d'un pair → un vieux paquet
        rejoué ne peut plus rembobiner un joueur.
      - [x] **5.3 — Orbe signée + Shield local.** Le maître **signe** l'orbe (corps
        41 o + sceau 64 o = 105 o) → plus de vol à distance. Et on **borne le saut de
        version** (≤ 16) : un bond vers `65535` pour verrouiller l'orbe à vie est
        refusé *et* compté comme faute. Chaque nœud est ainsi le « Shield » de ce
        qu'il observe. *(MVP : le quorum BFT inter-nœuds — accusations signées
        partagées — reste l'approfondissement.)*
      - [x] **5.4 — Réputation locale.** Compteur de fautes par pair (orbe trichée,
        état signé impossible) → **mise en sourdine** au-delà d'un seuil. Règle clé
        anti-*framing* : on n'accuse JAMAIS sur une signature invalide (non
        attribuable), seulement sur un paquet **valablement signé mais trichant**.
        *(MVP : l'agrégation décentralisée des réputations — EigenTrust — reste à venir.)*
      - [x] **5.5 — Rate-limit & plafond.** « Seau à jetons » par adresse (jette
        l'excès d'une inondation) + plafond d'avatars distants (anti-DoS). *(MVP :
        coût d'entrée anti-Sybil et relais TURN pour NAT symétrique restent à venir.)*
      - [x] **Harnais adversarial** : `cargo run -- attack <forge|replay|flood|orb-steal|orb-freeze>`
        — un VRAI programme attaquant, sur de vraies sockets, qui prouve la robustesse.
        + 22 tests unitaires adversariaux (sceau forgé, altéré, rejeu, saut de version…).
- [~] **Chapitre 6 — Refonte BÉTON : durcissement intégral** *(en cours)*
      On reprend CHAQUE script et on ferme le fossé entre « 5 attaques connues
      neutralisées » et l'objectif réel : **55 000 joueurs en P2P pur, avec un
      maximum d'attaquants de tout genre, et que ça tienne**. Honnêteté assumée :
      le P2P sans serveur central à cette échelle, face à des adversaires
      byzantins, est à la frontière de la recherche — on ne promet pas l'inviolable
      absolu (ça n'existe pas). On vise : **chaque attaque devient soit impossible,
      soit chère, soit attribuable et bannie.** On avance fondation d'abord.
      - [x] **6.0 — Mode bot headless + harnais d'attaque « rouge ».** *(fait)*
        Un client `cargo run -- bot <nom>` fait tourner le VRAI protocole **sans 3D**
        (mêmes décisions de confiance que le jeu : sceau, anti-rejeu, réputation,
        rate-limit, autorité d'orbe) et imprime un « ledger » de ce qu'il accepte /
        refuse / relaie. On peut donc lancer « rendez-vous + N bots + 1 attaquant »
        **en terminaux, sans GPU**. On a ajouté à `attack.rs` les attaques qui
        RÉUSSISSENT encore (`teleport`, `sybil`, `orb-creep`, `amplify`) : autant de
        preuves « rouges » à passer au vert aux étapes suivantes. **Vérifié en
        headless** : orb-creep vole l'orbe (v30, 0 faute), amplify fait rediffuser
        la victime, teleport est accepté sans borne. *(C'est l'embryon de la
        simulation 55K du 6.8.)*
      - [x] **6.1 — Identité auto-certifiante (le keystone « web3 »).** *(fait)*
        L'identité d'un joueur EST désormais sa **clé publique** (`PeerId` = 32
        octets), portée dans chaque paquet signé ; le récepteur vérifie le sceau
        CONTRE cette clé embarquée (`sig_ok` ne consulte plus aucun annuaire). Le
        type `u8` a disparu de tout le protocole. **Ce que ça ferme :** le mur des
        255 joueurs (trou n°1), les collisions d'id (n°4), et surtout **le
        rendez-vous ne peut plus mentir** sur « telle clé = tel joueur » (trou n°5,
        le plus grave : avant, toute la signature reposait sur l'honnêteté du
        serveur). Le rendez-vous est rétrogradé en simple carnet d'adresses.
        **Vérifié** (headless + 23 tests) : usurpation rejetée (clé embarquée ≠
        signataire), chemin honnête intact. Tailles : état signé 56→**182 o**,
        orbe signée 105→**136 o**.
      - [x] **6.2 — Coût d'entrée anti-Sybil (preuve de travail).** *(fait)* Une
        identité n'est VALIDE que si sa clé publique a `POW_BITS` (= 16) bits de tête
        à zéro. En trouver une exige de « miner » ~2^16 paires de clés (`generate_pow`,
        ~0,9 s mesuré) ; vérifier est gratuit (`PeerId::has_pow`). Pairs ET rendez-vous
        **ignorent** une identité non minée. **Ce que ça ferme :** trou n°6 — un banni
        ne se reconnecte plus gratuitement, il doit RE-MINER à chaque fois → la
        réputation/sourdine reprend du sens. **Vérifié headless** : les identités
        minées commencent visiblement par `0000…` ; le chemin honnête tourne ; l'attaque
        `sybil` doit re-payer la preuve. *(MVP tunable : 16 bits ≈ quelques dixièmes de
        s ; on peut monter la difficulté. Un attaquant à GPU mine plus vite — la vraie
        défense forte combine PoW + réputation partagée du 6.7.)*
      - [x] **6.3 — Validation de mouvement (anti-téléport / speed-hack).** *(fait)*
        Nouveau module `net/anticheat.rs` (le « Shield local ») : à la réception, on
        compare la distance parcourue depuis le dernier état accepté d'un joueur au
        temps écoulé ; au-delà de `MAX_SPEED` (30 m/s, généreux), c'est un téléport →
        état **refusé** + **faute** (attribuable : il est validement signé). On ne
        recopie même pas un tricheur relayé (pas d'amplification de triche). **Ce que
        ça ferme :** trou n°7. La signature prouve QUI ; ceci prouve que le mouvement
        est PLAUSIBLE. **Vérifié headless** : le téléporteur prend « 🛡 Faute…
        téléport » à chaque saut → SOURDINE ; la marche normale passe. +4 tests.
      - [x] **6.4 — Orbe : preuve de contact.** *(fait)* On ferme le vol d'orbe par
        incréments +1 (`orb-creep`) : pour devenir maître, `apply_incoming` exige
        désormais que le revendiqueur ait été **près de l'orbe** (≤ `CONTACT_DIST` =
        3 m) au moment où il la réclame — sinon `NoContact` → état refusé + faute. Un
        maître INCONNU (qu'on ne voit pas) n'est toléré que lors d'une **migration**
        (l'ancien maître s'est tu > `MASTER_TIMEOUT`). **Ce que ça ferme :** trou n°8.
        **Vérifié headless** : le creeper prend « 🛡 … orbe revendiquée sans contact »
        → SOURDINE, l'orbe reste sans maître. +1 test. *(Limite assumée : la voie de
        migration reste plus permissive — durcissement au 6.7 quorum BFT.)*
      - [x] **6.5 — DoS durci.** *(fait)* Deux bornes : (a) **éviction des seaux** —
        au-delà de `MAX_BUCKETS` adresses suivies (usurpation de sources), on jette
        les seaux pleins (adresses inactives) → **mémoire bornée** (avant : 1 M de
        fausses adresses = OOM). (b) **Relais borné** — un protégé a un budget de
        relais (`RELAY_RATE`/s) et chaque paquet n'est ré-émis qu'à au plus
        `MAX_RELAY_FANOUT` voisins → le **facteur d'amplification réfléchie est
        plafonné** (avant : 1 paquet → N sortants illimités avec l'upload de la
        victime). **Ce que ça ferme :** trous n°9 et n°10. *(Ce sont des BORNES,
        pas un accept/reject binaire ; l'usurpation d'adresse source elle-même n'est
        pas testable sur localhost. Consentement explicite du relais = 6.7.)*
      - [x] **6.6 — Passage à l'échelle : voisinage borné.** *(fait)* Le rendez-vous
        ne renvoie plus TOUS les joueurs mais seulement les `MAX_NEIGHBORS` (= 32) les
        plus PROCHES (`keep_nearest` trie par distance et tronque). **Ce que ça ferme :**
        le WELCOME ne peut plus déborder le tampon (trou n°2), et chacun ne suit/parle
        qu'à ≤ 32 voisins → **O(N·K) au lieu d'O(N²)** (trou n°3) : c'est LA borne qui
        rend l'échelle possible (des milliers de petits voisinages de ~32, pas un
        maillage géant). **Vérifié** : sélection des K plus proches testée unitairement ;
        3 bots se voient tous (< 32). *(Limite assumée : le rendez-vous reste un point
        unique — pour une vraie échelle planétaire il faudrait LE sharder spatialement
        / le répliquer, et ajouter des relais TURN pour les NAT symétriques. C'est le
        gros chantier au-delà de cette étape.)*
      - [x] **6.7 — Réputation partagée (accusations signées + quorum).** *(fait)*
        Nouveau `net/accuse.rs` + paquet `KIND_ACCUSE`. Quand un nœud bannit un
        tricheur (triche ATTRIBUABLE), il **diffuse une accusation signée** (`punish`).
        Les autres n'y croient pas sur parole : ils attendent un **quorum**
        (`ACCUSE_QUORUM` = 3) d'accusateurs **distincts** avant de bannir à leur tour
        (`record_accusation`) — et chaque identité coûte une preuve de travail (6.2),
        donc fabriquer un faux quorum est cher. **Anti-framing** : un seul (ou
        quelques) menteur(s) ne peut rien ; on ne re-diffuse pas (pas de cascade) ;
        un nœud déjà banni ne « vote » plus. C'est la version légère, byzantine-
        tolérante, d'EigenTrust — l'étage « Shields » au-dessus du strike local.
        **Vérifié** : round-trip + quorum testés unitairement ; en headless le
        tricheur est banni et les accusations diffusées. *(Limite assumée : pas
        encore de quorum BFT 3f+1 formel sur l'orbe elle-même — voir « au-delà ».)*
      - [x] **6.8 — Simulation massive + essaim d'attaquants.** *(fait)* Nouveau mode
        `cargo run -- sim [bots] [attaquants] [secondes]` (`net/sim.rs`) : il lance un
        rendez-vous + N nœuds headless (`Bot`, refactoré en brique réutilisable) + M
        attaquants variés (orb-creep/teleport/flood/forge/sybil/gossip-flood), en threads sur une
        seule machine, et imprime un **rapport agrégé**. **Résultats mesurés** (release) :
        à **50** ET **300 bots** + attaquants → 100 % des nœuds montés, **voisins/nœud
        plafonnés à 32** (moy 32,0 — la borne d'échelle 6.6 tient), **orbe volée :
        0/N**, ~83 000 paquets honnêtes/s à 300, attaquants mis en sourdine. **Pourquoi
        ça vaut pour 55 K :** la charge PAR NŒUD ne dépend pas de N (chacun ne parle
        qu'à ~32 voisins) → la vraie échelle se fait en ajoutant des MACHINES (chaque
        joueur = un appareil réel), pas en surchargeant une seule. La simu valide la
        correction + la résistance aux attaques ; le passage planétaire = le chapitre
        suivant (adaptation au lien).
- [ ] **Chapitres 7 → 14 — le grand chantier post-BÉTON.**
      Découpage **détaillé dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)** (avec les
      22 doutes/risques qu'ils ferment) :
      **7** confrontation au réel (`tc netem` : latence/perte/NAT — *prochaine étape*) ·
      **8** inclusivité & adaptation au lien (de 0 à 2 Gb/s, parent/répartition de
      puissance, anti free-riding) · **9** durcissement de la confiance (Sybil, éclipse,
      rendez-vous décentralisé) · **10** identité persistante + chiffrement de tout ·
      **11** autorité généralisée (au-delà de l'orbe) & ordre temporel · **12**
      robustesse/longévité (éviction mémoire, TURN, IPv6) · **13** voix spatiale ·
      **14** (plus tard) portabilité Unreal/Unity.

---

## L'architecture cible (vision long terme)

Le but final, formalisé avec nos mots :

**« Own + Shields »** — pas de serveur central, chaque joueur est un nœud.
- **Own** (autorité) : **arbitre** l'état d'un objet ou d'une zone contestés. Si
  l'Own triche ou crashe, on le remplace (migration, déjà faite sur l'orbe).
- **Shields** (boucliers) : recalculent/vérifient l'Own et le bannissent en cas de
  triche. Quorum **BFT 3f+1** (1 Own + 3 Shields tolère 1 traître), comme PBFT.

> **Affinement majeur (acté en codant) : l'autorité est PAR OBJET, pas globale.**
> Un Own unique qui relaie *tout* pour toute l'instance redeviendrait le goulot
> d'upload qu'on veut éviter (un seul PC ne tient pas des milliers de flux). Donc :
> - **Ce qui est à toi** (ta position, ta voix) → **pas d'Own** : tu es ta propre
>   autorité, tu diffuses en **direct** à tes ~10 voisins (aucun conflit possible).
> - **Ce qui est partagé/contesté** (l'orbe, une porte, un score) → **un Own par
>   objet/zone**, à **bas débit** (un événement de temps en temps). Des milliers de
>   petits Owns, jamais un seul. 55 000 joueurs = des milliers de zones de ~10.
>
> **Own ≠ Relais.** L'**Own** *décide* (autorité, conflits). Le **Relais/parent**
> *recopie* des octets pour un joueur à faible upload (transport, zéro décision).
> Un même bon PC peut porter les deux casquettes, mais ce sont deux rôles séparés
> — et le relais ne doit jamais pouvoir **modifier** ce qu'il transporte (le joueur
> faible signera ses messages : enveloppe scellée, chapitre 5).

**Choix de l'Own** : meilleur matériel + meilleure réputation + au centre
géographique des joueurs (latence minimale).

**Parrainage / supernœuds** : un joueur à faible débit montant envoie ses
données une seule fois à un relais proche et fiable, qui les redistribue.

**Repli (fallback)** : si aucun nœud fiable n'est dispo, bascule sur un serveur
perso. Pareil pour le *signaling* (STUN/TURN) qui aide à percer les NAT.

### Vérités à garder en tête (corrections déjà actées)
- **« Plus de joueurs = plus stable » → faux.** En P2P naïf, chacun parle à
  tous : O(N²). La solution, c'est l'**AoI** (chapitre 3), pas la force brute.
- **« La blockchain résout la latence » → faux.** Un consensus est *lent*
  (secondes). On la réserverait à la **réputation**, jamais à la synchro temps
  réel (qui exige < 50 ms).
- **« On peut supprimer tout serveur » → presque.** Le **NAT** des box bloque
  les connexions entrantes ; il faut un petit serveur de *signaling* pour amorcer
  les connexions directes (hole-punching). Le jeu, lui, reste 100 % P2P : une fois
  les présentations faites, on peut **tuer le rendez-vous**, la partie continue.
- **« Réduire à 4 envois par joueur résout le passage à l'échelle » → faux.** Le
  travail O(N²) ne disparaît pas, il **déménage** sur le nœud qui redistribue (qui
  exploserait à 22 Gbps pour 55 000 joueurs). Le goulot, c'est **toujours** l'upload
  de celui qui rediffuse. La vraie réponse : l'**AoI** (tu ne parles qu'à tes ~10–100
  voisins, **indépendamment de N**) + des Owns/relais **locaux**, jamais un hub.

---

## Glossaire express

- **P2P** : les joueurs se parlent directement, sans serveur au milieu.
- **NAT** : la box Internet qui cache et bloque les connexions entrantes.
- **UDP** : envoi de paquets « sans accusé de réception » (rapide, mais on peut
  perdre des paquets — parfait pour un jeu).
- **Jitter** : les paquets n'arrivent pas à intervalles réguliers.
- **Interpolation** : afficher une position intermédiaire entre deux paquets
  pour un mouvement fluide.
- **AoI (Area of Interest)** : ne se synchroniser qu'avec les joueurs proches.
- **BFT** : tolérance aux pannes/traîtres par vote majoritaire.
- **Sybil** : un attaquant crée plein de faux nœuds pour fausser les votes.
