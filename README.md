# web3game

**Un moteur pair-à-pair pour des mondes partagés massifs — sans serveur de jeu central.**

[![tests](https://github.com/shazamifius/web3game/actions/workflows/tests.yml/badge.svg)](https://github.com/shazamifius/web3game/actions/workflows/tests.yml)

> Un univers de mondes partagés où l'on se retrouve à plusieurs — **sans serveur central**, avec une **identité
> qu'on possède vraiment**. Une infrastructure réseau écrite à la main, attaquée par nous-mêmes, et documentée
> sans rien arrondir.

---

> **Ce dépôt n'est ni un produit, ni une promesse.** C'est un projet de **R&D — poussé et de niche —** documenté
> honnêtement : ce qui marche, ce qui résiste, et **surtout les doutes encore ouverts.** On ne vend rien ; on explore.

## La vision

L'objectif, lointain et assumé : **un univers de mondes partagés massifs**, dans l'esprit de l'**OASIS de
*Ready Player One*** (le métavers du roman puis du film de Spielberg) — mais **en pair-à-pair**. Un monde où l'on
retrouve ses amis, où l'on passe d'un jeu à l'autre, et que **n'importe qui pourra un jour enrichir du sien**
(Unreal, Unity, Godot…).

La différence, c'est l'**architecture** : pas de serveur de jeu central qui fait autorité. Les joueurs forment
eux-mêmes le réseau. Ce que ça change concrètement :

- **Pas de coût serveur qui explose avec le succès** — l'infrastructure, ce sont les joueurs.
- **Une identité que tu possèdes** — une clé cryptographique (comme une clé SSH), pas un compte sur la machine
  d'un tiers. *« web3 » est ici à prendre au sens **décentralisé / identité possédée** — **pas** un token, pas de
  crypto (en clair : [le « sans serveur »](docs/comprendre-le-p2p.md)).*
- **Pas de point de défaillance unique** qui éteint tout le monde d'un coup.

La **boussole d'échelle** est volontairement vertigineuse. Le plus grand affrontement coordonné de l'histoire du
jeu vidéo — un **record du monde Guinness** détenu par la communauté d'**EVE Online** (la bataille de *B-R5RB*, 2014) —
a réuni des **milliers** de joueurs dans un même espace : la preuve qu'une présence partagée à très grande échelle
est possible. Ce projet en explore la **frontière, en pair-à-pair**, avec pour cap lointain l'ordre de grandeur d'une
très grande foule (~55 000, une salle de concert). *C'est une **direction de recherche**, pas une métrique déjà
atteinte (voir « En toute transparence »).*

## Ce qui est déjà construit (et éprouvé)

Le cœur est du code, en **Rust**, **fait main, sans boîte noire** (la seule dépendance externe est la bibliothèque
de cryptographie). Ce qui suit a été écrit puis vérifié — en distinguant honnêtement ce qui est prouvé de ce qui ne
l'est pas encore (cf. *En toute transparence*). **118 tests automatiques, 0 warning** ; chaque chiffre est
**reproductible** (détail mesuré + commandes : [revue chiffrée](docs/etat-du-projet.md)) :

- **Identité = ta clé.** Chaque message est signé : impossible de se faire passer pour un autre, aucun annuaire
  central ne décide qui tu es. Identité **persistante** entre sessions (comme un vrai compte, mais à toi).
- **Traversée NAT réelle, jusqu'au cas le plus dur.** Deux humains derrière leurs box se connectent en direct ;
  et quand c'est impossible (NAT symétrique mobile 4G/5G), un **relais** prend le relais — **prouvé entre deux
  vrais réseaux sur Internet**, pas en laboratoire.
- **Résistance aux attaques, testée pour de vrai.** Des simulations d'attaques (Sybil, éclipse, *framing*,
  inondation de gossip) sont jouées contre le réseau : l'essaim tient, les tricheurs sont mis en sourdine.
- **Perception de foule à coût borné.** Chacun ne dialogue à plein débit qu'avec un **petit voisinage** (~32) et
  perçoit la foule lointaine en basse fidélité — le coût reste **borné, indépendant du nombre total** : **~34 Ko/s
  par nœud (~0,27 Mbit/s)**. En simulation, la perception est restaurée à **~87 % à 1 000 nœuds, à débit reçu plat.**
- **Indépendant du moteur 3D.** Le cœur réseau est **agnostique** : la preuve en a été faite en réunissant **deux
  moteurs différents (Bevy et Unreal) dans le même espace partagé** via un pont local (*sidecar*). C'est ce qui
  rend la plateforme multi-moteur crédible.
- **Présence vivante.** Les avatars distants bougent de façon fluide et « habitée » dans Unreal (interpolation +
  vie procédurale), même sous perte de paquets.

> Le cœur réseau est en **Rust pur, sans aucun moteur 3D**. La présentation se fait dans **Unreal Engine** (un
> client léger branché par une socket locale). La logique réseau resterait la même avec n'importe quel moteur.

## La feuille de route

- **🟢 Phase 1 — le cœur réseau : faite.** Crypto/identité, découverte, traversée NAT + relais, anti-triche,
  architecture de foule, pont multi-moteur. La fondation tient.
- **🟡 Phase 2 — le passage à l'échelle : en cours.** Rendre la foule réellement *perçue et vivante*, durcir la
  robustesse face aux tricheurs, **jusqu'à un premier jeu jouable et partageable**.
- **⚪ Phase 3 — la plateforme.** Le hub, les mondes téléchargeables, l'ouverture à des créateurs tiers — l'OASIS.

## En toute transparence

C'est une **règle du projet** : on écrit ce qui est prouvé **et** ce qui ne l'est pas. La crédibilité vient de là.

- **L'échelle « 55 000 » n'est pas encore mesurée directement** : les coûts sont mesurés jusqu'à ~1 000–2 000
  nœuds en simulation ; au-delà, c'est de l'architecture + de l'extrapolation, pas une preuve.
- **« Sans serveur » garde un astérisque honnête** : l'amorçage passe encore par un point de rendez-vous (pour les
  présentations uniquement) ; la décentralisation complète de cette dernière brique est un chantier à venir.
