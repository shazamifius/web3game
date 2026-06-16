# web3game — un VRChat-like en pair-à-pair

Un petit univers social en **vue première personne** où l'on veut connecter des
centaines de joueurs **sans aucun serveur de jeu central**, en pur pair-à-pair
(P2P). Le but n'est pas un jeu de tir : c'est un espace social, comme VRChat.

> ⚠️ **Important** : ce projet est écrit en **Rust + Bevy**, pas en Unreal
> Engine. On a choisi Rust pour tout contrôler nous-mêmes — chaque octet qui
> part sur le réseau est écrit à la main, aucune « boîte noire ». L'architecture
> réseau décrite plus bas (Own+Shields, BFT, AoI…) est de la **logique**, elle
> resterait la même avec n'importe quel moteur 3D.

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

**Jouer à deux fenêtres sur le même PC** (multijoueur local) :

```fish
nix-shell --run "cargo run -- a"     # terminal 1  (joueur 1)
nix-shell --run "cargo run -- b"     # terminal 2  (joueur 2)
```

**Voir le réseau seul, en texte** (sans la 3D, pour observer les paquets) :

```fish
nix-shell --run "cargo run -- net-demo a"
nix-shell --run "cargo run -- net-demo b"
```

**Développer avec relance automatique** (le jeu se recompile et redémarre à
chaque sauvegarde — confort de dev, via `cargo-watch`) :

```fish
nix-shell --run "cargo watch -x 'run -- a'"     # terminal 1
nix-shell --run "cargo watch -x 'run -- b'"     # terminal 2
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
    ├── message.rs       le format d'un paquet (PlayerState, encode/decode)
    ├── transport.rs     la prise UDP brute (NetPeer) — la « connexion »
    ├── skin.rs          la couleur de skin aléatoire
    ├── demo.rs          le mode texte net-demo (observer les paquets)
    ├── link.rs          NetLink, la ressource qui relie le réseau au jeu
    └── netcode/         LE RATTRAPAGE DE LATENCE
        ├── mod.rs       assemble le sous-module
        ├── state.rs     instantanés, file par joueur, RÉGLAGES (constantes)
        ├── send.rs      émettre notre état (débit limité + vraie vitesse)
        ├── receive.rs   ranger les paquets reçus (et créer l'avatar)
        ├── interpolate.rs   animer chaque image (horloge adaptative + ressort)
        ├── predict.rs   calculer l'état voulu (interpolation ou prédiction)
        └── smooth.rs    le ressort amorti (SmoothDamp) + helpers d'angles
```

Un paquet de joueur fait aujourd'hui **45 octets** : `id` (1) + `x,y,z` +
`vx,vy,vz` + `yaw,pitch` + `r,g,b` (11 × 4 octets). Voir `net/message.rs`.

**Convention « fichier inactif »** : un fichier qui n'est plus utilisé est
préfixé d'un `_` (ex. `_demo.rs`) et sa ligne `mod` est retirée. Il remonte en
tête de liste et signale d'un coup d'œil qu'il ne sert plus — sans le ranger
dans un sous-dossier. (Le compilateur Rust confirme l'inverse : si un fichier
*est* branché sans warning `unused`, c'est qu'il sert.)

---

## Feuille de route (le cours en 6 chapitres)

On avance **en codant pour de vrai**, chapitre par chapitre. On part du plus
simple (deux PC qui se parlent) vers le plus dur (des centaines de joueurs,
anti-triche).

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
- [ ] **Chapitre 3 — Topologie & passage à l'échelle**
      NAT, STUN, hole-punching (se connecter sans serveur). **Area of Interest**
      (AoI) : ne parler qu'aux joueurs proches → passer de O(N²) à O(N).
- [ ] **Chapitre 4 — Autorité & migration d'hôte**
      Modèle **Own + Shields** (1 hôte + 3 vérificateurs = BFT 3f+1). Élection,
      détection de panne, migration sans coupure (problème du *split-brain*).
- [ ] **Chapitre 5 — Confiance & anti-triche**
      Réputation décentralisée (**EigenTrust**), supernœuds/parrainage pour les
      mauvaises connexions, et le vrai ennemi de fond : l'**attaque Sybil**.
- [ ] **Chapitre 6 — Voix spatiale**
      Chat vocal P2P avec priorité au volume (*loudness priority*).

---

## L'architecture cible (vision long terme)

Le but final, formalisé avec nos mots :

**« Own + Shields »** — pas de serveur central, chaque joueur est un nœud.
- **1 Own** (hôte) : reçoit les actions de tous, les valide, redistribue.
- **3 Shields** (boucliers) : recalculent en parallèle et comparent. Si l'Own
  triche ou crashe, ils le bannissent et élisent un nouvel Own.
- C'est exactement un quorum **BFT 3f+1** (tolère 1 traître) — la même idée que
  PBFT dans les blockchains.

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
  les connexions directes (hole-punching). Le jeu, lui, reste 100 % P2P.

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
