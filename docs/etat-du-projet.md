# État du projet — une revue honnête

> Une **revue de l'état courant** : ce qui a été construit et vérifié, les difficultés traversées, les murs
> rencontrés, et — surtout — les **doutes encore ouverts**. Le ton est volontairement sobre : un travail de R&D se
> juge autant à ses limites assumées qu'à ses réussites.
>
> Ce dépôt n'est **ni un produit, ni une promesse** : c'est une **exploration**. Voir aussi le
> [README](../README.md) (vue d'ensemble) et l'[architecture du code](ARCHITECTURE.md).

---

## 1. Ce qui a été construit et vérifié

Chaque point ci-dessous est adossé à une mesure ou un test reproductible (sans GPU, en headless), et indique
**honnêtement son périmètre de preuve**.

- **Identité = une clé.** Chaque message est signé (Ed25519) ; l'identité est la clé publique, portée dans le
  paquet — elle s'auto-prouve, sans annuaire de confiance. L'identité est **persistante** entre sessions.
  *Prouvé par tests unitaires et par le comportement (rechargement de la même clé).*
- **Traversée NAT réelle, jusqu'au cas difficile.** Hole-punching direct entre deux box ; et quand c'est impossible
  (NAT symétrique mobile), un **relais** prend le relais. *Prouvé entre deux réseaux distincts sur Internet, pas
  seulement en laboratoire.*
- **Résistance aux attaques.** Des attaquants (Sybil, éclipse, *framing*, inondation de gossip) sont **simulés
  contre le réseau** : l'essaim tient, les états volés sont rejetés, les tricheurs mis en sourdine. *Prouvé en
  simulation, avec de vrais programmes attaquants.*
- **Perception de foule à coût borné.** Chacun ne dialogue à plein débit qu'avec un petit voisinage et perçoit la
  foule lointaine en basse fidélité (résumés agrégés) → le débit reçu reste borné, **indépendant du nombre total**.
  *Mesuré en simulation : perception restaurée à ~87 % à 1 000 nœuds, à débit reçu plat.*
- **Indépendance du moteur 3D.** Le cœur réseau est agnostique : deux moteurs différents (Bevy et Unreal) ont été
  réunis **dans le même espace partagé** via un pont local (*sidecar*). *Prouvé à l'écran.*
- **Présence « vivante ».** Les avatars distants bougent de façon fluide et habitée (interpolation + vie
  procédurale), même sous perte de paquets. *Côté présentation (moteur), validé visuellement.*

## 2. Les combats menés (la méthode)

La démarche tient en une boucle stricte : **compiler → tester → prouver → écrire**. On ne marque « fait » que ce
qui est vérifié, et tout « fait » liste ce qu'il **ne** fait **pas** (le registre de dettes).

- **Un cœur attaqué par nous-mêmes.** Plutôt que d'affirmer « c'est sûr », on écrit le programme adverse et on
  mesure. Un résultat négatif (un mur, un effondrement) **compte comme un progrès** s'il est honnête et reproductible.
- **Le juge est neutre.** Pour les preuves réelles (NAT, relais), c'est le **journal du point de rendez-vous** qui
  tranche, pas l'enthousiasme : on a découvert ainsi des bugs invisibles en laboratoire (le perçage réussit toujours
  sur une seule machine — seul un humain dehors révèle le vrai comportement).
- **La présentation a été séparée du cœur.** Le moteur 3D a été retiré du cœur réseau (devenu pur, agnostique), puis
  rebranché via un pont construit **par paliers, chacun prouvé avant le suivant** (contrat → latence mesurée → vrai
  cœur → NAT réel).

## 3. Les murs rencontrés (les limites assumées)

- **L'échelle « 55 000 » n'est pas mesurée directement.** Les coûts sont mesurés jusqu'à ~1 000–2 000 nœuds en
  simulation ; au-delà, c'est de l'**architecture + extrapolation**, pas une preuve. (La découverte à grande échelle
  — l'amorçage — est le plafond restant, distinct du coût en régime établi.)
- **« Sans serveur » garde un astérisque.** L'amorçage passe encore par un point de rendez-vous (présentations
  uniquement) ; le décentraliser entièrement est un chantier ouvert.
- **Pas de chiffrement bout-à-bout.** Les positions circulent en clair (la signature garantit l'authenticité, pas le
  secret). Planifié, pas fait.
- **Le test décisif n'a pas eu lieu.** Prouver qu'un espace est *vivant* à plusieurs ne se fait pas en simulation :
  il faut de vrais joueurs, dehors. C'est le mur le plus important, et il est devant nous.

## 4. Les doutes ouverts — le cœur de la démarche

> Ici, **un doute n'est pas une faiblesse à cacher : c'est l'objet du travail.** Les principales familles :

- **L'inclusivité.** Un joueur à très faible débit *reçoit* trop de données en foule dense (l'aire d'intérêt borne
  ce qu'on émet, pas ce qu'on reçoit). Comment garantir que *n'importe quel* lien, du plus faible au plus rapide,
  garde une expérience correcte ?
- **La perception par pertinence, pas par proximité.** Voir « les 32 plus proches » n'est pas voir « les 32 qui
  comptent » (ceux à qui l'on parle, ceux qui manipulent l'objet regardé). Sélectionner la foule par **pertinence
  sociale** est un problème d'architecture, pas de réglage.
- **La persistance sans serveur.** Une progression qui survit aux sessions, sans magasin central qui fasse autorité
  ni permette la forge : où vit l'état d'un joueur, et qui empêche de le falsifier ?
- **La distribution / l'arrivée des joueurs.** Installer, traverser les box, se retrouver à plusieurs : un mur
  d'usage réel, souvent sous-estimé, qui conditionne tous les tests.
- **La « forteresse vide ».** Le doute qui peut tout résumer : a-t-on bâti une belle infrastructure dans laquelle
  deux humains ne se sont jamais *vraiment* retrouvés, en mouvement, via le vrai Internet ? Tant que ce n'est pas
  vécu dehors, ce doute reste ouvert.

---

### Pour comprendre comment ça marche

- [Architecture & organisation du code](ARCHITECTURE.md)
- [Comment lancer & tester](TESTS.md)
- *(à venir)* une introduction pédagogique au pair-à-pair décentralisé — du néophyte à la compréhension du « sans serveur ».
