# ARCHITECTURE — l'organisation du code & la cible long terme

> Organisation des fichiers (`src/`), l'en-tête de paquet commun, et l'architecture cible « Own + Shields »
> (autorité par objet, BFT, relais).
> *L'idée en mots simples, avant le code : [le « sans serveur », en clair](comprendre-le-p2p.md).*

## Organisation du code (`src/`)

Principe : **un fichier = une responsabilité** (plein de petits fichiers plutôt
qu'un gros).

```
src/
├── main.rs              point d'entrée, aiguillage des modes headless (rendezvous/sidecar/bot/agent/sim/…)
├── math.rs              Vec3 maison (sans moteur 3D) — la brique maths du cœur
├── dsp/                 LA VOIX : bancs de traitement du signal, fait main, std-only (chantier en pause)
│   ├── fft.rs           FFT/STFT radix-2 fait main — le socle spectral commun
│   ├── psycho.rs        modèle psychoacoustique (bandes de Bark + masquage) — la qualité perçue
│   ├── codec.rs         codec transform-domain + allocation perceptuelle (garde chant/chuchotement/beatbox)
│   ├── denoise.rs       « étude du micro » : débruitage par soustraction spectrale, contrôlé par l'utilisateur
│   ├── separate.rs      séparation de sources : énumérer/isoler/retirer chaque bruit, au choix
│   ├── stoi.rs          intelligibilité STOI (white-box) — « la voix est-elle comprise ? »
│   ├── chain.rs         la chaîne bout-en-bout (débruitage → codec → transport → verdict)
│   ├── optim.rs         optimisation du codec guidée par STOI (trouver le « genou » débit/qualité)
│   ├── adaptive.rs      contrôleur adaptatif : re-règle débit/buffer/débruitage selon le lien observé
│   └── spectro.rs       spectrogramme PNG fait main — des « yeux » pour juger un son sans oreilles
└── net/                 LE RÉSEAU, fait main (engine-agnostique, aucun moteur 3D)
    ├── mod.rs           assemble le module et expose l'API publique
    ├── wire.rs          le TYPE d'un paquet (1er octet) + la version de protocole + le port du rendez-vous
    ├── message.rs       le format d'un paquet (PlayerState, encode/decode + signé, lots d'états)
    ├── control.rs       les messages d'annuaire (HELLO / WELCOME)
    ├── crypto.rs        Ed25519 + PeerId (identité = clé) + preuve de travail — la seule « boîte noire »
    ├── rendezvous.rs    le POINT DE RENDEZ-VOUS : présente les joueurs puis s'efface (+ relais des NAT durs)
    ├── transport.rs     la prise UDP brute (Socket) — la « connexion »
    ├── punch.rs         hole punching : frontière wire (encode/decode/abandon du perçage)
    ├── linkprobe.rs     la SONDE DE LIEN : type de NAT (STUN), latence, gigue, nature de la perte (Phase 2)
    ├── linkstats.rs     les stats d'un lien observé : perte RÉELLE vs apparente, gigue, fraîcheur
    ├── gossip.rs        découverte décentralisée : « cartes de visite » entre pairs, sans annuaire (D22)
    ├── cell.rs          RÉSUMÉ de cellule : percevoir une foule lointaine sans N flux (D22)
    ├── aoi.rs           Area of Interest (water-filling : qui reçoit quel débit)
    ├── aoi_bench.rs     banc « pertinence sociale vs proximité » (cargo run -- aoi)
    ├── aoi_e2e.rs       le même, bout-en-bout sur le vrai transport (cargo run -- aoi-live)
    ├── anticheat.rs     le « Shield local » : règles de plausibilité physique (chap. 6.3+)
    ├── accuse.rs        accusations signées + quorum : réputation partagée (chap. 6.7)
    ├── orb.rs           l'orbe partagée : logique PURE d'autorité (ORBE+OWN, encode/decode signé, migration)
    ├── stars.rs         le champ d'étoiles partagé (monde de démonstration)
    ├── skin.rs          la couleur de skin aléatoire (portée dans le paquet d'état)
    ├── link.rs          NetLink : l'état réseau d'un nœud (table de pairs, réputation, cellules…)
    ├── bot.rs           le CLIENT HEADLESS (cargo run -- bot …) + brique `Bot` réutilisable
    ├── sidecar.rs       LE PONT vers Unreal (cargo run -- sidecar) : socket locale vers le client moteur
    ├── metrics.rs       l'AGENT DE MESURE distribué (cargo run -- agent/stats) : mesure les liens dehors et remonte les chiffres
    ├── session_window.rs la fenêtre visible de session : le volontaire VOIT ce que l'agent mesure
    ├── probe.rs         sonde système (CPU/RAM du nœud, via /proc) — chiffrer le coût réel d'un nœud
    ├── liveness.rs      banc « est-ce vivant ? » : fidélité / fraîcheur / à-coups (cargo run -- vivant)
    ├── voice_bench.rs   banc de TRANSPORT de la voix (latence bouche-à-oreille, jitter buffer)
    ├── sim.rs           la SIMULATION MASSIVE (cargo run -- sim N M T) : N nœuds + M attaquants
    ├── coopsim.rs       bancs de foule en thread coopératif / bus mémoire (coopsim, coopsim-bus)
    ├── lossbench.rs     banc DÉTERMINISTE de la redondance sous perte connue (cargo run -- phase1)
    ├── netembench.rs    banc RÉEL de la redondance sur lien dégradé (cargo run -- netem-bench)
    ├── attack.rs        le PROGRAMME ATTAQUANT (cargo run -- attack …) — chap. 5 & 6
    ├── natdemo.rs       le mode texte nat-test (hole punching sans 3D, pour netns)
    └── demo.rs          le mode texte net-demo (observer les paquets)
```

> Deux **composants exécutables** méritent un mot, car ils sont au cœur des preuves « dehors » : le **rendez-vous**
> (`rendezvous.rs`) — l'annuaire minimal qui présente les joueurs, puis relaie pour les NAT trop fermés — et l'**agent
> de mesure** (`metrics.rs`) — un instrument autonome qu'un volontaire lance chez lui, qui rejoint le réseau, **sonde
> son lien** (`linkprobe.rs`) et **remonte des chiffres honnêtes** sur la vivacité des liens distants.

> Le **rattrapage de latence** (interpolation, prédiction, ressort amorti) qui vivait dans
> `net/netcode/` côté client Bevy a été **retiré** : c'est désormais Unreal qui interpole
> les avatars distants à partir de la vitesse reçue (via le pont *sidecar*).

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

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Architecture & code** — une étape des parcours **⚙️ Le code** et **🧭 Tout comprendre**.

**Continuer le fil :**
- ⚙️ *Le code* · 🧭 *Tout comprendre* → **[Chantier réseau](chantier-reseau.md)**

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)

---

