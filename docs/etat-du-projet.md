# État du projet — une revue honnête (et chiffrée)

> Une **revue de l'état courant** : ce qui a été construit et vérifié, **avec les mesures** ; les combats menés ;
> les murs rencontrés ; et — surtout — les **doutes ouverts**. Règle du projet : **une preuve = un chiffre
> reproductible.** Chaque mesure ci-dessous indique la commande pour la rejouer, et ce qu'elle **ne** prouve **pas**.
>
> Ce dépôt n'est **ni un produit, ni une promesse** : c'est une **exploration**. Voir aussi le
> [README](../README.md) et l'[architecture du code](ARCHITECTURE.md). Les commandes : [TESTS.md](TESTS.md).
>
> *Les chiffres ci-dessous ont été mesurés sur un PC de test (tour, 12 cœurs). Ils valent comme **ordres de
> grandeur reproductibles**, pas comme des constantes universelles.*

---

## 1. Les chiffres clés (reproductibles)

| Mesure | Valeur (PC de test) | Comment la reproduire |
|---|---|---|
| Débit **montant** par nœud (saturation, 32 voisins) | **~34 Ko/s** (max ~38) | `cargo run -- sim 50 3 15` |
| Débit **descendant** par nœud | **~31 Ko/s** | idem |
| **CPU** par nœud | **~0,7 %** d'un cœur | idem |
| **RAM** (crête du process) | **~38 Mo** | idem |
| **Perception de foule** à N = 1 000 | **~87 %**, à **débit reçu plat** (~46 Ko/s) | `cargo run -- coopsim-bus 1000` *(avec `POW_BITS=8`)* |
| Perception à N = 2 000 / 5 000 | **52 % / 49 %** (bridé par l'amorçage, pas le coût) | `coopsim-bus 2000` / `coopsim-bus 5000` |
| **Latence** du pont local (sidecar Rust↔moteur) | RTT **médian 47 µs**, p95 67 µs | `cargo run -- sidecar` |
| Taille d'un **paquet d'état signé** | **182 octets** (118 utiles + 64 de sceau Ed25519) | format, cf. [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Traversée NAT symétrique** via relais | établie **dans les deux sens** (réseaux réels) | `cargo run -- relay-test 6` *(banc déterministe)* |
| **Fraîcheur** sur liens **distants réels** (plusieurs pays, dont CGNAT) | **p95 ~200–335 ms** (< seuil 500 ms « vivant »), **perte réelle ~0** | instrument de mesure (agent) + journal du rendez-vous |
| **Tests** automatiques | **118**, 0 warning | `cargo test` |
| Plafond du **banc** de simulation | **~1 500 nœuds** (1 thread OS / nœud, 12 cœurs) | limite matérielle (voir §4) |

> **Ce que ces chiffres NE prouvent PAS** (honnêteté de méthode) : ils sont pris en **simulation / localhost** — le
> coût réseau réel (carte, RTT physique) n'est pas compté ; la « perception » compte les pairs **connus**, pas
> forcément **entendus à temps** ; et **« 55 000 » n'est jamais mesuré directement**. On ne dira donc **jamais**
> « 55K prouvé ».

## 2. Comment on extrapole à 55 000 (le calcul, pas un slogan)

Le débit d'un nœud est **borné par son voisinage (~32 voisins), pas par le total N** : un joueur n'émet à plein
débit qu'à ses voisins immédiats, quelle que soit la taille de la foule. À saturation, on **mesure ~34 Ko/s** en
émission, soit :

```
34 Ko/s ≈ 0,27 Mbit/s  (≈ 0,4 Mbit/s en comptant les en-têtes IP/UDP non mesurés)
```

Ce chiffre **ne change pas** à 55 000 joueurs : l'aire d'intérêt borne l'émission au voisinage, pas au total.
L'échelle se fait donc en **ajoutant des machines**, jamais en chargeant une seule. **Limite assumée :** c'est un
**argument d'architecture**, mesuré jusqu'à ~1–2 000 nœuds ; au-delà, c'est une **extrapolation**, et l'amorçage
(la découverte de pairs à grande échelle) reste le plafond à lever — d'où les 52 % / 49 % à 2 000 / 5 000.

## 3. Ce qui a été construit et vérifié

- **Identité = une clé.** Chaque message signé (Ed25519) ; l'identité est la clé publique, portée dans le paquet —
  elle s'auto-prouve, sans annuaire de confiance, et **persiste** entre sessions. *(tests unitaires + rechargement.)*
- **Traversée NAT réelle, jusqu'au cas difficile.** Hole-punching direct ; et sur NAT symétrique mobile, un
  **relais** prend le relais — établi **dans les deux sens entre deux réseaux distincts sur Internet** (pas en labo).
- **Résistance aux attaques.** De vrais programmes adverses (Sybil, éclipse, *framing*, inondation de gossip) sont
  joués contre le réseau : l'essaim tient, les états volés sont rejetés, les tricheurs mis en sourdine.
  *(`cargo run -- attack sybil-frame` · `attack gossip-flood`.)*
- **Perception de foule à coût borné.** Voisinage net + foule lointaine en résumés agrégés → **débit reçu plat**
  (~46 Ko/s), perception **~87 % à 1 000 nœuds**. *(`coopsim-bus`.)*
- **Indépendance du moteur 3D.** Deux moteurs (Bevy et Unreal) réunis **dans le même espace** via un pont local
  (latence **~47 µs**). *(prouvé à l'écran.)*
- **Présence « vivante ».** Avatars distants fluides et habités (interpolation + vie procédurale), même sous perte.
- **Mesuré dehors, pas seulement simulé.** Un **instrument de mesure** (un agent autonome que des volontaires
  lancent) a relevé, sur de **vrais liens distants** (plusieurs pays, certains derrière le NAT le plus dur), une
  présence **vivante** : fraîcheur **p95 ~200–335 ms** (sous le seuil de 500 ms), **perte réelle ~0**. *(Premier fait
  dur contre la « forteresse vide » — voir §6.)*

## 4. Les combats menés (la méthode)

Boucle stricte : **compiler → tester → prouver (un chiffre) → écrire.** On ne marque « fait » que le vérifié, et
tout « fait » liste ce qu'il **ne** fait **pas** (un registre de dettes).

- **Un cœur attaqué par nous-mêmes** : au lieu d'affirmer « c'est sûr », on écrit l'adversaire et on mesure. Un
  résultat négatif (un mur, un effondrement) **compte comme un progrès** s'il est reproductible.
- **Le juge est neutre** : pour les preuves réelles (NAT, relais), c'est le **journal du rendez-vous** qui tranche,
  pas l'enthousiasme — c'est ainsi qu'on a trouvé des bugs invisibles en labo (sur une seule machine le perçage
  réussit toujours ; seul un humain dehors révèle le vrai comportement).
- **La présentation a été séparée du cœur** (cœur devenu pur, agnostique), puis rebranchée par **paliers, chacun
  prouvé avant le suivant** (contrat → latence mesurée → vrai cœur → NAT réel).

## 5. Les murs rencontrés (les limites assumées)

- **L'échelle « 55 000 » n'est pas mesurée directement** : ~1–2 000 nœuds mesurés ; au-delà = extrapolation.
  L'**amorçage** à grande échelle est le plafond restant (52 % à 2 000, 49 % à 5 000).
- **« Sans serveur » garde un astérisque** : l'amorçage passe par un rendez-vous (présentations uniquement) ; le
  décentraliser entièrement est ouvert.
- **Pas de chiffrement bout-à-bout** : positions en clair (la signature garantit l'authenticité, pas le secret).
- **Le test décisif est *entamé*, pas franchi.** On a une **première mesure dehors** : des nœuds **distants réels**
  (plusieurs pays, dont CGNAT) sont **vivants** (fraîcheur p95 < 500 ms, perte réelle ~0). C'est un fait dur —
  l'infrastructure n'est plus *vide*. Mais ça mesure le **substrat** (la présence transportée), pas encore le
  **ressenti** : des humains qui **bougent et jouent ensemble** et le **sentent** vivant. Ce test-là, le plus
  important, reste devant.

## 6. Les doutes ouverts — le cœur de la démarche

> Ici, **un doute n'est pas une faiblesse à cacher : c'est l'objet du travail.** Le registre complet et suivi :
> **[les doutes](doutes.md)**.

- **L'inclusivité** : un lien faible *reçoit* trop en foule dense (l'aire d'intérêt borne l'émission, pas la
  réception). Comment garantir une expérience correcte du plus faible au plus rapide ?
- **La perception par pertinence, pas par proximité** : « les 32 plus proches » ≠ « les 32 qui comptent » (ceux à
  qui l'on parle, qui manipulent l'objet regardé). Problème d'architecture, pas de réglage.
- **La persistance sans serveur** : où vit l'état d'un joueur entre deux sessions, et qui empêche de le forger ?
- **La distribution / l'arrivée des joueurs** : installer, traverser les box, se retrouver — un mur d'usage réel.
- **La « forteresse vide »** : a-t-on bâti une belle infrastructure dans laquelle deux humains ne se sont jamais
  *vraiment* retrouvés, en mouvement, via le vrai Internet ? **Première réponse mesurée :** des nœuds distants réels
  y sont **vivants** (p95 < 500 ms, perte réelle ~0) — l'infrastructure n'est plus *vide*. Mais le **ressenti** (des
  humains qui bougent et jouent ensemble) n'est pas encore prouvé : le doute s'allège, il ne se ferme pas.

---

### Comprendre comment ça marche

- [Architecture & organisation du code](ARCHITECTURE.md) — les modules, le format d'un paquet, l'archi cible.
- [Comment lancer & tester](TESTS.md) — toutes les commandes ci-dessus, en détail.
- [Le « sans serveur », en clair](comprendre-le-p2p.md) — l'idée du projet expliquée en mots simples.

> **🌱 Parcours « découverte » →** étape suivante : **[Le registre des doutes](doutes.md)**.