- **Le chiffrement bout-à-bout n'est pas encore là** : aujourd'hui les positions circulent en clair (la signature
  garantit l'authenticité, pas le secret) ; c'est planifié.
- **Le test décisif reste à faire dehors, avec de vrais joueurs** : prouver que l'espace est *vivant* à plusieurs.

## Conçu avec l'IA — assumé, et revendiqué

Ce projet est imaginé et construit par son auteur **en binôme avec Claude** (l'IA d'Anthropic). Ce n'est pas un
détail qu'on cache — c'est une partie de l'histoire : l'IA est le **levier** qui permet à un fondateur seul de
concevoir et d'écrire, octet par octet, une infrastructure réseau de cette ambition. Chaque ligne est **relue,
comprise et assumée par un humain** — mais ce projet, dans cette forme et à ce rythme, **n'aurait pas existé sans
cette collaboration.** Le dire clairement, c'est cohérent avec la règle n°1 du dépôt : **l'honnêteté avant tout.**

## Pour aller plus loin

- **Le « sans serveur », en clair** : [`docs/comprendre-le-p2p.md`](docs/comprendre-le-p2p.md)
- **L'état du projet — réalisations, combats, murs, doutes (chiffré)** : [`docs/etat-du-projet.md`](docs/etat-du-projet.md)
- **Le registre des doutes (le cœur de la démarche)** : [`docs/doutes.md`](docs/doutes.md)
- **L'architecture & l'organisation du code** : [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- **Comment lancer & tester** : [`docs/TESTS.md`](docs/TESTS.md)

---

*Auteur : [shazamifius](https://github.com/shazamifius). Licence : **Tous droits réservés** (voir [`LICENSE`](LICENSE)).*
