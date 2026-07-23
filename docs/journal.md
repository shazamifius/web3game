# Journal de développement — comment le cœur a été bâti

> L'histoire du projet, étape par étape : du bac à sable 3D au cœur réseau durci. On a avancé **en codant pour de
> vrai**, du plus simple (deux machines qui se parlent) au plus dur (des centaines de joueurs, l'anti-triche).
> Chaque étape a été compilée, testée et prouvée avant la suivante.
>
> Voir : [l'idée en clair](comprendre-le-p2p.md) · [l'état chiffré](etat-du-projet.md) · [les doutes](doutes.md).

---

## Chapitre 0 — Le bac à sable 3D
Une salle, un personnage articulé, une vue à la première personne. Le terrain de jeu minimal sur lequel tout le
reste va se construire.

## Chapitre 1 — Le transport brut
UDP fait main : encoder une position en octets, l'envoyer, la recevoir. Deux fenêtres se voient bouger.
L'orientation (corps + tête) et une couleur voyagent dans le paquet.

## Chapitre 2 — Le netcode : fluidité et prédiction
Le cœur du « ça bouge bien » malgré le réseau. Les positions arrivent ~20 fois par seconde ; chacune est rangée
dans une file d'instantanés horodatés, et l'avatar est dessiné **~100 ms dans le passé** (retard d'interpolation),
en glissant entre les deux instantanés qui l'entourent. Quand un paquet manque, on **prédit** la suite par
extrapolation de la vitesse plutôt que de figer l'avatar, puis on **réconcilie** en douceur (ressort amorti) quand
le vrai paquet arrive. Chaque avatar a même sa propre horloge de lecture, qui accélère ou ralentit un peu pour
rattraper **en marchant** plutôt qu'en téléportant.

> Note de conception : la prédiction est faite « à la main » — la physique de l'inertie humaine suffit sur 100 ms,
> c'est déterministe, lisible, et gratuit en calcul (pas besoin d'un réseau de neurones pour ça).

## Chapitre 3 — La topologie et le passage à l'échelle
- **N joueurs + un point de rendez-vous** : un annuaire présente les joueurs ; chacun s'inscrit, puis envoie son
  état **directement** aux autres.
- **NAT & hole-punching** : la box jette les paquets entrants non sollicités, mais *envoyer* ouvre un « trou de
  retour ». Les deux pairs s'envoient donc une salve simultanée : les premiers paquets meurent, les suivants passent
  → **connexion directe, sans relais**.
- **L'aire d'intérêt par budget** : on ne coupe jamais personne par règle ; on **répartit un budget d'émission**
  entre les pairs selon leur pertinence. Peu de monde → plein débit pour tous ; foule → dégradation douce, jamais zéro.

## Chapitre 4 — L'autorité et la migration
- **Un premier objet partagé** qui n'appartient à personne : le dernier à le toucher en devient le **maître** (il
  simule sa physique et la diffuse), et l'autorité **saute de main en main**. Les conflits se règlent par un couple
  (version, identité) — départage **déterministe**, sans serveur.
- **La migration** : si le maître se tait, chacun élit le **même** remplaçant déterministe, sans voter ; un éventuel
  « split-brain » se résout tout seul.
- **Le relais (« parent »)** : un joueur à faible débit montant envoie son état une seule fois à un parent, qui le
  recopie à ses voisins — l'identité reste celle de l'auteur. *Le relais porte les octets, il n'arbitre pas* : deux
  rôles distincts.

## Chapitre 5 — La confiance et l'anti-triche (les fondations)
- **Identité signée** : chaque session a une paire de clés ; tout état est **signé** et vérifié. On ne peut plus se
  faire passer pour un autre, ni falsifier l'état qu'on relaie. *(La cryptographie vit dans un seul fichier — la
  seule « boîte noire » assumée ; on ne code jamais sa propre crypto.)*
