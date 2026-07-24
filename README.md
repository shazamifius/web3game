# web3game

**Un moteur pair-à-pair pour des mondes partagés massifs — sans serveur de jeu central.**

*[English version](README.en.md)*

> Le code de ce projet est **privé**. Cette page existe pour le présenter honnêtement, et pour que
> vous puissiez **demander à le voir**. La démarche est décrite plus bas — elle prend deux minutes.

---

Un univers de mondes partagés où l'on se retrouve à plusieurs, sans serveur central, avec une identité
qu'on possède vraiment. Une infrastructure réseau écrite à la main, attaquée par nous-mêmes, et documentée
sans rien arrondir.

**Ce n'est ni un produit, ni une promesse.** C'est un projet de R&D — poussé et de niche — mené par une
personne seule. Ce qui suit distingue systématiquement ce qui est mesuré de ce qui ne l'est pas.

## Par où commencer

Deux chemins, selon ce que vous cherchez — et rien n'empêche de faire les deux :

- **[Lire la documentation, librement](#la-documentation-en-libre-accès)** — l'idée, les mesures et leurs
  limites, les doutes ouverts. Rien à demander à personne.
- **[Voir le code, sur demande](#demander-à-voir-le-code)** — quelques lignes par courriel, et je vous ouvre
  le dépôt privé en lecture (ou une démonstration en direct).

## L'idée

Pas de serveur de jeu central qui fait autorité : les joueurs forment eux-mêmes le réseau. Ce que ça change :

- **Pas de coût serveur qui explose avec le succès** — l'infrastructure, ce sont les joueurs.
- **Une identité que vous possédez** — une clé cryptographique, comme une clé SSH, pas un compte sur la
  machine d'un tiers. *« web3 » est ici à prendre au sens décentralisé / identité possédée — pas un token,
  aucune crypto-monnaie.*
- **Pas de point de défaillance unique** qui éteint tout le monde d'un coup.

La boussole d'échelle est assumée comme lointaine : le plus grand affrontement coordonné de l'histoire du
jeu vidéo (EVE Online, bataille de B-R5RB, 2014, record du monde Guinness) a réuni des milliers de joueurs
dans un même espace — porté par une infrastructure serveur centralisée d'exception. Ce projet explore la
même frontière en se privant justement de ce serveur. C'est une direction de recherche, pas une métrique
atteinte.

## Conçu avec l'IA — et je le dis franchement

Je conçois et je construis ce projet **en binôme avec Claude** (l'IA d'Anthropic). Je ne le cache pas et je
ne le maquille pas : ça fait partie de l'histoire, et je préfère la raconter honnêtement.

L'IA est le **levier** qui me permet, seul et sans équipe, d'écrire octet par octet une infrastructure de
cette ambition — et d'apprendre en la construisant. Mais elle ne remplace pas le jugement : **chaque ligne,
je la relis, je la comprends, je l'assume.** Les décisions, les doutes, le cap tenu, le refus de crier
victoire trop vite — ça, ça reste moi. L'IA propose et va vite ; moi je tranche, je vérifie, et je signe.

Je le dis simplement : ce projet, dans cette forme et à ce rythme, **n'aurait pas existé sans cette
collaboration** — et le dire clairement, c'est ma règle numéro un : **l'honnêteté avant tout.**

## Ce qui est construit, et vérifié

Cœur en **Rust**, fait main, sans boîte noire — la seule dépendance externe est la bibliothèque de
cryptographie. **363 tests automatiques, 0 warning**, chaque chiffre reproductible.

- **Identité = votre clé.** Chaque message est signé ; aucun annuaire central ne décide qui vous êtes.
- **Traversée NAT réelle, jusqu'au cas le plus dur.** Deux humains derrière leurs box se connectent en
  direct ; quand le NAT est trop fermé (dit *symétrique*), un relais prend le relais — prouvé entre deux
  vrais réseaux sur Internet, pas en laboratoire.
- **Un réseau qui mesure ses propres liens** et adapte sa stratégie : dupliquer les données protège un lien
  à perte aléatoire mais aggrave un lien saturé, donc on ne le fait que lorsque c'est utile. Cette sonde a
  déjà corrigé l'une de nos hypothèses — la mesure prime sur l'intuition.
- **Perception de foule à coût borné.** Chacun ne dialogue à plein débit qu'avec un voisinage restreint
  (~32) et perçoit la foule lointaine en basse fidélité : ~34 Ko/s par nœud (~0,27 Mbit/s), indépendant du
  nombre total. En simulation, perception restaurée à ~87 % à 1 000 nœuds, à débit reçu plat.
- **Indépendant du moteur 3D.** Deux moteurs différents (Bevy et Unreal) ont été réunis dans le même espace
  partagé via un pont local. C'est ce qui rend une plateforme multi-moteur crédible.
- **Mesuré dehors, pas seulement simulé.** Un agent de mesure lancé par des volontaires a relevé, sur de
  vrais liens distants (plusieurs pays, certains derrière le NAT le plus dur), une présence vivante :
  fraîcheur p95 ~200–335 ms, perte réelle ~0.
- **Un premier passage de monde à monde.** Un lanceur natif et deux mondes Unreal (un hub, une île) : on
  franchit un portail et on bascule de l'un à l'autre, l'ancien monde n'étant fermé qu'une fois le nouveau
  affiché. Récent, et jugé fluide en jeu réel.

## Sécurité

Un réseau pair-à-pair n'a pas de serveur central pour arbitrer : chaque nœud doit se défendre seul, contre
des paquets écrits par n'importe qui.

Un audit interne mené le 22 juillet 2026 a trouvé **16 défauts** — dont une fuite de données personnelles et
une possibilité de transformer un nœud en arme d'amplification. Les 16 sont corrigés, chacun avec un test qui
échoue si la correction disparaît. Ce qu'on en retient vaut plus que la liste :

- **Aucune méthode ne suffit seule.** Le vol d'un objet partagé est passé au travers de 11 tests unitaires,
  4 tests de propriété, de ThreadSanitizer et de 240 millions de paquets de *fuzzing* — seul le banc
  d'attaque en conditions réelles l'a vu.
- **Prouver bat essayer.** [Kani](https://model-checking.github.io/kani) (AWS) a cassé en quelques secondes
  une fonction anti-amplification que 240 millions d'essais de *fuzzing* avaient jugée saine.
- **Test de mutation systématique.** Chaque correction est validée en remettant le code vulnérable et en
  vérifiant que le test tombe. Un test qui ne peut pas échouer ne prouve rien.

## Ce qui n'est pas prouvé

C'est une règle du projet : on écrit ce qui est prouvé **et** ce qui ne l'est pas.

- **L'échelle « très grande foule » n'est pas mesurée directement.** Les coûts sont mesurés jusqu'à
  ~1 000–2 000 nœuds en simulation ; au-delà, c'est de l'architecture et de l'extrapolation.
- **« Sans serveur » garde un astérisque.** L'amorçage passe encore par un point de rendez-vous ; la
  décentralisation de cette dernière brique est un chantier à venir.
- **Le chiffrement bout-à-bout n'est pas là.** Les positions circulent en clair : la signature garantit
  l'authenticité, pas le secret. C'est planifié.
- **Le test décisif est entamé, pas franchi.** On mesure que le substrat transporte une présence vivante
  entre machines distantes réelles — pas encore le ressenti de vrais joueurs qui bougent et jouent ensemble.

---

## La documentation, en libre accès

Tout ce qui suit se lit ici, sans rien demander à personne. C'est la matière technique du projet : ce qui
est mesuré, par quelle méthode, et où sont les limites.

**Vous voulez l'idée, sans jargon**
1. [Le « sans serveur », en clair](docs/comprendre-le-p2p.md)
2. [L'état du projet, chiffré](docs/etat-du-projet.md)
3. [Le registre des doutes](docs/doutes.md)

**Vous voulez l'architecture et les choix techniques**
1. [L'architecture et l'organisation du code](docs/ARCHITECTURE.md)
2. Les chantiers : [réseau](docs/chantier-reseau.md) · [foule dense](docs/chantier-foule.md) · [« vivant »](docs/chantier-vivant.md) · [robustesse](docs/chantier-robustesse.md)
3. [La sécurité, et ce qu'on a cassé chez nous](docs/SECURITE.md)

**Vous voulez juger vite**
1. [L'état du projet, chiffré](docs/etat-du-projet.md) — les mesures et leurs limites
2. [Le registre des doutes](docs/doutes.md) — la frontière honnête
3. [Les coulisses](docs/coulisses.md) — comment une mesure a, plusieurs fois, corrigé le plan

Le [sommaire complet](docs/) déroule le reste, et un [glossaire](docs/glossaire.md) explique chaque terme en
une phrase. Une réserve honnête : [docs/TESTS.md](docs/TESTS.md) explique comment rejouer les mesures
soi-même — ces commandes supposent l'accès au code, qui se demande juste en dessous.

## Demander à voir le code

Le code est privé pour une raison simple : il n'est pas prêt à être publié, et je préfère le montrer en
l'expliquant plutôt que de le laisser être jugé sur un malentendu. L'accès est accordé au cas par cas, et
volontiers — studios, entreprises, chercheurs, curieux du domaine.

**Deux niveaux, selon ce qui vous intéresse :**

| | Ce que vous obtenez |
|---|---|
| **Lecture du code** | Accès en lecture au dépôt privé : le cœur réseau en Rust, les tests, l'historique complet. |
| **Démonstration** | Une session en direct : le réseau qui tourne, la bascule d'un monde à l'autre, les mesures rejouées devant vous. |

**Le plus simple : écrivez-moi un mot.**

### → [shazamifius@gmail.com](mailto:shazamifius@gmail.com)

Dites qui vous êtes, ce que vous aimeriez voir — le code, une démonstration, ou les deux — et, si vous
voulez lire le code, votre identifiant GitHub (c'est ce qui me permet de vous ajouter au dépôt privé).
Rien de formel : quelques lignes suffisent, en français ou en anglais. Je réponds à tout le monde ; si
je n'ai pas répondu sous une semaine, relancez sans hésiter, c'est que ça m'a échappé.

*Vous êtes à l'aise avec GitHub et une demande visible publiquement ne vous dérange pas ? Vous pouvez aussi
[ouvrir une demande via un formulaire](../../issues/new?template=demande-acces.yml) — c'est le même
traitement.*

---

*Auteur : [shazamifius](https://github.com/shazamifius). Le code est sous licence « tous droits réservés ».*
