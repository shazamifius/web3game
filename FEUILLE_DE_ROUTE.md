# Feuille de route détaillée — web3game

> **But de ce document.** Le `README.md` est le *résumé* (où on en est, comment lancer).
> CE fichier est le *plan complet et honnête* : tous mes doutes d'ingénieur, et le
> programme détaillé pour les résoudre. Il est volontairement long. On l'écrit
> AVANT de coder, pour avoir une carte claire du gros chantier qui vient.
>
> **Comment le lire.** Section **B** = ce qui est fait. Section **C** = l'inventaire
> des doutes (D1…D21), c'est le cœur. Section **D** = le programme (chapitres 7→14),
> chaque chapitre ferme des doutes précis. Section **E** = comment tester *pour de
> vrai* avec une seule machine. Section **F** = l'ordre conseillé. Section **G** =
> les décisions qui t'appartiennent.
>
> **Règle d'or rappelée.** On ne vise pas l'inviolable absolu (ça n'existe pas). On
> vise : *chaque attaque devient soit impossible, soit chère, soit attribuable et
> bannie* — et *chaque joueur, du 0 connexion au 2 Gb/s, a la meilleure expérience
> possible pour SON lien*.

---

## 0. ▶️ POINT DE REPRISE (lis ça en premier, surtout si nouvelle session)

**Où on en est :** le **chapitre 6 (refonte BÉTON) est TERMINÉ** — les 10 trous de
l'audit fermés/bornés (0.0→6.8), **35 tests, 0 warning**, et **le jeu 3D réel
fonctionne** (avatars + pseudos `0000…` + badge OWN BALLE visibles à l'écran). Tout
est poussé sur GitHub (`shazamifius/web3game`, branche `main`).

**Les 4 décisions de direction sont prises** (détail section G) : ① **on chiffre
tout** (ch. 10) ; ② **PoW anti-Sybil réglable** (on durcit si les tests l'exigent) ;
③ **ordre normal** 7→8→9→10 (pas de priorité forcée au 0-connexion) ; ④ **identité
persistante = clé sauvée dans un fichier** (ch. 10).

**On est dans le CHAPITRE 7 (confrontation au réel). 7.1 est FAIT.**
**7.1 ✓** — `tools/sim-netem.sh` écrit et **prouvé réel** : il applique `tc netem` sur
`lo` (3 profils `bon|moyen|mauvais`), lance la simu, et retire toujours le netem à la
sortie (`trap`). Validé en profil `moyen` (~120 ms + 2 % perte) : l'essaim a TENU
(20/20 montés, orbe intègre, orb-creep mis en sourdine). Piège tranché : sur `lo` le
délai compte double → profils exprimés en ping cible, `delay = ping ÷ 2`.

**PROCHAINE ACTION = 7.2** : faire tourner la simu sous les 3 profils (bon 30 ms /
moyen 120 ms+2 % / mauvais 250 ms+5 %+jitter+ré-ordo) et MESURER (orbe intègre ? fausses
migrations ? débit honnête stable ? rejets anti-rejeu dus au ré-ordonnancement ? et
**élucider pourquoi le `teleport` n'a laissé aucune trace** au run 7.1). Puis **7.3**
corriger ce que netem révèle (probable : l'anti-rejeu strict casse sur paquets
re-ordonnés → fenêtre de tolérance). Détail complet : section D, chapitre 7.

**Méthode de travail (rappel des préférences de l'utilisateur) :** parler **français**
uniquement ; débutant Linux → toujours donner les commandes complètes **avec `cd`** ;
**critique honnête d'ingénieur, jamais de flatterie** ; **toujours exprimer ses doutes** ;
on **écrit le plan avant de coder** (cette phase de plan est faite — on peut coder le
ch. 7) ; **petites étapes** (chacune compilée + testée + prouvée en headless/simu, puis
commitée et écrite dans ce doc / le README) ; **toujours sauver sur GitHub** à chaque
étape. La vérification se fait **sans GPU** via les bots/simu (le jeu 3D, c'est
l'utilisateur qui le lance). Avant tout gros run de simu : `tools/sim-cool.sh` pousse
les ventilos au max (PC tour ASUS — sinon BIOS Q-Fan « Full Speed »).

**Comment lancer / tester :** voir le `README.md` (section « Comment lancer ») et les
modes `rendezvous | a | b | bot <nom> | attack <type> | sim <bots> <attaquants> <s>`.

---

## A. Les principes directeurs (la boussole)

1. **Aucun serveur de jeu central.** Un rendez-vous d'annuaire est toléré, mais il ne
   doit jamais devenir l'arbitre du jeu ni un point de confiance aveugle. À terme,
   même lui doit se décentraliser.
2. **Contrôle maximal, tout fait main.** Une seule boîte noire assumée : la crypto
   (`ed25519-dalek`). Tout le reste, on le comprend octet par octet.
3. **Répartition de la puissance = pilier.** Les forts aident les faibles (système de
   parent/relais). Personne n'a de pouvoir total (moindre privilège).
4. **Inclusivité radicale.** N'importe qui doit pouvoir jouer : de la pire connexion
   (quelques Ko/s, voire 0 via un parent) à la fibre 2 Gb/s. Chacun la meilleure
   expérience POUR SON lien.
5. **Honnêteté.** On préfère un MVP qui marche et des limites écrites, à une fausse
   promesse. Tout doute est un travail à planifier, pas à cacher.
6. **Plus tard, pas maintenant :** portabilité Unreal/Unity. On finit le cœur d'abord.

---

## B. État des lieux (ce qui est FAIT)

Chapitres **0 → 5** : bac à sable 3D, transport UDP brut, netcode (interpolation /
prédiction / réconciliation), topologie & AoI (water-filling), autorité & migration
d'hôte (l'orbe), confiance & anti-triche (Ed25519, anti-rejeu, réputation locale,
rate-limit). Détail dans le `README`.

Chapitre **6 — refonte BÉTON** : **terminé**, les 10 trous de l'audit fermés/bornés.

| Sous-étape | Ce qu'elle ferme |
|---|---|
| 6.0 Bot headless + 4 attaques « rouges » | Le filet de vérification (sans GPU) |
| 6.1 Identité auto-certifiante (id = clé) | Plafond 255, collisions, **rendez-vous menteur** |
| 6.2 Preuve de travail anti-Sybil | Identité gratuite |
| 6.3 Validation de mouvement | Téléport / speed-hack |
| 6.4 Preuve de contact de l'orbe | Vol d'orbe par incréments |
| 6.5 DoS borné (éviction + relais plafonné) | Mémoire saturée, amplification réfléchie |
| 6.6 Voisinage borné (`MAX_NEIGHBORS`) | WELCOME tronqué, maillage O(N²) |
| 6.7 Réputation partagée (accusations + quorum) | Tricheur invisible pour qui ne l'a pas vu |
| 6.8 Simulation massive + essaim | **Preuve** que ça tient (50 & 300 nœuds) |

**Outils disponibles** : `cargo run -- rendezvous | a | b | bot <nom> | attack <type> |
sim <bots> <attaquants> <s>`, plus `tools/sim-cool.sh` (ventilos), `tools/test-nat.sh`
(NAT en namespaces) et `tools/sim-netem.sh` (mauvaise connexion via `tc netem`, ch.7.1).
**35 tests unitaires, 0 warning.** Le jeu 3D réel fonctionne
(vérifié à l'écran : avatars + pseudos `0000…` + badge OWN BALLE).

> **Ce que B prouve vraiment, et ne prouve pas.** B prouve la *correction* et la
> *résistance aux attaques connues*, sur localhost. B ne prouve **rien** sur le
> comportement en *vraie* condition réseau (latence, perte, NAT) ni sur l'*inclusivité*
> des liens faibles. C'est le point de départ de tout ce qui suit.

---

## C. L'inventaire des doutes (le cœur du document)

Chaque doute a : **Constat** (ce qui ne va pas / ce dont je ne suis pas sûr),
**Pourquoi ça compte**, **Gravité** (🔴 critique / 🟠 important / 🟡 à surveiller),
**Piste de correction**, **Comment on le vérifiera**. Le chapitre qui le ferme est
indiqué entre crochets `[ch. X]`.

### Catégorie 1 — Le réalisme de nos tests

**D1 — Nos tests « mentent » comme localhost.** 🔴 `[ch. 7]`
*Constat :* toute la simu tourne sur une machine, sans latence, sans perte, sans NAT,
sans jitter. *Pourquoi :* la règle d'or du projet est « jamais ‘ça marche sur
localhost' » ; un netcode peut être parfait sur `lo` et s'écrouler à 150 ms + 2 % de
perte (la prédiction part en vrille, l'anti-rejeu rejette des paquets re-ordonnés,
les migrations se déclenchent à tort). *Piste :* injecter des conditions réseau
réelles avec `tc netem` sur la boucle locale, et du NAT avec `ip netns` — **une seule
machine suffit** (voir section E). *Vérif :* refaire tourner `sim` à travers netem et
mesurer que l'essaim tient (orbe intègre, débit honnête stable, pas de fausses
migrations) à 100/150/250 ms et 1/3/5 % de perte.

**D2 — Le bot de sim ≠ exactement le jeu.** 🟡 `[ch. 12]`
*Constat :* `bot.rs` réécrit la boucle de réception de `receive.rs` (les *décisions* de
confiance sont partagées, mais l'*orchestration* est dupliquée). *Pourquoi :* un
correctif dans l'un peut ne pas atteindre l'autre → divergence silencieuse à long
terme. *Piste :* extraire un cœur de session commun (un seul `Bot`/`Session` que le
jeu Bevy ET le bot pilotent). *Vérif :* le jeu et le bot partagent le même module de
boucle ; un test prouve qu'ils traitent un paquet donné identiquement.

### Catégorie 2 — L'inclusivité (le cœur de la vision)

**D3 — Un lien faible ne peut PAS suivre, et ne peut pas dire « envoyez-moi moins ».** 🔴 `[ch. 8]`
*Constat :* dans une foule dense, un joueur **reçoit ~43 Ko/s** (jusqu'à 32 voisins qui
lui émettent ; l'AoI borne ce que TOI tu émets, **pas ce que tu reçois**). Un joueur à
quelques Ko/s est noyé. *Pourquoi :* c'est l'inverse exact de l'objectif « n'importe
qui peut jouer ». *Piste :* (a) un **contrôle de débit côté receveur** — chaque joueur
annonce un « budget de réception » (Ko/s) et un rayon d'intérêt ; les émetteurs en
tiennent compte (le water-filling devient bilatéral). (b) Pour les très faibles : le
**parent agrège et dégrade** — il reçoit les 32 voisins et n'envoie au protégé qu'un
résumé basse fréquence (positions échantillonnées, foule en LOD). *Vérif :* sous netem
throttlé à 5 Ko/s, le joueur reste fonctionnel (voit ses voisins immédiats, foule
lointaine en basse fidélité) et ne perd pas la connexion.

**D4 — L'économie du parent n'est pas résolue (free-riding).** 🔴 `[ch. 8]`
*Constat :* *pourquoi* un nœud à bon upload dépenserait-il sa bande passante pour les
autres ? Sans réponse, tout le monde se déclare « faible » et personne ne relaie.
*Pourquoi :* c'est LE problème de mécanisme du « partage de puissance ». *Piste :*
réciprocité façon **BitTorrent** (donnant-donnant : je relaie surtout pour ceux qui me
rendent service / ont bonne réputation ; « optimistic unchoke » pour amorcer), la
réputation devenant une quasi-monnaie. *Vérif :* en simu, des nœuds égoïstes (qui ne
relaient jamais) obtiennent un service dégradé ; les coopératifs, un bon service.

**D5 — Un parent malveillant censure en silence (disponibilité ≠ intégrité).** 🟠 `[ch. 8]`
*Constat :* la signature garantit qu'un parent ne *falsifie* pas ton état, mais rien ne
l'empêche de le *jeter* — tu deviens invisible. *Piste :* plusieurs parents en
parallèle (redondance), + détection du drop (si mes voisins ne confirment jamais avoir
reçu mon état via le parent, je change de parent). *Vérif :* un parent « trou noir »
est détecté et contourné en N secondes.

### Catégorie 3 — Sybil & réputation

**D6 — La preuve de travail (16 bits ≈ 1 s) est un jouet.** 🔴 `[ch. 9]`
*Constat :* miner une identité coûte ~1 s ; un attaquant en fabrique des centaines en
minutes. *Pourquoi :* toute la réputation/quorum repose sur « une identité coûte cher »
— or ce n'est pas le cas. *Piste :* difficulté **beaucoup** plus haute (réglable, +
adaptative à la charge), et/ou un second facteur (coût social : il faut être *vouché*
par des pairs déjà réputés pour peser dans un quorum). *Vérif :* le temps/coût pour
fabriquer un quorum d'identités devient prohibitif (minutes→heures), mesuré.

**D7 — Le quorum d'accusation (=3) permet le *framing*.** 🔴 `[ch. 9]`
*Constat :* 3 identités minées (~3 s avec D6 non corrigé) suffisent à faire bannir un
innocent par quorum. *Pourquoi :* l'anti-framing du 6.7 n'est solide que si fabriquer 3
accusateurs distincts est cher. *Piste :* lier au D6 (identités chères) **et** pondérer
les accusations par la réputation de l'accusateur (un inconnu pèse peu), **et** exiger
que les accusateurs soient eux-mêmes des voisins plausibles de l'accusé (on n'accuse
crédiblement que ce qu'on a pu observer). *Vérif :* en simu, K attaquants ne peuvent
PAS faire bannir un nœud honnête, même en se coordonnant.

**D8 — Aucune réhabilitation, aucune expiration.** 🟠 `[ch. 9]`
*Constat :* une fois muet (par fautes ou quorum), c'est définitif ; les compteurs de
fautes/accusations ne s'effacent jamais. *Pourquoi :* injuste (un faux positif est une
condamnation à vie) et ça fuit en mémoire (cf. D17). *Piste :* fenêtre glissante (les
fautes anciennes s'estompent), et procédure d'appel/quarantaine plutôt que ban sec.
*Vérif :* un nœud injustement muté redevient audible après une période de bon
comportement ; les compteurs décroissent.

### Catégorie 4 — La confiance topologique

**D9 — La position n'est pas vérifiée → attaque par éclipse.** 🔴 `[ch. 9]`
*Constat :* un nœud peut **mentir sur sa position** pour se déclarer voisin le plus
proche de tout le monde, s'insérer dans chaque voisinage, t'isoler (eclipse) ou tout
surveiller. Le choix des voisins fait confiance au rendez-vous + à des positions
auto-déclarées. *Pourquoi :* qui contrôle tes voisins contrôle ta réalité (il peut te
cacher des joueurs, t'en inventer, te couper). *Piste :* diversité forcée du voisinage
(ne pas prendre tous ses voisins de la même source/sous-réseau ; mélanger « plus
proches » et « aléatoires vérifiés », façon table de routage Kademlia), et corroborer
la position d'un pair par ce que d'autres en rapportent. *Vérif :* un attaquant qui ment
sur sa position ne peut pas occuper plus d'une fraction bornée d'un voisinage.

**D10 — Le rendez-vous reste un point unique (panne, centralisation, vie privée, DoS).** 🔴 `[ch. 9]`
*Constat :* s'il meurt → plus de découverte ni de détection de vivacité ; il voit
**toutes** les IP + positions (vie privée) ; il choisit tes voisins (cf. D9) ; il n'a
**aucun rate-limit** (on peut l'inonder de HELLO valides). *Pourquoi :* c'est l'astérisque
géant sous « pas de serveur ». *Piste :* fédération (plusieurs rendez-vous qui
s'échangent des pairs), puis découverte par **DHT / gossip** (les pairs s'entre-présentent),
le rendez-vous ne servant qu'à l'amorçage. + rate-limit + PoW à l'entrée du rendez-vous.
*Vérif :* tuer le rendez-vous en cours de partie ne casse pas une session déjà établie ;
deux rendez-vous se partagent les joueurs.

### Catégorie 5 — L'autorité des objets

**D11 — La migration de l'orbe est le point mou (ta question « comment corriger ? »).** 🟠 `[ch. 11]`
*Constat :* quand le maître se tait (> `MASTER_TIMEOUT`), un revendiqueur **inconnu** est
accepté **sans** preuve de contact (sinon on bloquerait une vraie reprise). Un attaquant
patient attend ce délai puis prend l'orbe. *Pourquoi :* c'est la seule porte laissée
ouverte sur l'orbe. *Comment corriger, concrètement :*
  1. **Élection confirmée par quorum** : la reprise n'est valide que si un quorum de
     voisins (a) confirme que l'ancien maître est bien silencieux ET (b) est d'accord
     sur le gagnant déterministe (le plus petit id parmi les *présents confirmés*). Un
     attaquant seul ne fait pas quorum → il ne peut pas voler pendant la panne.
  2. **Le gagnant doit prouver sa présence récente** (un état signé récent dans la zone),
     pas forcément le contact, mais au moins « j'existe et je suis là, maintenant ».
  3. **Période de grâce** : pendant la fenêtre de migration, on n'accepte AUCUN saut de
     version > +1, et on log/quarantaine tout candidat multiple (signe de course/triche).
*Vérif :* une attaque « j'attends le timeout puis je prends » échoue tant qu'un quorum de
voisins honnêtes est présent.

**D12 — Tout est codé pour UN objet (l'orbe).** 🟠 `[ch. 11]`
*Constat :* le vrai jeu aura des milliers d'objets partagés (portes, scores, projectiles).
L'autorité par objet + migration + preuve de contact doit se **généraliser** et passer
à l'échelle (des milliers d'autorités à bas débit). *Piste :* un registre générique
d'objets autoritaires (id d'objet, maître, version, règle de contact paramétrable),
réutilisant la machinerie de l'orbe. *Vérif :* 1000 objets partagés simulés tiennent,
chacun avec son maître, sans exploser le débit.

**D13 — Pas d'horloge commune → conflits mal arbitrés.** 🟡 `[ch. 11]`
*Constat :* deux joueurs qui touchent un objet « en même temps » : qui a raison ? On
départage par (version, id), pas par le temps réel. Les `now` locaux diffèrent. *Pourquoi :*
pour des règles de jeu équitables (qui a marqué en premier ?), il faut un ordre temporel.
*Piste :* horodatage signé dans les paquets + synchro d'horloge légère entre pairs
(estimation d'offset, façon NTP simplifié) ; **jamais** de consensus lourd sur le chemin
temps réel. *Vérif :* une course à l'objet est tranchée de façon cohérente par tous.

### Catégorie 6 — Identité & vie privée

**D14 — L'identité n'est PAS persistante.** 🔴 `[ch. 10]`
*Constat :* `NetLink::new` **mine une nouvelle identité à chaque lancement** du jeu. Ton
identité change donc à chaque session : tes amis ne te reconnaissent pas, ta réputation
ne s'accumule pas, tu n'as pas de « compte ». *Pourquoi :* indispensable pour un jeu
social réel ET pour que la réputation/Sybil ait un sens dans la durée. *Piste :* générer
la paire de clés **une fois**, la sauver chiffrée sur le disque, la recharger ensuite
(la preuve de travail ne se paie qu'à la création). Option : pseudo lisible + avatar liés
à la clé. *Vérif :* relancer le jeu garde la même identité (même `0000…`) ; la réputation
survit aux sessions.

**D15 — Tout est en clair (aucune confidentialité).** 🟠 `[ch. 10]`
*Constat :* positions, et bientôt la **voix**, circulent en clair ; le rendez-vous et tout
nœud sur le chemin voient où tu es et avec qui. La signature prouve l'authenticité, pas
le secret. *Pourquoi :* pour de vrais utilisateurs, savoir « qui est où » en clair est un
trou de vie privée (harcèlement, traçage). *Piste :* chiffrement de transport par paire
via un échange de clés **X25519** (Diffie-Hellman sur courbe, cousin d'Ed25519) → un
secret partagé par paire, le contenu chiffré, la signature conservée pour l'intégrité.
*Vérif :* un observateur tiers ne peut plus lire positions/voix entre deux pairs.

### Catégorie 7 — Robustesse & longévité

**D16 — Fuites mémoire à long terme.** 🟠 `[ch. 12]`
*Constat :* `last_seq`, `strikes`, `accusations`, `last_state`, `holes` grossissent avec
chaque pair jamais croisé ; on n'évince jamais les partis (on a borné les *seaux* au
6.5, pas ceux-là). Sur une longue session avec du va-et-vient → la mémoire enfle.
*Piste :* TTL / éviction des pairs absents depuis longtemps (lié à D8 pour la réputation).
*Vérif :* sur une simu longue avec fort renouvellement, la mémoire se stabilise.

**D17 — NAT symétrique = repli forcé sur relais (et donc sur D3/D4).** 🟠 `[ch. 8/12]`
*Constat :* le hole punching échoue sur NAT symétrique (CGNAT mobile). Une fraction
réelle des joueurs ne pourra pas se connecter en direct → relais obligatoire → toute la
promesse P2P dépend de l'économie du parent (D4). *Piste :* relais TURN **décentralisés**
(chaque bon nœud peut en héberger un), priorité IPv6 (plus de NAT du tout). *Vérif :* le
cas symétrique de `tools/test-nat.sh` réussit via un relais.

**D18 — Le seuil de speed-hack (30 m/s) est grossier.** 🟡 `[ch. 11/12]`
*Constat :* un tricheur subtil reste juste sous 30 m/s (plus rapide qu'un honnête, mais
non détecté) ; pas de borne d'accélération ni de téléport-sur-longue-absence. *Piste :*
bornes plus fines (vitesse + accélération + cohérence avec la vitesse annoncée `vx,vy,vz`),
calibrées sur la vraie vitesse du jeu. *Vérif :* un « speed-hack discret » à 29 m/s est
repéré comme statistiquement anormal.

### Catégorie 8 — Méta-doutes (sur la démarche)

**D19 — On n'a jamais mesuré le coût RÉEL par nœud (CPU, RAM, bande passante).** 🟠 `[ch. 7]`
*Constat :* la simu dit « ça tient » mais ne chiffre pas Ko/s ↑↓, % CPU, Mo RAM par nœud.
*Pourquoi :* sans ces chiffres, « 55K » reste une intuition. *Piste :* instrumenter `sim`
pour rapporter ces métriques par nœud. *Vérif :* on peut dire « un nœud = X Ko/s ↑, Y
Ko/s ↓, Z % CPU » → on extrapole honnêtement vers 55K.

**D20 — Attaques combinées / adaptatives jamais testées.** 🟠 `[ch. 9]`
*Constat :* nos attaques sont jouées isolément. Un vrai adversaire combine (Sybil +
éclipse + framing), s'adapte, joue honnête puis trahit. *Piste :* un mode d'attaque
« scénario » qui enchaîne et coordonne plusieurs vecteurs. *Vérif :* l'essaim tient face
à un scénario combiné, pas juste face à une attaque pure.

**D21 — La sécurité du rendez-vous lui-même.** 🟠 `[ch. 9]`
*Constat :* le rendez-vous accepte tout HELLO valide sans rate-limit ; sa table `clients`
grossit sans borne ; il n'a pas la machinerie anti-DoS des clients. *Piste :* lui donner
rate-limit + éviction + PoW à l'entrée (déjà le `has_pow`, mais pas le débit). *Vérif :*
inonder le rendez-vous ne le met pas à genoux.

---

## D. Le programme (les chapitres à venir)

> Chaque chapitre liste : **But**, **sous-étapes** (à cocher comme au chap. 6),
> **doutes fermés**, **méthode de vérification**. On garde la discipline du chap. 6 :
> petites étapes, chacune compilée + testée + prouvée (souvent en headless/simu), puis
> commitée et écrite ici.

### Chapitre 7 — Confrontation au réel (latence, perte, NAT) 🔴 *priorité 1*
**But :** arrêter de mentir comme localhost. Mesurer la vérité.
- [x] 7.1 — Harnais `tc netem` sur `lo` : script `tools/sim-netem.sh` (3 profils
  `bon|moyen|mauvais`) qui applique latence/jitter/perte/ré-ordonnancement, lance `sim`,
  et **retire toujours** le netem à la sortie (`trap`, comme `sim-cool.sh`). *(fait)*
  **Piège tranché :** sur `lo`, le délai compte DOUBLE (aller + retour sur la même
  interface) → les profils sont exprimés en PING cible et le script applique `delay =
  ping ÷ 2`. **Prouvé réel** (profil `moyen`, ~120 ms + 2 % perte) : netem posé → simu
  → `lo` revenu à `noqueue` ; l'essaim a TENU (20/20 montés, voisins moy 21/plafond 32,
  ~5680 paquets honnêtes/s, orbe 0/20 volée, orb-creep mis en SOURDINE).
  > À creuser au 7.2 : le `teleport` n'a laissé **aucune** trace (0 paquet de triche
  > rejeté, 0 faute) — rejoint trop tard, perte netem, ou angle mort réel ?
- [ ] 7.2 — Faire tourner la simu sous 3 profils réseau (bon / moyen / mauvais : 30/120/250
  ms, 0/2/5 % perte, jitter) et **mesurer** : orbe intègre ? fausses migrations ? débit
  honnête stable ? rejets anti-rejeu dus au ré-ordonnancement ?
- [ ] 7.3 — Corriger ce que netem révèle (très probable : l'anti-rejeu strict casse sur
  paquets re-ordonnés → fenêtre de tolérance ; la prédiction/migration à régler).
- [ ] 7.4 — Instrumenter `sim` : Ko/s ↑↓, CPU, RAM **par nœud** (ferme D19).
- [ ] 7.5 — NAT : généraliser `tools/test-nat.sh` au scénario multi-joueurs.
**Ferme :** D1, D19 (et révèle des correctifs réseau réels). **Vérif :** rapport de simu
sous netem montrant que l'essaim tient avec de *vrais* défauts réseau.

### Chapitre 8 — Inclusivité & adaptation au lien (0 → 2 Gb/s) 🔴 *priorité 2 — ta vision*
**But :** que le 0-connexion et le 2-Gb/s aient chacun LA meilleure expérience.
- [ ] 8.1 — **Budget de réception annoncé** : chaque joueur publie son débit descendant
  soutenable + son rayon d'intérêt ; les émetteurs en tiennent compte (water-filling
  **bilatéral**). Ferme D3.
- [ ] 8.2 — **Dégradation gracieuse** : au-delà du budget, on baisse la fréquence des
  lointains avant les proches (paliers focus / proche / foule).
- [ ] 8.3 — **Parent agrégateur** pour très faibles : le parent reçoit le voisinage et
  n'envoie au protégé qu'un résumé basse fréquence. Le 0-connexion joue *via* son parent.
- [ ] 8.4 — **Économie du parent (anti free-riding)** : réciprocité façon BitTorrent
  (choking/optimistic unchoke pondéré par la réputation). Ferme D4.
- [ ] 8.5 — **Anti-censure du parent** : multi-parents + détection du « trou noir ».
  Ferme D5.
**Ferme :** D3, D4, D5, (amorce D17). **Vérif :** sous netem throttlé à 5 Ko/s, le joueur
reste fonctionnel ; un nœud égoïste est servi en dégradé.

### Chapitre 9 — Durcissement de la confiance (Sybil, éclipse, rendez-vous) 🔴 *priorité 3*
**But :** rendre la triche *coordonnée* coûteuse et l'isolement impossible.
- [ ] 9.1 — **Refonte anti-Sybil** : difficulté PoW bien plus haute + adaptative ; étude
  d'un second facteur (vouching social). Ferme D6.
- [ ] 9.2 — **Quorum d'accusation pondéré** : par réputation de l'accusateur + plausibilité
  de voisinage ; K attaquants ne peuvent pas framer un honnête. Ferme D7, D20.
- [ ] 9.3 — **Réhabilitation** : fenêtre glissante des fautes + appel/quarantaine. Ferme D8.
- [ ] 9.4 — **Anti-éclipse** : diversité forcée du voisinage (proches + aléatoires
  vérifiés, façon Kademlia) + corroboration des positions. Ferme D9.
- [ ] 9.5 — **Rendez-vous résilient** : rate-limit + éviction + fédération (2+ rendez-vous
  qui s'échangent des pairs) ; amorce d'une découverte par gossip. Ferme D10, D21.
**Ferme :** D6, D7, D8, D9, D10, D20, D21. **Vérif :** scénario d'attaque combinée en simu
(Sybil + éclipse + framing) → l'essaim tient.

### Chapitre 10 — Identité persistante & vie privée 🔴 *priorité 4*
**But :** un vrai « compte » décentralisé, et de la confidentialité.
- [ ] 10.1 — **Identité persistante** : clé sauvée (chiffrée) sur disque, rechargée au
  lancement ; pseudo + avatar liés à la clé. Ferme D14.
- [ ] 10.2 — **Chiffrement de transport** : échange X25519 par paire → contenu chiffré +
  signé. Ferme D15.
**Ferme :** D14, D15. **Vérif :** relancer garde l'identité & la réputation ; un tiers ne
peut plus lire positions/voix.

### Chapitre 11 — Autorité généralisée & ordre 🟠
**But :** sortir de l'orbe-unique, durcir la migration, arbitrer le temps.
- [ ] 11.1 — **Registre d'objets autoritaires** générique (réutilise la machinerie orbe).
  Ferme D12.
- [ ] 11.2 — **Migration par quorum** (élection confirmée + présence prouvée + grâce).
  Ferme D11.
- [ ] 11.3 — **Horodatage signé + synchro d'horloge légère** pour arbitrer les courses.
  Ferme D13.
- [ ] 11.4 — **Anti speed-hack fin** (vitesse + accélération + cohérence). Ferme D18.
**Ferme :** D11, D12, D13, D18. **Vérif :** 1000 objets + courses arbitrées de façon
cohérente par tous les nœuds.

### Chapitre 12 — Robustesse, longévité, unification 🟠
**But :** que ça tienne des heures, et que le code ne dérive pas.
- [ ] 12.1 — **Éviction/TTL** des pairs absents (mémoire stable). Ferme D16.
- [ ] 12.2 — **Unifier bot/jeu** : un cœur de session partagé. Ferme D2.
- [ ] 12.3 — **Relais TURN décentralisés + IPv6** (NAT symétrique). Ferme D17.
**Ferme :** D2, D16, D17. **Vérif :** simu longue (mémoire stable) ; cas NAT symétrique OK.

### Chapitre 13 — Voix spatiale
**But :** chat vocal P2P, priorité au volume (loudness priority), spatialisé. Profite du
chiffrement (10.2) et de l'inclusivité (ch. 8 — la voix s'adapte au lien).

### Chapitre 14 (plus tard, pas maintenant) — Portabilité moteurs
**But :** extraire un `net-core` portable (ABI C) pour Unreal/Unity. **Décidé : reporté.**

---

## E. Comment tester POUR DE VRAI avec une seule machine (ta question D1)

Tu as dit « on n'a qu'une seule machine, je ne sais pas comment faire ». **Bonne
nouvelle : une seule machine suffit.** Linux sait simuler une mauvaise connexion :

- **`tc netem`** (sur l'interface loopback `lo`) ajoute **latence, jitter, perte,
  ré-ordonnancement** à TOUT le trafic localhost. On lance notre `sim` derrière, et nos
  centaines de nœuds se parlent soudain « comme sur Internet ». C'est exactement fait
  pour ça. (Au chap. 7 je t'écris `tools/sim-netem.sh` qui l'applique puis le retire
  proprement, comme `sim-cool.sh` le fait pour les ventilos.)
- **`tc tbf`** limite le débit (pour simuler les « quelques Ko/s »).
- **`ip netns`** (namespaces réseau) crée de « fausses machines » isolées derrière de
  « faux NAT » sur le même PC — c'est déjà ce que fait `tools/test-nat.sh`.

Donc : **on n'a pas besoin de 2 machines** pour confronter au réel. Une seule + netem =
un labo réseau complet. C'est ça qui transforme nos tests « localhost » en vraie preuve.

---

## F. Ordre conseillé & priorités

1. **Chapitre 7 d'abord** (réalisme/netem) — sans ça, tout le reste est de la confiance,
   pas de la preuve. Et il va probablement révéler des bugs réseau réels à corriger.
2. **Chapitre 8 ensuite** (inclusivité) — c'est le cœur de ta vision « tout le monde
   peut jouer », et ça dépend de mesures réalistes (donc après 7).
3. **Chapitre 9** (confiance dure) — referme les attaques *coordonnées*, les vraies.
4. **Chapitre 10** (identité persistante + chiffrement) — indispensable pour de vrais
   utilisateurs.
5. **Chapitres 11–12** (autorité généralisée, robustesse) — élargir et durcir.
6. **Chapitre 13** (voix), puis **14** (moteurs, plus tard).

> Note : 7 → 8 → 9 → 10 est le chemin « solide ». Mais si un jour tu veux du *visible*
> vite (pour le moral), 8.3 (le 0-connexion qui joue via un parent) est très
> spectaculaire. À toi de doser preuve vs effet.

---

## G. Décisions PRISES (juin 2026)

Ces choix ont été tranchés avec l'utilisateur. Ils orientent le plan :

1. **Vie privée → ON CHIFFRE TOUT.** Direction ferme : positions + voix chiffrées.
   Décidé par l'utilisateur (« en vrai il faudrait tout chiffrer »). Réalisé au
   **chapitre 10.2** (X25519 par paire) — pas avant, la base passe d'abord.
2. **Anti-Sybil → preuve de travail RÉGLABLE.** On garde la PoW, on rend sa difficulté
   ajustable, et on ne l'augmente QUE si les tests montrent que les attaquants gagnent.
   Le *vouching* social reste en réserve (chap. 9.1).
3. **0-connexion → on suit l'ordre normal.** Pas de priorité forcée : chapitre 7
   (mesurer) puis chapitre 8 (inclusivité, déjà priorité 2) ; le 0-connexion (8.3) vient
   dans le 8, après ses briques de base.
4. **Identité persistante → OUI, clé sauvée dans un fichier** (comme une clé SSH).
   Simple d'abord ; protection par mot de passe plus tard, avec le chiffrement du
   chapitre 10. Réalisé au **chapitre 10.1**.

---

*Ce document est vivant : on coche les sous-étapes et on l'enrichit au fil de l'eau,
exactement comme on l'a fait pour le chapitre 6 dans le README.*