- **Anti-rejeu** : un compteur monotone empêche de rejouer un vieux paquet.
- **Orbe signée + bornes** : le maître signe l'objet, et un saut de version aberrant (tentative de verrouillage à
  vie) est refusé **et** compté comme faute.
- **Réputation locale** : un tricheur attribuable est mis en sourdine. Règle clé anti-*framing* : on n'accuse
  **jamais** sur une signature invalide (non attribuable), seulement sur un paquet **valablement signé mais
  trichant**.
- **Un vrai harnais d'attaque** : un programme adverse, sur de vraies sockets, qui prouve la robustesse.

## Chapitre 6 — La refonte « béton » : durcissement intégral
On a repris chaque morceau pour fermer le fossé entre « quelques attaques connues neutralisées » et l'objectif
réel : une foule en pair-à-pair pur, face à un maximum d'adversaires, **qui tient**.

> *Cadre honnête posé d'emblée :* le P2P sans serveur à cette échelle, face à des adversaires byzantins, est à la
> frontière de la recherche — on ne promet pas l'inviolable absolu (ça n'existe pas). On vise : **chaque attaque
> devient soit impossible, soit chère, soit attribuable et bannie.**

Dix « trous » ont été fermés ou bornés, dont les plus structurants :

- **L'identité auto-certifiante** (le keystone) : l'identité **est** la clé publique, portée dans chaque paquet ; le
  rendez-vous **ne peut plus mentir** sur « telle clé = tel joueur ». Au passage, le mur des 255 joueurs et les
  collisions d'identité disparaissent.
- **Un coût d'entrée anti-Sybil** (preuve de travail) : une identité n'est valide que « minée » → un banni ne
  revient plus gratuitement.
- **La validation de mouvement** : un téléport (distance et temps incohérents) est refusé et compté comme faute.
- **La preuve de contact** : pour devenir maître d'un objet, il faut avoir été **près** de lui.
- **Le DoS borné** : mémoire plafonnée (éviction), amplification de relais plafonnée.
- **Le voisinage borné** (~32 plus proches) : c'est *la* borne qui rend l'échelle possible — des milliers de petits
  voisinages plutôt qu'un maillage géant.
- **La réputation partagée** : un tricheur attribuable est banni par **quorum** d'accusateurs distincts (chacun
  coûtant une preuve de travail) → fabriquer un faux quorum est cher, et un menteur isolé ne peut rien.
- **Une simulation massive** : 50 puis 300 nœuds + une nuée d'attaquants sur une seule machine → tous montés,
  voisinage plafonné à 32, **objet partagé jamais volé**. *Ce qui vaut pour une grande échelle : la charge par nœud
  ne dépend pas du nombre total — la vraie échelle se fait en ajoutant des machines, pas en surchargeant une seule.*

## Chapitre 7 — Dehors : l'instrument de mesure et les premiers nœuds distants réels
Jusqu'ici, toutes les preuves venaient de la **simulation** ou de **deux machines**. Or le doute fondateur du projet
— la « forteresse vide » — ne se lève pas comme ça : il demande de **vrais gens, sur de vrais réseaux, à de vraies
distances**.

On a donc bâti un **instrument de mesure** : un petit **agent autonome** (sans aucune dépendance, comme le cœur)
qu'un volontaire lance chez lui. Il rejoint le réseau, mesure la **vivacité des liens distants** qu'il perçoit —
fraîcheur, perte, gigue, ré-ordonnancement — et **renvoie les chiffres**. L'instrument est lui-même tenu d'être
**honnête** : visible, lancé en connaissance de cause, et au repos basse consommation quand il ne mesure pas.

