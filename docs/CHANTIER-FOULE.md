# 👥 CHANTIER FOULE DENSE & INCLUSIVITÉ (ch. 8, ferme D22) — le gros morceau d'archi

> Focus/conscience, gossip, résumés de cellule, perception à l'échelle, corroboration anti-inflation.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

### Chapitre 8 — La foule dense & l'inclusivité (fermer D22) 🔴 *priorité 2 — LE gros morceau d'archi*
**But :** que dans une foule de 200, 500, 5000, chacun **perçoive la foule entière**
(proches nets, lointains dégradés) **sans plafond dur à 32** et **sans exploser son
débit** — et que le 0-connexion comme le 2 Gb/s aient chacun LA meilleure expérience.

> **Pourquoi ce chapitre existe (constat mesuré au 7.4b, voir D22).** Tout notre passage
> à l'échelle (6.6) achète son « coût constant à 55k » par un **plafond dur** :
> [rendezvous.rs](src/net/rendezvous.rs) ne présente que les `MAX_NEIGHBORS = 32` plus
> proches, et [receive.rs](src/net/netcode/receive.rs) **écrase** `link.peers` avec ce
> roster (les clients ne s'échangent RIEN entre eux). Donc le **33e voisin n'existe pas
> et ne peut jamais exister** — le water-filling d'[aoi.rs](src/net/aoi.rs) répartit le
> débit entre 32 connus mais n'apprend jamais le 33e. À une foule de 200, chacun est
> **aveugle à 168**. Dans un monde social type VRChat, VOIR la foule EST le jeu.

> **L'idée directrice — séparer deux choses que le plafond à 32 confond :**
> 1. le **FOCUS** = à qui je tiens un lien netcode plein débit (prédiction/20 Hz). Ça
>    DOIT rester borné (~16-32). C'est, en gros, le système actuel.
> 2. la **CONSCIENCE** = qui je perçois / sais exister (des centaines), en LOD/basse
>    fidélité. Ça ne doit **PAS** être plafonné. C'est ton « **AoI par vision** » : pas
>    besoin d'un lien 20 Hz avec un type à 80 m, juste de savoir qu'il est là, pour pas cher.

> **⚠ L'INVARIANT À TENIR (le piège à ne JAMAIS se cacher).** Le coût de réception doit
> rester **O(K_focus + C_cellules)**, *indépendant de N* (la taille de la foule). Sinon on
> a juste rebaptisé le problème. La PREUVE de réussite n'est donc pas « la couverture
> monte » seule — c'est **couverture → ~100 % ET débit ↓ qui reste PLAT quand la foule
> grandit**. Augmenter `MAX_NEIGHBORS` est interdit : ça rouvre l'O(N²) (trou n°3), le
> WELCOME débordé (trou n°2) et surtout D3 (le faible noyé). On déplacerait le mur, pas
> on ne le casserait.

**— Phase A : VOIR la foule sans plafond dur (le cœur de D22) —**

- [x] 8.0 ✓ — **Scénario de foule + métriques de perception dans `sim` (mesurer le mur AVANT
  de le casser).** Nouveau mode `cargo run -- crowd <N> [secs]` (N bots co-localisés sur un
  cercle de 3 m → tous réellement à portée) et DEUX mesures neuves au rapport, à côté du
  probe 7.4 (Ko/s ↓, CPU) :
  • **Couverture de perception** = sur les pairs réellement à portée (actifs − 1), combien
    ce nœud perçoit-il ? Aujourd'hui : plafonné → `32/(N−1)`.
  • **Tiers de fidélité** = *focus* (lien plein) vs *conscience* (basse fidélité). Aujourd'hui :
    conscience = 0 (le tier n'existe pas encore → 8.2).
  **PROUVÉ (le mur, en ROUGE comme prévu) :** `crowd 60` → couverture **54 %** (aveugle à 27) ;
  `crowd 200` → couverture **16 %**, FOCUS 32 + CONSCIENCE 0, **aveugle à 167**. Le chiffre
  prédit (16 %) = le mesuré. **Débit de référence À BATTRE** (le coût qui devra rester PLAT
  quand la couverture montera) : `crowd 200` → **↓ 24,8 Ko/s moy (41 max), ↑ 26,8 Ko/s** ;
  l'essaim TIENT (orbe 0/200). *Rien n'est « résolu » ici — le mur est juste rendu chiffrable
  et reproductible, pour qu'on sache à la fin si on l'a vraiment cassé.* 36 tests, 0 warning.

> **⚙ CONCEPTION DÉTAILLÉE 8.1 (écrite AVANT de coder, 19 juin) — avec une DÉCOUVERTE faite
> en lisant le code.**
>
> **La bonne nouvelle d'abord (le water-filling nous a déjà à moitié sauvés).** En relisant
> [aoi.rs](src/net/aoi.rs) + [bot.rs](src/net/bot.rs), le budget d'émission est FIXE
> (`SEND_BUDGET_HZ = 240`) et réparti entre les pairs connus. Conséquence ARITHMÉTIQUE : si
> chacun connaît N−1 pairs, il envoie ~240/(N−1) Hz à chacun ; un receveur reçoit donc de
> (N−1) émetteurs × 240/(N−1) = **~240 Hz au total, QUEL QUE SOIT N**. Autrement dit, **le coût
> de réception est DÉJÀ borné** par la générosité (fixe) des émetteurs — l'invariant « débit
> plat » est à moitié déjà tenu, côté débit. Ce qui n'est PAS tenu : (1) la DÉCOUVERTE (plafond
> 32 → le 33e jamais appris) et (2) la FRAÎCHEUR par pair, qui s'effondre uniformément en 1/N
> (à 5000, une maj toutes les ~20 s pour tout le monde → trop vieux même pour du LOD).
>
> **Ce que ça change pour le plan (et l'ordre se confirme) :**
> - **8.1 (gossip) seul** doit faire MONTER la couverture 16 %→~100 % à `crowd 200`, avec un
>   débit qui monte au plafond (~43 Ko/s) puis reste PLAT à 500. Preuve que le plafond 32 était
>   arbitraire. MAIS tout le monde sera également « flou » (~1 Hz).
> - **8.2** rend les PROCHES nets (focus 20 Hz ; la conscience ne touche que les miettes du budget).
> - **8.3** fait tenir la fraîcheur des LOINTAINS à 5000 : une région = UN flux agrégé, pas des milliers.
>
> **Mécanique concrète de 8.1 :**
> - Nouveau paquet `KIND_GOSSIP` (carte de visite), nouveau `src/net/gossip.rs` :
>   `[type|ver|count| (id 32, ip 4, port 2, x 4, z 4)×count ]`, `count` borné (≤16 → paquet < 800 o).
> - [receive.rs](src/net/netcode/receive.rs) + [bot.rs](src/net/bot.rs) : le WELCOME n'ÉCRASE
>   plus `link.peers`, il l'AMORCE (merge). On RETIRE le `retain(present)` du bot (ligne ~194).
> - Chaque nœud émet périodiquement (~1 Hz) un GOSSIP vers quelques pairs, avec un sous-ensemble
>   DIVERS (tirage tournant, pas toujours les mêmes → anti-éclipse) des pairs qu'il connaît.
> - À la réception : on fusionne les cartes INCONNUES (hole=false → on les perce ensuite via la
>   machinerie PUNCH existante), table bornée en MÉMOIRE (`MAX_KNOWN`, éviction des plus vieux →
>   amorce D16). Aucune confiance à une source unique (corroboration durcie en 8.8).
> - **Preuve :** `crowd 200` couverture 16 % → ~100 % ; puis `crowd 500` montre le débit ↓ PLAT
>   (ne croît pas avec N) → le plafond est cassé sans rouvrir l'O(N²) ni exploser le débit.

- [x] 8.1 ✓ — **Découverte décentralisée par gossip (le 33e devient APPRENABLE).** Le
  rendez-vous cesse d'être l'énumérateur autoritaire et redevient un simple **amorçage** :
  le WELCOME n'**écrase** plus `link.peers`, il l'**amorce**. Ensuite, chaque pair annonce
  à bas débit, à ses voisins, quelques AUTRES pairs qu'il connaît (id + dernière position
  connue) — des « cartes de visite ». La table de pairs s'enrichit ainsi **sans** plafond à
  32 et **sans** serveur central qui énumère. Table bornée en MÉMOIRE (lié D16) mais pas en
  VISION. **Anti-éclipse dès le départ (amorce D9) :** diversité forcée des informateurs, on
  corrobore une position par plusieurs sources (un menteur seul ne peut pas te cacher/inventer
  la foule). *Ferme une partie de D10 (rendez-vous démoté à l'amorçage) ; amorce D9.*
  **Preuve :** à `crowd 200`, couverture 16 % → ~100 % (chacun finit par apprendre les 200),
  sans que le rendez-vous ne les énumère. *Risque à surveiller : le gossip lui-même coûte du
  débit — c'est 8.2 + 8.3 qui le bornent ; à 200 c'est tenable, à 5000 il FAUDRA l'agrégation.*
  **FAIT (19 juin) — le mur tombe, l'invariant TIENT.** Nouveau `src/net/gossip.rs`
  (paquet `KIND_GOSSIP` = « cartes de visite » : id + adresse + dernière position, ≤16/paquet,
  sérialisation à la main + 6 tests). Logique d'apprentissage CENTRALISÉE dans `NetLink`
  (`learn_peer` + `note_pos` + `peer_pos`, borne mémoire `MAX_KNOWN = 4096`) → **partagée par le
  bot ET le jeu** (on évite de rouvrir D2). Le WELCOME **amorce** désormais `link.peers` (merge),
  il ne l'**écrase** plus ([receive.rs] + [bot.rs], `retain(present)` supprimé). Chaque nœud diffuse
  ~2 Hz un lot de cartes (sous-ensemble DIVERS par curseur tournant → amorce anti-éclipse) à
  `GOSSIP_FANOUT = 4` voisins ; à la réception on apprend les inconnus (puis on les perce).
  **MESURÉ** (`crowd`, PC tour ASUS, ventilos au max) : couverture **16 % → 98 %** à `crowd 200`
  (50 s ; moy 194/199 voisins), **67 %** à `crowd 500` (40 s, convergence non finie : démarrage
  échelonné). **L'INVARIANT EST PROUVÉ** : le débit ↓ ne grandit PAS de 200 à 500 (↓35 → ↓27 Ko/s,
  ↑42 → ↑39 Ko/s, **PLAT/en baisse**), CPU ~0,7 % inchangé, orbe 0 volée, essaim TENU. 42 tests,
  0 warning. *Pourquoi le débit reste plat sans rien faire de plus : le budget d'émission est FIXE
  (240 Hz) et réparti → quand chacun connaît N−1 pairs, il envoie 240/(N−1) Hz à chacun, donc un
  receveur reçoit (N−1)×240/(N−1) = ~240 Hz, indépendant de N (la DÉCOUVERTE était le seul mur).*
  **Résidu pour 8.2/8.3 :** la fraîcheur PAR pair s'effondre en 1/N (uniforme) → 8.2 rend les
  proches nets (focus 20 Hz) ; 8.3 fait tenir les lointains à 5000 (agrégation par cellule).

> **⚙ CONCEPTION DÉTAILLÉE 8.1b (écrite AVANT de coder, 19 juin) — la rançon honnête du gossip.**
>
> **Le diagnostic, en relisant le code (D23 n'est pas UN bug mais TROIS sous-attaques).** Une carte
> de visite affirme `(id, adresse)` choisis par l'émetteur, SANS preuve. En lisant [punch.rs](src/net/punch.rs),
> [send.rs](src/net/netcode/send.rs), [bot.rs](src/net/bot.rs) et [receive.rs](src/net/netcode/receive.rs),
> j'ai isolé exactement ce qu'un menteur peut en faire :
> 1. **Pollution de table (ids-poubelle).** `learn_peer` n'exige PAS `has_pow` sur l'id d'une carte.
>    Un attaquant forge des milliers d'ids gratuits → il remplit `link.peers` jusqu'à `MAX_KNOWN = 4096`,
>    et comme il n'y a pas d'éviction (D16), les vrais pairs ne rentrent plus. Coût attaquant : ~0.
> 2. **Réflexion / amplification de PUNCH (le vrai danger).** Découverte clé : le STATE et le gossip
>    ne partent QU'aux trous DÉJÀ ouverts (un trou ne s'ouvre que quand on a *entendu* le pair). La SEULE
>    chose qu'on envoie à une adresse jamais corroborée, c'est le **PUNCH** — et [net_punch](src/net/punch.rs)
>    le répète vers une adresse jamais confirmée **indéfiniment** (4×/s, sans jamais abandonner). Donc une
>    carte `(id quelconque, adresse = VICTIME)` fait que CHAQUE nœud qui la reçoit perce la victime 4×/s,
>    pour toujours. Diffusée à la foule → flot réfléchi soutenu vers la victime, source (l'attaquant) masquée.
> 3. **Redirection d'un pair connu.** `learn_peer` rafraîchit l'adresse d'un pair DÉJÀ connu depuis
>    n'importe quelle carte. Une carte `(id-d-un-vrai-pair, adresse = VICTIME)` détourne donc notre trafic
>    vers la victime ET nous fait perdre le vrai pair.
>
> **Les défenses (en profondeur — chacune dit son rôle ET sa limite, règle anti-enfermement).**
> - **(a) PoW exigée sur chaque carte apprise.** On rejette tout id sans `has_pow(POW_BITS)`. Une fausse
>   identité coûte alors ~2¹⁶ (comme une vraie). *Ferme #1. Limite : n'arrête PAS la réflexion via des ids
>   RÉELS récoltés au rendez-vous/dans le gossip (#2/#3).*
> - **(b) Le gossip n'ÉCRASE jamais l'adresse d'un pair déjà connu.** On sépare « apprendre via gossip »
>   (ouï-dire → n'AJOUTE que des inconnus) de « rafraîchir via WELCOME/paquet signé » (corroboré). *Ferme #3.
>   Limite : un vrai changement d'adresse (NAT rebinding) ne sera vu que via le prochain paquet signé du pair,
>   pas via gossip — c'est voulu.*
> - **(c) Perçage spéculatif borné ET abandonné.** [net_punch] (et le bot) cessent de percer un trou jamais
>   corroboré après un délai généreux (~`PUNCH_GIVEUP` essais ≈ 10 s) au lieu de marteler à vie. *Ferme la
>   DURÉE de #2 : une carte empoisonnée n'arrose la victime que quelques secondes, plus l'éternité. Limite :
>   un pair derrière NAT symétrique (qui ne répond jamais au perçage direct) sera aussi abandonné → il lui
>   faudra un relais (D17), pas un perçage éternel. C'est le bon compromis.*
> - **(d) Rate-limit de l'apprentissage PAR SOURCE.** Un même expéditeur de gossip (son adresse) ne peut
>   nous faire apprendre qu'un nombre borné de NOUVEAUX pairs/s (seau à jetons par source, comme le 5.5).
>   *Ferme le DÉBIT d'injection (#1 et #2 à la source). Limite : un attaquant à plusieurs sources contourne
>   partiellement — mais chaque source reste bornée, et émettre du gossip crédible suppose un PoW payé.*
> - *NON retenu pour 8.1b (anti-gold-plating) : un plafond GLOBAL de perçage/s. (c)+(d) bornent déjà débit
>   ET durée ; si la preuve montre un résidu, on l'ajoutera — pas avant.*
>
> **Où vit le code (anti-D2).** Toute la confiance des cartes va dans `NetLink` (`learn_from_gossip` +
> le seau par source), comme `learn_peer` au 8.1 → **partagée par le bot ET le jeu**, testée une fois. Les
> appelants ne font que « recharger les seaux » + « tenter d'apprendre ». L'abandon de perçage touche
> [punch.rs] (jeu) et la boucle de perçage du [bot.rs] (miroir).
>
> **Preuve (un VRAI attaquant, fidèle à la philosophie du projet).** `cargo run -- attack gossip-flood` :
> s'inscrit, récolte les vraies victimes, ouvre une 2ᵉ prise « cible de réflexion », puis déverse aux
> victimes des cartes empoisonnées `(id-poubelle, addr=cible)` et `(id-réel, addr=cible)` ; il COMPTE les
> perçages réfléchis arrivant sur la cible. Attendu : ~0 (les poubelles rejetées par PoW, les ids réels non
> écrasés). + tests unitaires sur `learn_from_gossip` (PoW rejeté, adresse non écrasée, seau par source qui
> plafonne) et sur l'abandon de perçage. + `gossip-flood` ajouté aux variantes de `sim` : l'essaim TIENT,
> couverture honnête inchangée.

- [x] 8.1b ✓ — **Durcir le gossip (ferme D23) — AVANT d'empiler 8.2.** Le 8.1 a échangé le plafond
  de 32 contre une porte d'entrée DoS : on apprenait des cartes `(id, adresse)` sans preuve de travail
  ni corroboration → réflexion/amplification possible vers une victime, et pollution de table. On a
  refermé AVANT de construire dessus (règle d'or : pas de béton sur du sable). Quatre défenses en
  profondeur : (a) `has_pow` exigé sur l'id d'une carte apprise (poubelles jetées) ; (b) le gossip
  n'écrase JAMAIS l'adresse d'un pair déjà connu (anti-redirection) ; (c) abandon du perçage spéculatif
  après ~10 s non corroborés (`PUNCH_GIVEUP`, [punch.rs]) — avant on arrosait à VIE ; (d) rate-limit de
  l'apprentissage PAR SOURCE (seau à jetons dans `NetLink`). Logique centralisée dans `NetLink`
  (`learn_from_gossip`) + une fonction pure partagée `punch_abandoned` → bot ET jeu, pas de re-D2.
  **FAIT (19 juin) — la porte DoS est fermée, la découverte honnête intacte.** Nouvel attaquant RÉEL
  `cargo run -- attack gossip-flood` ([attack.rs]) : il ouvre une 2ᵉ prise « cible » et déverse aux
  victimes des cartes pointant toutes vers elle. **MESURÉ : 0 perçage réfléchi reçu par la cible**
  (poubelles jetées par PoW, ids réels non redirigés), table de la victime NON polluée (reste à ses
  vrais pairs). La découverte honnête est inchangée : `crowd 60` → couverture **100 % (min 90 %)**,
  orbe 0/60. `sim 40 6 20` (avec `gossip-flood` ajouté aux variantes) → essaim TENU, orbe 0/40,
  couverture 100 %. **47 tests, 0 warning** (+5 : `learn_from_gossip` PoW/no-overwrite/rate-limit,
  abandon de perçage, aller-retour PUNCH). *Limites assumées (registre) : un attaquant à plusieurs
  sources contourne partiellement le rate-limit (chaque source reste bornée) ; un pair NAT symétrique
  est aussi abandonné par (c) → relais nécessaire (D17). Plafond GLOBAL de perçage NON ajouté
  (anti-gold-plating) : (c)+(d) suffisent, on l'ajoutera si une mesure le réclame.* *Ferme D23 ; amorce D9.*

- [x] 8.1b-preuve ✓ — **Preuve RÉELLE de l'abandon de perçage (doute #1 FERMÉ).**
  *Honnêteté :* le « 0 perçage réfléchi » du 8.1b couvrait les cartes-poubelle (jetées par PoW) et les
  ids déjà connus (non redirigés). Le cas le plus DUR — un attaquant qui **mine de vraies identités PoW
  neuves** et les pointe vers une victime — n'était prouvé que par TESTS UNITAIRES (`punch_abandoned` +
  seau par source), pas bout-en-bout. Trou de *démonstration*, pas de *défense*. **FAIT (19 juin) :**
  `attack gossip-flood` mine maintenant `N_POW = 4` identités PoW neuves pointant vers la cible et compte
  les perçages réfléchis **par fenêtres de 2 s sur ~20 s**, EN inondant sans arrêt. **MESURÉ (3 bots) :**
  rafale CONSTANTE de **96 perçages/2 s de t=0 à t=10 s, puis 0 de t=10 à t=20 s** (total 480, queue 0).
  L'abandon mord **pile à ~10 s** (`PUNCH_GIVEUP = 40 × 0,25 s`), **même en continuant d'inonder** —
  re-déverser une carte ne réarme pas le perçage (un id connu n'est pas réappris). Avant 8.1b : 96/2 s
  **SANS FIN**. *La réflexion par ids PoW minés est donc BORNÉE dans le temps, prouvé en réel — pas
  seulement en test unitaire. Doute #1 fermé.* *(Résidu inchangé : un attaquant peut relancer une
  rafale en minant de NOUVELLES identités, mais chacune coûte un PoW et ne dure que ~10 s → plus une
  amplification, juste un coût attaquant linéaire ; la corroboration multi-informateurs du 8.8 le réduira encore.)*

> **⚙ CONCEPTION DÉTAILLÉE 8.2 (écrite AVANT de coder, 19 juin) — avec une DÉCOUVERTE en relisant [aoi.rs].**
>
> **Le vrai problème, vu dans le code (pas celui que je croyais).** Le water-filling d'[aoi.rs] répartit
> le budget par `relevance_weight(distance)`. En géométrie VARIÉE (vrai monde), ça donne déjà un dégradé :
> proches ≈ 20 Hz, lointains peu. MAIS en **foule DENSE** (le cas `crowd` : tout le monde à ~même
> distance, sous `COMFORT_DIST = 6 m`), **tous les poids sont ~égaux** (~1,0) → le budget 240 Hz se
> répartit UNIFORMÉMENT → ~240/(N−1) Hz pour chacun (≈1,2 Hz à 200). **Tout le monde est également flou.**
> C'est ça que 8.1 a laissé ouvert et que 8.2 doit casser : faire émerger des PROCHES nets même quand la
> pertinence-distance ne discrimine plus.
>
> **L'idée : deux tiers EXPLICITES (pas juste un dégradé).**
> - **FOCUS** = les `K_FOCUS` pairs les PLUS pertinents (≈16). Lien plein : jusqu'à `SEND_HZ = 20`,
>   prédiction/interpolation, avatar 3D détaillé. Borné → c'est la borne d'échelle (on NE touche PAS à
>   `MAX_NEIGHBORS` côté émission ; le focus est un sous-ensemble servi en priorité).
> - **CONSCIENCE** = tout le reste de la table connue. Petit budget réservé, débit plafonné bas
>   (`CONSCIENCE_HZ` ≈ 1–2 Hz), pas de prédiction fine, rendu LOD/imposteur bon marché.
>
> **Mécanique d'allocation (extension d'[aoi.rs], pas une réécriture).** On trie les pairs par
> `relevance_weight` ; les `K_FOCUS` premiers reçoivent un water-filling avec la GROSSE part du budget
> (≈ `K_FOCUS × SEND_HZ`), le reste (conscience) un water-filling avec la part résiduelle et un `r_max`
> bas. *Différence clé avec aujourd'hui : on RÉSERVE de quoi rendre les proches nets AVANT de saupoudrer
> le reste, au lieu de tout étaler.* En foule dense où les poids sont égaux, le « tri » choisit K pairs
> (arbitraire mais STABLE) → en vrai jeu la géométrie variée les choisit naturellement.
>
> **⚠ L'invariant tient toujours (à re-prouver, pas à affirmer).** Réception d'un nœud = somme de ce que
> les autres lui envoient. Il est dans le FOCUS de ses ~`K_FOCUS` voisins proches (≈ `K_FOCUS × 20` Hz)
> + dans la CONSCIENCE de tous les autres (chacun lui donne une miette ; la somme des miettes ≈ le budget
> conscience d'UN émetteur, **indépendant de N**). Donc réception ≈ `K_FOCUS × 20 + budget_conscience` =
> **PLAT quand N grandit**. La preuve = rejouer `crowd 200` puis `500` et montrer ↓ plat ET focus≈K plein.
>
> **+ Honnêteté de mesure (dette du 8.1).** La couverture compte aujourd'hui les pairs CONNUS (`link.peers`).
> On la corrige pour compter les **ENTENDUS récemment** (un instantané reçu dans les N dernières s). Le
> rapport `crowd` distinguera alors FOCUS (entendus à plein débit) / CONSCIENCE (entendus en basse fidélité).
>
> **+ Amorce D24 (rendu à deux tiers).** Côté [receive.rs] : les pairs FOCUS → avatar détaillé (la borne
> `MAX_AVATARS` ne s'applique plus qu'à EUX, anti-DoS conservé) ; les CONSCIENCE → imposteur LOD bon marché
> (silhouette/point), plafond bien plus haut. *Je code la logique de tiers (vérifiable au bot : débit, qui
> est focus/conscience), mais le RENDU 3D (FPS, silhouettes) ne se vérifie que dans le vrai jeu → c'est TOI
> qui le lances ; je te donnerai `tools/foule-3d.sh <N>` pour ouvrir une foule d'un coup.*
>
> **Sous-étapes prévues (chacune compilée + testée + commitée) :** **8.2a** allocation à deux tiers dans
> [aoi.rs] (+ tests : focus servi plein, conscience en miettes, budget respecté) appelée par [send.rs] ET
> [bot.rs] ; **8.2b** métrique « entendus récemment » + tiers focus/conscience au rapport `crowd` ; **8.2c**
> rendu à deux tiers ([receive.rs]) + `tools/foule-3d.sh` (amorce D24, vérif 3D par l'utilisateur).
> **Preuve globale :** `crowd 200`/`500` → couverture (entendue) ~haute, focus≈`K_FOCUS` à plein débit,
> conscience = le gros de la foule en basse fidélité, **débit ↓ PLAT** quand N grandit.

- [ ] 8.2 — **AoI à DEUX TIERS : focus (≤K, plein débit) + conscience (basse fidélité).**
  Séparer dans le code « à qui je tiens un lien netcode complet » (borné ~16-32, prédiction/
  réconciliation/20 Hz, ≈ aujourd'hui) de « qui je perçois » (tout le reste de la table :
  position échantillonnée ~1 Hz, pas de prédiction fine, rendu LOD/imposteur). Le
  water-filling d'[aoi.rs](src/net/aoi.rs) s'étend : il sert d'ABORD le focus, puis saupoudre
  un PETIT budget résiduel sur la conscience. *C'est ton « AoI par vision » concrétisée.*
  **+ Honnêteté de mesure (dette du 8.1) :** corriger la couverture pour compter les pairs
  **vraiment entendus récemment**, pas seulement *connus* dans `link.peers`.
  **+ Amorce D24 :** relier `MAX_AVATARS` au tier de rendu — focus en avatars détaillés (borne
  anti-DoS conservée), conscience en LOD/imposteurs bon marché → on AFFICHE des centaines de
  silhouettes sans le plafond plat de 64. **Preuve :** couverture (entendue) ~100 % MAIS le débit ↓
  reste **borné** (focus ≈ 32 pleins, la conscience ne coûte que des miettes) → la vraie preuve de
  D22 : couverture↑ **sans** explosion du débit ; et dans le vrai jeu 3D on voit > 64 silhouettes.
  - **8.2a ✓ FAIT (19 juin)** — allocation à deux tiers `allocate_two_tier` dans [aoi.rs] (+ 3 tests :
    focus émerge en foule dense, petit groupe tout au plein débit, le plus pertinent passe au focus),
    branchée sur le jeu ([send.rs]) ET le bot ([bot.rs]). **Choix corrigé en codant : `K_FOCUS = 8`,
    PAS 16** — à 240 Hz de budget, 16 focus (×20 = 320) videraient tout et tueraient la conscience ;
    8 (×20 = 160) laissent **80 Hz garantis** à la conscience. Sanity `crowd 60` : essaim TENU, orbe
    0/60, débit borné (↑45 Ko/s). 50 tests, 0 warning. *La preuve PARLANTE (focus net / conscience LOD
    distingués) vient avec 8.2b — la métrique actuelle compte les CONNUS, pas la fidélité d'écoute.*
  - **8.2b ✓ FAIT (19 juin) — et la métrique a DÉMASQUÉ un défaut de 8.2a (le but d'une bonne mesure).**
    Le rapport `crowd` compte désormais les pairs **ENTENDUS** sur la fenêtre (compteur par pair dans
    [bot.rs], remis à zéro au début de la fenêtre) et les classe en **FOCUS** (≥ ~5 Hz, plein débit) vs
    **CONSCIENCE** (entendu mais moins). ⚠️ **CE QU'ELLE RÉVÈLE (mesuré) :** `crowd 80` → FOCUS 8,6 +
    CONSCIENCE 68 (couv. 98 %) — beau ; MAIS `crowd 160` → **FOCUS 0,2** + CONSCIENCE 147 (couv. 92 %),
    et ça **ne remonte pas** avec une fenêtre plus longue (40 s) → **pas un artefact de convergence, un
    vrai défaut.** *Cause (en relisant le code) :* le focus CHURNE — `relevance_weight` dépend des
    positions VIVANTES (les bots bougent) → le « top-8 » se recalcule à chaque tick ; en foule dense des
    dizaines de pairs sont à quasi-égalité au bord du top-8 → l'ensemble focus change sans cesse → aucun
    lien 20 Hz SOUTENU. *Pire :* un pair pas-encore-entendu a distance 0 → poids MAX (le coup de pouce
    « découvre vite ») → le focus cible les INCONNUS, qui sortent une fois entendus → rotation perpétuelle.
    **Conséquence : le budget plein débit est ÉTALÉ → retour au « tout le monde flou » que 8.2 devait
    casser.** À 80 ça marchait par chance (moins de churn). *La métrique a fait son travail : 8.2 n'est
    PAS fini.* 50 tests, 0 warning.
  - **8.2a-bis ✓ FAIT (19 juin) — focus COLLANT : le churn est mort, l'invariant TIENT.** Ensemble
    focus PERSISTANT dans `NetLink` (`focus: Vec<PeerId>` + `refresh_focus`/`is_focus`), maintenu par
    HYSTÉRÉSIS : (1) on retire les partis, (2) on remplit les places libres par les plus pertinents, (3)
    on ne REMPLACE un membre que si un autre est `FOCUS_SWAP_MARGIN = 1,5`× plus pertinent (un échange/tick).
    DÉCOUPLAGE découverte/focus : la pertinence vient de la position CONNUE (`peer_pos`) ; un pair sans
    position connue a pertinence 0 → il n'accapare plus de slot (il se fait entendre par la conscience, pas
    en volant le plein débit). `allocate_rates` → `allocate_tiers(weights, is_focus, …)` : le focus est
    DONNÉ, plus recalculé au tri. Logique centralisée dans `NetLink` → partagée [send.rs] (passé en `ResMut`)
    ET [bot.rs] (anti-D2). **PROUVÉ (fenêtre identique 30 s) :** `crowd 160` FOCUS **0,2 → 9,4** (le churn
    était bien la cause). Pair d'invariant 80↔160 : **FOCUS borné 8,8 → 10,5** (ne grossit pas avec N),
    **CONSCIENCE 68 → 134** (= LOD de toute la foule, scale avec N), **débit ↓ 43,8 → 40,4 Ko/s = PLAT**
    quand N double, orbe 0 volée. +1 test (`focus_est_collant_pas_de_churn` : stable sous bruit, accepte un
    pair nettement plus proche). 51 tests, 0 warning. *L'invariant de D22 est enfin tenu POUR DE VRAI :
    couverture en deux tiers (proches nets + foule en LOD) à débit reçu CONSTANT. Reste 8.2c (rendu 3D).*
    *Résidu honnête : couverture « entendue » à 30 s = 91 % à 160 (convergence non finie dans la fenêtre,
    comme au 8.1) ; le tri des candidats focus est O(N log N)/tick → à revoir pour 5000 (index spatial, 8.3).*
  - **8.2c ✓ CODÉ (19 juin) — rendu 3D à deux tiers (ferme l'amorce D24) — ⏳ VÉRIF VISUELLE = utilisateur.**
    [receive.rs] : un avatar porte un tier (`detailed` dans [state.rs]). FOCUS (`link.is_focus`) → avatar
    DÉTAILLÉ (corps articulé + tête + pseudo), borné par `MAX_FOCUS_DETAIL = 64` (la borne anti-DoS ne pèse
    plus que sur le rendu COÛTEUX). CONSCIENCE → IMPOSTEUR LOD (une silhouette capsule, sans tête ni pseudo),
    borné bien plus haut (`MAX_AWARE = 512`). Bascule de tier à la volée (despawn+respawn) quand un pair
    entre/sort du focus — rare car le focus est collant (8.2a-bis). [nameplates.rs] n'étiquette QUE le focus
    (sinon 500 labels illisibles). Nouveau `tools/foule-3d.sh <N>` : ouvre le rendez-vous + N fenêtres
    clientes d'un coup (récupère l'env nix une fois, lance le binaire direct → rapide ; Ctrl-C ferme tout).
    **51 tests, 0 warning, build vert.** ⚠️ *JE NE PEUX PAS LE VOIR (pas de GPU ici) — c'est TOI qui valides :*
    lance `./tools/foule-3d.sh 80`, pilote une fenêtre, et confirme : (1) on voit BIEN PLUS que 64 silhouettes,
    (2) ~8 détaillées (avec pseudo) près de toi + la foule en imposteurs, (3) pas de chute de FPS. *Doutes
    que je te signale d'avance : (a) un « pop » visuel possible quand un pair bascule de tier (despawn/respawn) ;
    (b) 512 imposteurs = 512 dessins → si ça rame sur ta carte, on baissera `MAX_AWARE` ou on fera de l'instancing ;
    (c) le binaire est lancé hors nix-shell via `LD_LIBRARY_PATH` capturé — si une fenêtre refuse de s'ouvrir,
    dis-le, on ajustera.* **→ ✓ CONFIRMÉ À L'ÉCRAN (19 juin, capture utilisateur, 80 fenêtres) : LOD parfait,
    > 64 visibles, pas de lag de rendu. D24 FERMÉ.** *(Les 80 fenêtres se sont ouvertes sans souci — niri +
    netcode encaissent une vraie foule.) Spawn éparpillé dans la salle ajouté pour distinguer les tiers à l'œil.*

> **⚙ CONCEPTION DÉTAILLÉE 8.3 (écrite AVANT de coder, 19 juin) — pourquoi la conscience NE scale PAS seule.**
>
> **Le mur qui reste (mesurable).** 8.2 a borné le DÉBIT reçu (plat 80↔160). Mais la conscience distribue
> un budget FIXE entre TOUS les pairs lointains : à N=5000, chaque émetteur donne à chaque conscient
> `80 Hz / ~5000 ≈ 0,016 Hz` = **une mise à jour par MINUTE**. Le débit reste plat (bien), mais la
> FRAÎCHEUR par pair lointain s'effondre en 1/N → la « foule lointaine » devient une purée figée, inutile.
> C'est le résidu noté depuis 8.1. **8.3 le règle en remplaçant N flux individuels lointains par
> QUELQUES flux de RÉSUMÉ.**
>
> **L'idée : des CELLULES, chacune avec un HÔTE agrégateur.**
> - Le monde est découpé en **cellules** (grille ; `cell = (floor(x/T), floor(z/T))`).
> - Chaque cellule a un **hôte élu** (réutilise l'élection déterministe + migration de l'orbe : plus petit
>   id parmi les pairs connus DANS la cellule ; relie **D12** « tout est codé pour un objet » → on généralise).
> - L'hôte produit **UN résumé basse fréquence** de sa cellule : nombre d'occupants + quelques positions
>   représentatives (échantillon) — pas les 500 individus. Il le diffuse à qui regarde cette cellule.
> - Un observateur lointain s'abonne aux **cellules** (un flux résumé chacune) au lieu des N individus.
>   Réception = `focus (plein) + voisinage proche (conscience) + C_cellules (résumés)` = **O(K + C),
>   indépendant de N** — l'invariant tenu jusqu'à 5000, ET avec une fraîcheur correcte des lointains.
>
> **Un point clé qui SIMPLIFIE (et qui répond à R2/D11) :** un résumé est **consultatif**, pas autoritaire
> comme l'orbe. Si deux hôtes résument la même cellule (split-brain de migration en grande foule), on a
> juste **deux flux redondants** — un peu de gaspillage, AUCUNE corruption. Donc **8.3 N'A PAS besoin que la
> migration soit durcie d'abord** (D11/ch.11.2) : il TOLÈRE le multi-hôte par nature. (À l'inverse de l'orbe,
> où le split-brain corrompt l'autorité.) On construit donc 8.3 sur l'élection simple existante, sans béton sur du sable.
>
> **Bonus :** l'index spatial des cellules sert AUSSI à choisir le focus en O(K) au lieu du tri O(N log N)
> de `refresh_focus` (la dette du 8.2a-bis) — on ne trie plus que les pairs des cellules proches.
>
> **Sous-étapes prévues :** **8.3a** grille de cellules + `cell_of(pos)` + index « qui est dans quelle
> cellule » (depuis `peer_pos`) — pur, testé. **8.3b** élection/migration d'hôte de cellule (réutilise la
> machinerie orbe, généralisée) — testé en headless. **8.3c** paquet `KIND_CELL_SUMMARY` (occupants +
> représentants) + émission par l'hôte + ingestion (affichage des lointains via résumé). **8.3d** preuve :
> `crowd 500 → 1000 → 2000` en headless → fraîcheur des lointains correcte ET débit ↓ PLAT, + brancher le
> focus sur l'index (vire le O(N log N)).
> **Doutes d'avance :** (a) coût/justice de l'hôte (il bosse pour les autres → relie D4, l'économie du parent,
> Phase B) ; (b) un hôte malveillant ment sur sa cellule (cache/invente des gens → relie D5/D9 ; corroboration 8.8) ;
> (c) granularité de la cellule (trop grosse = résumé grossier ; trop fine = trop de cellules) — à calibrer par la mesure.

- [ ] 8.3 — **Cellules spatiales + hôte de cellule agrégateur (ce qui fait tenir l'invariant
  à 500/5000).** Partitionner le monde en cellules ; chaque cellule a un **hôte élu** (réutilise
  la machinerie d'autorité+migration de l'orbe, D11/D12) qui : connaît grossièrement qui est
  dans la cellule (rendez-vous décentralisé local), et produit UN **résumé basse fréquence**
  de la cellule (positions échantillonnées / densité + quelques représentants) pour les
  observateurs lointains. Ainsi un nœud lointain reçoit **1 flux agrégé par cellule** au lieu
  de N flux individuels → réception en **O(K_focus + C_cellules)**, l'invariant est tenu. *Relie
  D11/D12 (autorité généralisée) et prépare la Phase B (le parent agrège pour le faible).*
  **Preuve :** à `crowd 500` puis `5000`, couverture ~100 % et débit ↓ **plat** (ne croît pas
  avec N) ; on rejoue 8.0 et on montre la courbe couverture(N) ≈ 100 % à débit constant.

> ### ⚙ REDESIGN 8.3★ — PERCEPTION AUTO-CERTIFIANTE (le chef de cellule RETIRÉ) — écrit le 20 juin 2026 (PAPIER, zéro code)
> *Décidé avec l'utilisateur après la passe de mesure du banc bus (cascade-vs-N). On NE patche PAS le contrôle
> d'hôte de D26 : on retire le besoin de chef. À CHALLENGER ensemble avant de coder ; la mesure tranchera.*
>
> **▶ ÉTAT (21 juin) — le papier est devenu de la MESURE :** **C-diag FAIT** (`DENSITY_MAX` — retirer le chef
> RESTAURE la densité : N=1000 → 89 %, taxe 0 %, débit plat ; à 5000 bridé par la découverte = mur n°2).
> **C-sécu-1a + 1b FAITS** (`CORROB` densité molle corroborée /24 + plancher d'union signée → récup **87 % @1000**,
> cible ≥80 % franchie) ; **C-sécu-2 EN COURS** (échantillons auto-signés, étape 1/5 faite, bloc wire à venir).
> État à jour + plan = l'⏱️ ANCRE en tête de §0. Détail sécurité : bloc **« ÉTAPE C-sécu »** plus bas. Journal mesures : `PLAN_AUTONOME.md`.
>
> **CE QUE LA MESURE A ÉTABLI (banc bus, 20 juin).** Deux murs DISTINCTS de la perception à l'échelle :
> - **Mur n°1 (DOMINANT) — la taxe `émetteur≠hôte` de D26 couche 1.** `ingest_summary` n'accepte un résumé que
>   si l'émetteur == `cell_host(cell)` = **le plus petit id connu DANS la cellule** (une *minimalité*). Or à grand
>   N chaque nœud connaît un sous-ensemble DIFFÉRENT → vues de l'hôte DIVERGENTES → rejet de résumés pourtant
>   légitimes. Mesuré (fenêtres ~45-55 s) : taxe **10 % (N=500) → 24 % (1000) → 61 % (2000) → 68 % (5000)** ;
>   perception/N s'effondre **91 % → 64 % → 10 %**. *C'est le suspect initial de l'utilisateur, CONFIRMÉ.*
> - **Mur n°2 (secondaire, orthogonal) — stagnation de découverte au bootstrap**, seulement à ~5000 (~49 pairs
>   pendant ~40 s puis cascade), robuste au jitter d'horloges → propriété réelle du protocole, pas un artefact de
>   banc. **NON traité par ce redesign** (il vise la perception, pas la découverte).
>
> **LA FAUTE D'ÉLÉGANCE D'ORIGINE.** On a emprunté à l'ORBE son modèle d'**autorité par chef unique élu** (plus
> petit id, déterministe). Pour l'orbe c'est *nécessaire* (un maître doit trancher la physique, sinon l'état se
> corrompt). Pour la PERCEPTION il n'y a **rien à trancher** : voir une foule est un **constat**, pas un acte
> d'autorité. On a donc importé un problème de **consensus sur un chef sous vue partielle** — intrinsèquement
> view-dépendant — là où il n'en fallait pas. La taxe 10→68 % EST cette view-dépendance. Aucune rustine (fenêtre
> d'hôtes, K plus petits ids) ne guérit la cause ; il faut **retirer le besoin de chef**.
>
> **LE PRINCIPE ÉLÉGANT — preuve, pas permission (le keystone du projet, un cran plus haut).**
> - **6.1** : l'identité ne dépend pas d'un annuaire de confiance, elle **se prouve** (id = clé).
> - **9.4b** : on ne croit pas une accusation sur parole, on exige de la **corroboration** (diversité d'IP).
> - **8.3★** : un résumé de cellule n'est pas « la parole d'un chef », c'est un **paquet de PREUVES** —
>   un échantillon de **positions SIGNÉES** `(id, pos, seq)` (auto-certifiantes, comme un état joueur). On ne
>   vérifie plus *QUI* envoie (l'émetteur n'est qu'un **porteur d'octets** : « Own ≠ Relais » du README), on
>   vérifie **les signatures dedans**. *C'est aussi le principe n°7 (mesurer le réel > croire une déclaration).*
>
> **LE MODÈLE (à challenger).**
> - Un résumé = `cell` + **K échantillons signés** `(id, x, z, seq, sceau)` (K borné, ~8) + un **indice de
>   densité** `count` (MOU, non autoritaire). **N'importe quel nœud** peut agréger les états signés qu'il a reçus
>   et en relayer un sous-ensemble. Plus d'élection, plus de `cell_host` à l'ingestion.
> - À la réception : pour chaque échantillon, **vérifier le sceau** (auto-certifiant) + **fraîcheur par id**
>   (`accept_seq`, qu'on a déjà) + **TTL** (D16, qu'on a déjà). On **UNIONNE** les échantillons vérifiés reçus de
>   **plusieurs relayeurs indépendants** (diversité /24, comme 9.4b).
> - **Perception** = `|union des gens signés vérifiés, récents, dans la région|` (l'ensemble des « visages » sûrs)
>   **+** la densité molle corroborée (la « marée » en nombre, pour le LOD lointain).
>
> **POURQUOI ÇA DISSOUT LES DEUX MURS DE PERCEPTION (pas patche : dissout).**
> - **Plus de chef → plus de vue divergente → la taxe `émetteur≠hôte` DISPARAÎT** (mur n°1 supprimé).
> - **Inventer des fantômes = IMPOSSIBLE** : chaque personne du résumé porte SA signature (on ne montre que du réel).
> - **Gonfler le compte = IMPOSSIBLE** si la perception se compte sur les échantillons VÉRIFIÉS (pas sur le `count`).
> - **Cacher des gens (éclipse douce) = BORNÉ** : l'**union** d'évidence signée est *monotone* — un trou-noir peut
>   OMETTRE, jamais retrancher à l'union des autres sources. → **la couche 2 (corroboration) est résolue PAR
>   CONSTRUCTION**, dans le même geste. *(D26 couche 1 + couche 2 fusionnées ; le concept « hôte de cellule
>   autoritaire » est RETIRÉ de la roadmap — il restait une scorie de l'orbe.)*
>
> **ANALOGIE PHYSIQUE (pourquoi c'est « vrai »).** Tu crois qu'une foule lointaine existe parce que (a) tu
> reconnais **quelques visages** assez nets pour être sûr, et (b) **plusieurs points de vue** concordent — pas
> parce qu'un porte-parole officiel te l'a dit.
>
> **L'INVARIANT & LE COÛT (preuve papier — à FERMER avant de coder, c'est le make-or-break).**
> - **Bande passante** : K échantillons signés × ~(32+4+4+8+64) ≈ **~112 o/échantillon** → K=8 ≈ **0,9 Ko/résumé**,
>   à basse fréquence, par cellule SUIVIE. Reste **O(cellules), indépendant de N** (l'invariant tient). À chiffrer
>   contre le débit-cible (~40-46 Ko/s mesuré).
> - **CPU (LE mur à fermer)** : vérifs Ed25519 = `C_cellules × K × fréquence`. Ed25519 verify ≈ 20-50 µs. Il faut
>   borner C, K, fréquence pour rester à quelques %/cœur. *Si ça ne ferme pas → l'idée tombe, et on l'aura su gratis.*
> - **Exactitude** : connaître **toute** la foule individuellement reste **O(N)** (incompressible — l'élégance
>   n'efface pas la physique). On l'assume : LOD = **densité molle** (combien) + **échantillon signé TOURNANT**
>   (quelques visages, renouvelés dans le temps et diversifiés entre relayeurs → l'union grossit). On NE prétend PAS
>   percevoir 5000 individus à coût O(cellules).
>
> **MURS POSSIBLES (honnêteté — à examiner sur le papier).** (1) coût CPU des vérifs (ci-dessus) ; (2)
> **échantillonnage représentatif** : si tout le monde échantillonne les mêmes K, l'union ne grossit pas → il faut
> un tirage TOURNANT/diversifié (par relayeur) ; (3) le `count` mou peut être sur-déclaré → ne JAMAIS le compter
> comme perception, seulement comme indice de rendu ; (4) fraîcheur/TTL pour ne pas unioner des partis (on a
> `accept_seq` + D16) ; (5) le format `KIND_CELL_SUMMARY` grossit → touche le wire (prouvable headless, mais à
> re-vérifier en 3D par l'utilisateur) ; (6) n'adresse pas le mur n°2 (découverte).
>
> **PLAN EN PETITS PAS + CRITÈRE DE SUCCÈS PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT).**
> 1. **Papier** : fermer le calcul de coût CPU/bande passante (ce bloc). Si CPU ne ferme pas → STOP, on garde l'idée mais on ne code pas.
> 2. **Prototype headless** (banc bus) : nouveau résumé à échantillons signés + ingestion par union vérifiée, le
>    `cell_host` RETIRÉ de l'ingestion. Additif/derrière un drapeau pour rester un harnais de régression.
> 3. **Mesure (le résultat décide)** : à N=1000/2000/5000, **la taxe `émetteur≠hôte` doit DISPARAÎTRE** et
>    **perception/N doit remonter nettement** (cible : ne s'effondre plus comme 91→10 %), **à débit ↓ PLAT** et
>    **CPU borné** (mesuré, pas argumenté), **sans rouvrir** la forge de fantômes (test red-team : échantillon non
>    signé / rejoué rejeté). Si la perception ne remonte pas OU le CPU explose → l'idée est réfutée, on le note.
> 4. **Anti-rejeu de l'attaque** : un `attack` qui forge un faux résumé (fantômes/gonflage) doit imprimer « ÉCHOUÉ ».

> ### ⚙ REDESIGN 8.3★ — ÉTAPE C-sécu : DENSITÉ MOLLE CORROBORÉE (PAPIER, écrit le 20 juin 2026)
> *Conçu + auto-challengé pendant que les benchs C-diag tournaient (l'utilisateur s'en remet à moi sur ce choix
> très pointu — cf. mémoire). À CHALLENGER encore à la relecture ; la mesure tranchera. Zéro code ici.*
>
> **CE QUE C-diag A ÉTABLI (banc bus, 20 juin) — le pré-requis de ce bloc.** Sous `DENSITY_MAX=1` (count/cellule =
> MAX vu, non-thrashant ; hôte relâché), la DENSITÉ se restaure : N=1000 → perception moy **895 / max 1000** (89 %
> de N), **taxe émetteur≠hôte = 0 %**, débit ↓ plat (~47 Ko/s) ; N=2000 → moy ~1035 (52 %, montait encore, plafonné
> par la découverte = mur n°2). *Donc le mur n°1 (taxe) est bien la cause, et la densité est RÉCUPÉRABLE une fois la
> taxe retirée.* MAIS `DENSITY_MAX` est un **INSTRUMENT non sécurisé** : le MAX est trivialement gonflable (un menteur
> déclare `count` énorme → densité empoisonnée). C-sécu = rendre cette densité SÛRE sans rouvrir le mur n°1.
>
> **LA TENSION À DÉNOUER.** Les deux extrêmes échouent symétriquement : **MAX** robuste à l'omission (un trou-noir
> qui sous-compte ne baisse pas le max) mais cassé par l'**inflation** ; **MIN** robuste à l'inflation mais cassé par
> l'**omission** (un menteur dit 0 → suppression). Pire : entre HONNÊTES, le vrai count ≈ le plus informé → « crois le
> plus haut » ; la sécurité dit « ne crois pas le plus haut ». Il faut un agrégat robuste qui réconcilie les deux.
>
> **LE PRINCIPE (cohérent 6.1 / 9.4b / 8.3★ : preuve + corroboration diversifiée, jamais une parole).** La densité
> finale n'est pas un scalaire qu'on croit — c'est DEUX choses superposées :
> - **(1) PLANCHER dur vérifiable = l'union signée (étape B réutilisée comme borne basse).** `|union des individus
>   SIGNÉS distincts, frais, dans la cellule|`. Infalsifiable (chaque élément porte sa signature), **monotone** : un
>   trou-noir peut OMETTRE, jamais RETRANCHER à ce que J'ai vérifié. Coût **O(cellules)** (échantillon ≤ K/cellule),
>   pas O(N). Faiblesse assumée : sous-compte (~139 à 2000 mesuré) → c'est un plancher de CONFIANCE, pas le chiffre.
> - **(2) DENSITÉ MOLLE corroborée (le vrai chiffre, sécurisé par diversité /24).** Chaque nœud peut publier
>   `(cell, count, signer_id, seq, sig)` — **SIGNÉ** (anti-forge anonyme) mais **SANS élection d'hôte** (c'est ce qui
>   supprime le mur n°1 : plus de `émetteur==cell_host` → plus de vue divergente → plus de taxe). Le receveur garde
>   **un count par /24 distinct** (le meilleur de ce /24, modèle de cap de 9.4b), et **densité estimée = le Q-ième plus
>   grand count parmi les /24 distincts** (un quantile haut borné par quorum, **Q≈3** comme `ACCUSE_QUORUM`).
> - **Densité retenue = `max(plancher vérifié, densité molle corroborée)`** ; **consultative** (LOD/rendu), jamais autoritaire.
>
> **POURQUOI ÇA DISSOUT LES DEUX ATTAQUES (par construction).**
> - **Inflation** : le `count` géant d'un menteur = **1 seul /24** → 1ᵉ plus haut, mais le **Q-ième est inchangé**.
>   Pour le bouger il faut **Q réseaux /24 distincts** → coût = diversité d'IP (ressource rare, 9.4b), pas du CPU gratuit.
> - **Omission / trou-noir** : déclarer 0 ajoute une valeur basse → ne baisse PAS le Q-ième plus haut, ne touche PAS au
>   plancher vérifié. **La couche 2 (corroboration) est résolue dans le même geste.**
> - **Suppression par flood de bas counts** : ajouter des valeurs basses ne déplace jamais un quantile HAUT, et on ne
>   peut pas « dé-signer » les claims hauts des honnêtes → robuste.
> - **Plus d'élection → la taxe émetteur≠hôte ne peut pas réapparaître** (mur n°1 dissous définitivement). Le count
>   signé réintroduit un *signataire* mais **PAS une autorité** (chacun signe le sien, aucune élection) → ce n'est pas
>   le retour du chef de cellule.
>
> **MES DOUTES (à garder ouverts — c'est mon rôle de les porter).**
> 1. **⚠ Le banc bus NE peut PAS prouver la sécurité /24** (loopback = même /24, ports gratuits pour un attaquant — la
>    limite EXACTE de 9.4b). → headless prouve la **récupération honnête** + la *logique* (tests unitaires) ; l'**anti-
>    inflation /24 se prouve sous le harnais NAT namespaces (vraies IP)**, en réutilisant l'infra 9.4b. À ne JAMAIS confondre.
> 2. **Temps de chauffe à découverte sparse** : avoir Q /24 distincts/cellule suppose assez de sources ; pendant le
>    bootstrap (mur n°2) la densité retombe sur le **plancher vérifié** (conservateur) → dégrade vers la PRUDENCE
>    (sous-compte), jamais vers la sur-confiance. Acceptable, à noter.
> 3. **Récupération < MAX** : le Q-ième plus haut est un cran sous le MAX → on perdra un peu des 89 % (prix de la sécurité).
> 4. **Choix de Q** : trop bas (1-2) = inflation trop facile ; trop haut = sous-compte permanent + temps de chauffe long.
>    Q=3 proposé, **à confirmer par la mesure**. Limite fondamentale (comme tout P2P) : un botnet à IP réellement diverses
>    contourne — et un attaquant qui POSSÈDE toute une cellule (tous Sybils) la définit (densité consultative → toléré).
>
> **PLAN EN PETITS PAS + CRITÈRE DE SUCCÈS PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT).**
> 1. **Papier** : ce bloc (fait). Si un trou apparaît à la relecture → STOP, on ne code pas.
> 2. **C-sécu-1 (headless, banc bus)** : count signé `(cell,count,signer,seq,sig)` SANS élection + agrégation
>    « Q-ième plus haut par /24 » + plancher = union vérifiée. *Critère : densité corroborée ≥ **~80 % de ce que
>    `DENSITY_MAX` atteignait** (on ne perd qu'un cran), débit ↓ PLAT, CPU borné.* Tests unitaires : Q-ième plus haut
>    correct, plancher monotone, anti-rejeu par signer (seq).
> 3. **C-sécu-2 (harnais NAT, vraies IP — réutilise 9.4b)** : red-team **inflation** (attaquant multi-Sybil sur 1 /24
>    gonfle des counts → inflation mesurée ≈ 0 au-dessus de l'honnête → `attack` imprime « inflation ÉCHOUÉE ») ;
>    red-team **omission** (trou-noir déclare 0 → densité perçue par les honnêtes INCHANGÉE).
> 4. Si la récupération s'effondre OU le CPU explose OU l'inflation passe → l'idée est réfutée, on le note (résultat négatif = résultat).

**— Phase B : l'inclusivité, maintenant que la foule est visible (ferme D3, D4, D5) —**

- [ ] 8.4 — **Budget de réception annoncé (water-filling BILATÉRAL).** Chaque joueur publie
  son débit descendant soutenable + son rayon d'intérêt ; les émetteurs en tiennent compte
  (le water-filling, jusqu'ici unilatéral côté émetteur, devient bilatéral). Ferme D3.
- [ ] 8.5 — **Dégradation gracieuse côté receveur** : au-delà du budget, on baisse la
  fréquence des lointains AVANT les proches (paliers focus / proche / foule) — la conscience
  (8.2) se raréfie avant le focus.
- [ ] 8.6 — **Parent agrégateur pour très faibles** : le parent (ou l'hôte de cellule de 8.3)
  reçoit le voisinage et n'envoie au protégé qu'un résumé basse fréquence. Le 0-connexion joue
  *via* son parent. *Réutilise 8.3 ; ferme la moitié « réception bornée » de D3.*
- [ ] 8.7 — **Économie du parent (anti free-riding)** : réciprocité façon BitTorrent
  (choking / optimistic unchoke pondéré par la réputation). Ferme D4.
- [ ] 8.8 — **Anti-censure du parent / hôte de cellule** : multi-parents + détection du « trou
  noir » ; un hôte qui CACHE une partie de la foule est repéré (corroboration gossip de 8.1) et
  contourné. Ferme D5 ; relie D9.

**Ferme :** D22, D23, D24, D3, D4, D5 (+ amorce D9 et D10, relie D11/D12, amorce D16/D17).
**Vérif globale :** à `crowd 200+` (puis 500, 5000), un joueur perçoit ~100 % de la foule
(focus net + lointains dégradés), son débit ↓ reste **borné et plat** quand N grandit, AUCUN
plafond dur à 32, et un hôte/parent malveillant ne peut ni cacher la foule ni couler le faible
(sous netem throttlé à 5 Ko/s, le faible reste fonctionnel ; un nœud égoïste est servi en dégradé).
Un essaim d'attaquants `gossip-flood` est absorbé (D23). **Et dans le VRAI jeu 3D** (lancé par
l'utilisateur, plusieurs fenêtres), on voit bien plus que 64 silhouettes sans chute de FPS (D24).

