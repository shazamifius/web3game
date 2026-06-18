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
    ├── control.rs       les messages d'annuaire (HELLO / WELCOME + clés publiques)
    ├── crypto.rs        signatures Ed25519 — la SEULE boîte noire (chap. 5)
    ├── aoi.rs           Area of Interest (water-filling : qui reçoit quel débit)
    ├── punch.rs         hole punching (percer les NAT pour une connexion directe)
    ├── orb.rs           l'orbe partagée : objet à maître unique + migration d'hôte
    ├── transport.rs     la prise UDP brute (Socket) — la « connexion »
    ├── skin.rs          la couleur de skin aléatoire
    ├── demo.rs          le mode texte net-demo (observer les paquets)
    ├── attack.rs        le PROGRAMME ATTAQUANT (cargo run -- attack …) — chap. 5
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
d'un **sceau Ed25519 de 64 octets** (état signé = 56 + 64 = **120 octets**). Le
récepteur le **vérifie** avec la clé publique de l'émetteur (reçue via le
rendez-vous) avant de l'accepter. Le corps lui-même reste :

Un paquet de joueur fait **56 octets** : `type` (1) + `version` (1) + `id` (1) +
`x,y,z` + `vx,vy,vz` + `yaw,pitch` + `r,g,b` (11 × 4 octets) + `parent` (1) +
`seq` (8, compteur anti-rejeu). Voir `net/message.rs`. Un paquet d'orbe **signé**
fait **105 octets** (corps 41 + sceau 64). Le corps de **41 octets** :
`type` + `version` + `owner` + `version d'orbe` +
position, vitesse et couleur. Voir `net/orb.rs`.

**Convention « fichier inactif »** : un fichier qui n'est plus utilisé est
préfixé d'un `_` (ex. `_demo.rs`) et sa ligne `mod` est retirée. Il remonte en
tête de liste et signale d'un coup d'œil qu'il ne sert plus — sans le ranger
dans un sous-dossier. (Le compilateur Rust confirme l'inverse : si un fichier
*est* branché sans warning `unused`, c'est qu'il sert.)

---

## Feuille de route (le cours, désormais en 7 chapitres)

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
      - [ ] **6.0 — Carte des menaces + harnais d'attaque « rouge ».** On écrit le
        modèle de menace et on ajoute à `attack.rs` les attaques qui RÉUSSISSENT
        encore aujourd'hui (téléport / speed-hack, Sybil-reconnexion, collision
        d'id, amplification par relais) : autant de tests « rouges » à passer au
        vert au fil des étapes suivantes.
      - [ ] **6.1 — Identité auto-certifiante (le keystone « web3 »).** L'identité
        d'un joueur DEVIENT sa clé publique (ou son empreinte), au lieu d'un `u8`
        assigné par le rendez-vous. D'un coup : plus de mur des 255 joueurs, plus
        de collision d'id, et surtout **le rendez-vous ne peut plus mentir** sur
        « telle clé = tel joueur » (aujourd'hui toute la signature repose sur son
        honnêteté). Touche tous les formats de paquet.
      - [ ] **6.2 — Coût d'entrée anti-Sybil.** Une identité doit COÛTER (preuve de
        travail façon Hashcash sur la clé). Sans ça, un banni se reconnecte en une
        milliseconde avec une clé neuve → la réputation/sourdine ne vaut rien.
      - [ ] **6.3 — Validation de mouvement (anti-téléport / speed-hack).** Un état
        signé avec un saut de position physiquement impossible est refusé et compté
        comme faute. La signature prouve QUI ; ici on prouve que le mouvement est
        PLAUSIBLE.
      - [ ] **6.4 — Orbe : preuve de contact + version stricte.** On ferme le vol
        d'orbe par incréments +1 (revendiquer l'orbe sans l'avoir touchée) : le
        Shield vérifie la plausibilité (le revendiqueur était-il près de l'orbe ?).
      - [ ] **6.5 — DoS durci.** Rate-limit résistant au spoofing d'adresse source +
        éviction des seaux (sinon 1 M de fausses adresses = mémoire saturée). Relais
        avec consentement + AoI sur la rediffusion (sinon amplification réfléchie
        avec l'upload de la victime).
      - [ ] **6.6 — Passage à l'échelle.** Roster paginé / sharding spatial côté
        rendez-vous, AoI qui borne le NOMBRE de voisins (pas seulement le débit),
        WELCOME découpé. Lève le mur actuel (~52 pairs visibles, et O(N²)).
      - [ ] **6.7 — Quorum BFT des Shields.** Accusations signées partagées entre
        nœuds (réputation décentralisée, EigenTrust) : l'étage au-dessus du strike
        purement local.
      - [ ] **6.8 — Simulation 55 K + essaim d'attaquants.** Un harnais qui lance
        des milliers de bots honnêtes ET N attaquants de tout type, et MESURE que
        l'architecture tient.
- [ ] **Chapitre 7 — Voix spatiale**
      Chat vocal P2P avec priorité au volume (*loudness priority*).

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