**Une fausse piste, d'abord — et la plus instructive.** Les premiers relevés annonçaient **89 % de perte** sur le
chemin relais. Panique légitime… puis enquête : ce n'était **pas** une panne réseau, mais **notre propre économie de
bande passante, mal mesurée** (l'instrument comparait le « filet » basse-fréquence, envoyé exprès aux pairs
lointains, au plein débit réservé au voisinage proche). On a appris à l'instrument à **distinguer « pas envoyé
exprès » de « envoyé puis perdu »**. L'histoire complète vit dans [les coulisses](coulisses.md).

**Le résultat, une fois l'instrument honnête.** Des volontaires répartis sur **plusieurs pays**, sur de **vrais
réseaux domestiques** (dont certains derrière le NAT le plus dur, le CGNAT), ont été mesurés **vivants** : fraîcheur
**p95 ~200–335 ms** — sous le seuil de **500 ms** qu'on s'est fixé pour « vivant » —, **perte réelle ~0**, verdict
« vivant ».

**Ce que ça prouve, et ce que ça ne prouve pas.** C'est un **fait dur** : le **substrat** transporte de la présence
**distante et réelle**, vivante, sur le vrai Internet — l'infrastructure n'est plus *vide*. Mais ça ne prouve **pas
encore** le **ressenti** : des humains qui **bougent et jouent ensemble** dans le même monde, et le **sentent**
vivant. Ce test-là — le plus important — reste devant. Le doute s'allège ; il ne se ferme pas.

## Chapitre 8 — Le réseau apprend à connaître ses liens
Le chapitre 7 avait posé l'instrument dehors et levé une fausse alerte. Restait une vraie question, soulevée par un
lien de test médiocre : **la redondance d'émission** (envoyer un état en double pour résister à la perte) a, sur ce
lien-là, *empiré* les choses au lieu d'aider. Pourquoi ?

La réponse a demandé d'apprendre au réseau à **se connaître lui-même**. On a écrit une **sonde de lien** (sans aucune
dépendance, comme le reste) qui, sur chaque nœud, mesure : le **type de NAT** (perçable ou non, en interrogeant deux
serveurs publics depuis une seule socket), la **latence**, la **gigue**, et surtout la **nature de la perte** — une
courte rafale à débit croissant révèle si le lien *sature* (congestion) ou s'il perd *au hasard*. Première surprise,
et leçon de méthode : un téléphone mobile grand public qu'on croyait « bloqué » (NAT symétrique) s'est révélé
**perçable**. La mesure a, une fois de plus, corrigé l'intuition.

Surtout, la sonde explique la redondance ratée : ce lien mobile ne perdait pas *au hasard*, il **saturait**. Or
dupliquer un état sur un lien saturé, c'est doubler le trafic d'un tuyau déjà plein → on aggrave. La redondance
n'aide que la perte *aléatoire*. D'où la réponse : une **redondance ADAPTATIVE** — chaque nœud lit sa propre sonde et
ne dédouble que s'il voit une perte aléatoire avec de la marge, **jamais** sur un lien qui sature. On l'a vu se
produire en vrai (un lien congestionné a renoncé seul à dupliquer) ; et on a prouvé l'autre moitié sur un banc à
perte **aléatoire** contrôlée (perte réseau injectée par le noyau) : **30 % → 9 %** à deux copies, **2,8 %** à trois
— la perte divisée comme la théorie le prédit, sur de vrais paquets.

Le récit complet de cette double correction vit dans [les coulisses](coulisses.md) (enquête n°3) ; le détail
technique, dans le [chantier réseau](chantier-reseau.md).

## La suite
À partir de là, le travail se poursuit dans les chantiers dédiés : confronter le tout au [réseau réel](chantier-reseau.md),
[la foule dense](chantier-foule.md), et [la robustesse](chantier-robustesse.md).

---

*Histoire tenue à jour au fil du développement.*

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Le journal de développement** — une étape du parcours **🧭 Tout comprendre** (la suite naturelle des [coulisses](coulisses.md)).

**Continuer le fil :**
- 🧭 *Tout comprendre* → **[Architecture & code](ARCHITECTURE.md)**

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
