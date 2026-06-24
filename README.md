# web3game — un espace social en pair-à-pair (P2P)

[![tests](https://github.com/shazamifius/web3game/actions/workflows/tests.yml/badge.svg)](https://github.com/shazamifius/web3game/actions/workflows/tests.yml)

> **Statut : R&D personnelle, solo, très expérimentale. Work in progress.**
> Rien n'est prouvé à grande échelle. L'état honnête et détaillé vit dans
> [`docs/ETAT.md`](docs/ETAT.md) ; le plan complet dans [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md) (un index).

## C'est quoi ?

Une R&D pour bâtir une **plateforme de jeux en pair-à-pair** : on rejoindrait des **mondes partagés
sans aucun serveur de jeu central** — chaque joueur est un nœud du réseau. La direction visée (façon
**gamejolt**, pas Roblox ni VRChat) : un launcher qui héberge des mondes faits dans **n'importe quel
moteur** (Unreal/Unity/Godot), qu'on découvre dans un hub 3D et que **n'importe qui** pourra un jour
publier. Le **cœur réseau P2P** (ce dépôt, en Rust) est le **liant commun** à tous ces mondes.

Le **tout premier monde** est un **espace social** (dans l'esprit de VRChat, mais P2P) : c'est par lui
qu'on teste si « ça vit » vraiment, à plusieurs, et qu'on débusque les bugs.

La vraie question qu'explore ce dépôt : **le « web3 » au sens décentralisé — pas de serveur central,
une identité qu'on possède vraiment — peut-il *réellement* fonctionner pour un monde vivant à plusieurs,
ou n'est-ce qu'une utopie ?** On ne répond pas par des slogans : on l'écrit en **Rust**, octet par octet,
on l'attaque nous-mêmes, et on tient un inventaire honnête de ce qui tient et de ce qui ne tient pas.

## L'architecture P2P, en clair

Tout le réseau est **fait main, sans boîte noire** (la seule dépendance « magique » est la
bibliothèque de cryptographie). Quatre idées suffisent à comprendre :

- **Ton identité = ta clé.** Pas de compte sur un serveur : ton identité est une **clé
  cryptographique** que tu gardes (comme une clé SSH). Chaque message que tu envoies est **signé** ;
  personne ne peut se faire passer pour toi, et aucun annuaire central ne « décide » qui tu es.
- **Pas de serveur de jeu.** Les joueurs s'envoient leurs positions **directement** les uns aux
  autres. Un petit serveur de *rendez-vous* sert juste à faire les **présentations** (percer les
  box/NAT pour ouvrir une connexion directe) ; une fois les présentations faites, on pourrait
  l'éteindre, la partie continue.
- **On ne parle qu'à ses voisins (AoI).** Si chacun parlait à tout le monde, ça exploserait à
  grande échelle. Donc chacun n'échange à plein débit qu'avec un **petit voisinage** (~32), et
  perçoit la **foule lointaine** à basse fidélité — coût borné, indépendant du nombre total.
- **« Own + Shields » pour les objets partagés.** Pour tout objet contesté (une balle, une porte),
  **un seul** joueur fait autorité à un instant donné (l'« Own ») ; les autres vérifient et peuvent
  le **destituer** s'il triche. L'autorité **migre** si l'Own part. Pas de serveur arbitre.

> Le cœur réseau est en **Rust pur**, **sans aucun moteur 3D**. La **présentation** se fait dans
> **Unreal Engine** (un client mince branché au cœur par une socket locale, le *sidecar*). La
> *logique* réseau, elle, resterait la même quel que soit le moteur.

## Où on va (la boussole)

- **À terme :** réunir un **énorme événement en P2P sans serveur** — la boussole est **~55 000
  personnes** (la jauge de la plus grande salle de concert au monde), un nombre jamais réuni dans un
  seul espace de jeu. *C'est une boussole, pas une échéance* : on bâtit l'archi qui *pourrait* y aller.
- **La plateforme visée** (vision long terme) : un launcher façon **gamejolt** qui embarque plusieurs
  moteurs (Unreal/Unity/Godot) ; on navigue dans un hub 3D parmi des jeux que **n'importe qui** pourra
  un jour créer. Détail : [`docs/VISION.md`](docs/VISION.md).
- **Tout de suite :** un **premier petit jeu** simple, vite installable, jouable à quelques amis sur
  la même map — pour récolter un maximum de retours, bugs et failles. (« L'île aux étoiles » :
  ramasser des étoiles, des cristaux, faire évoluer son avatar… *lentement*, pour pousser à se parler.)

## Ce qui est prouvé / pas encore

Par honnêteté (c'est une règle du projet), sans rien arrondir — détail dans [`docs/ETAT.md`](docs/ETAT.md)
et [`docs/DOUTES.md`](docs/DOUTES.md) :

- **✅ Prouvé** (mesuré, tests à l'appui) : identité = clé + états signés (anti-usurpation/rejeu) ;
  **hole-punching NAT réel** (+ relais pour les NAT symétriques, vérifié en mobile réel) ; anti-triche
  (Sybil-framing, gossip-flood, orbe) ; **perception de foule** restaurée à ~87 % à 1000 nœuds, à débit
  reçu plat ; identité persistante ; présence « vivante » des avatars dans Unreal.
- **⚠️ Pas encore** : **pas « 55 000 prouvé »** (mesuré jusqu'à ~1000-2000, au-delà = extrapolation) ;
  « sans serveur » garde un astérisque (l'amorçage passe par un rendez-vous) ; positions en clair (pas
  encore de chiffrement bout-à-bout) ; **chat vocal de proximité** pas encore là.

## Pour aller plus loin

- **L'état courant + la prochaine action** : [`docs/ETAT.md`](docs/ETAT.md) ⭐
- **Le plan complet** (index de tous les docs) : [`FEUILLE_DE_ROUTE.md`](FEUILLE_DE_ROUTE.md)
- **Comment lancer & tester** : [`docs/TESTS.md`](docs/TESTS.md)
- **L'architecture & l'organisation du code** : [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)

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

---

*Licence : Tous droits réservés (voir [`LICENSE`](LICENSE)).*
