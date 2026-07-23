# Chantier foule dense & inclusivité — voir la foule sans plafond, sans exploser le débit

> Le plus gros morceau d'architecture : faire qu'au milieu d'une foule de 200, 500, 5 000, **chacun perçoive la
> foule entière** (proches nets, lointains dégradés) **sans plafond dur** et **sans exploser son débit**. On y
> montre, sans les cacher, **les défauts que la mesure a révélés** et **une décision qu'une mesure a imposée
> contre l'intuition de départ** — c'est le cœur de la démarche.
>
> Voir : [les doutes traités](doutes.md) (surtout D22, D3, D29, D30) · [l'état chiffré](etat-du-projet.md) · [l'idée en clair](comprendre-le-p2p.md).

---

## Le problème (D22) : aveugle au-delà de 32

Le passage à l'échelle « gratuit » du cœur achetait son coût constant par un **plafond dur** : le point de
rendez-vous ne présentait que les **32 plus proches**, et les clients ne s'échangeaient rien entre eux. Donc le
**33ᵉ voisin n'existait pas et ne pouvait jamais exister**. À une foule de 200, chacun était **aveugle à 168**.
Or, dans un espace social, **voir la foule, c'est le jeu**.

**L'idée directrice — séparer deux choses que le plafond confondait :**
- le **focus** = les pairs à qui je tiens un lien plein débit (~20 Hz, prédiction, avatar détaillé). Ça **doit**
  rester borné (~8 à 32).
- la **conscience** = tous ceux que je perçois en basse fidélité (des centaines, en silhouettes). Ça ne doit
  **pas** être plafonné.

**L'invariant à tenir (le piège à ne jamais se cacher).** Le coût de réception doit rester **indépendant de la
taille de la foule**. La preuve de réussite n'est donc pas « la couverture monte », mais **« la couverture monte
ET le débit reçu reste plat quand la foule grandit »**. (Augmenter le plafond serait tricher : ça déplacerait le
mur, ça ne le casserait pas.)

## Étape 1 — Découverte décentralisée : le 33ᵉ devient apprenable

Le point de rendez-vous cesse d'être l'énumérateur autoritaire et redevient un simple **amorçage**. Ensuite,
chaque pair annonce à bas débit, à ses voisins, **quelques autres pairs qu'il connaît** (« cartes de visite ») —
une découverte par bouche-à-oreille, sans serveur qui énumère, sans plafond de vision.

**Mesuré :** à une foule de 200, la couverture passe de **16 % à 98 %** (chacun finit par apprendre les 200) — et
**le débit reçu ne grandit pas** de 200 à 500 (il reste plat, voire baisse). Le plafond de 32 était bien
arbitraire ; la découverte était le seul mur, et il tombe sans rouvrir l'explosion de trafic.

**La rançon honnête du gossip (D23).** Ce bouche-à-oreille a ouvert une porte : on apprenait des « cartes »
non prouvées → un attaquant pouvait polluer les tables ou **réfléchir un flot vers une victime**. On l'a refermée
**avant** de construire dessus (règle : pas de béton sur du sable), par quatre défenses : preuve de travail exigée
sur chaque carte, interdiction d'écraser l'adresse d'un pair connu, abandon du perçage non corroboré après ~10 s,
et limitation par source. **Prouvé par un vrai attaquant** (`gossip-flood`) : **0 perçage réfléchi** reçu par la
cible, même quand l'attaquant mine sans cesse de nouvelles identités (la rafale est bornée à ~10 s, mesuré).

## Étape 2 — Deux tiers nets/flous, et un défaut révélé par la mesure

En foule dense, tout le monde est à ~même distance → la pertinence par distance ne discrimine plus → **tout le
monde devient également flou**. Il fallait faire **émerger des proches nets** : réserver le gros du budget à un
**focus** de quelques pairs (plein débit), et ne saupoudrer que des miettes sur la **conscience** (le reste, en
basse fidélité).

**Une métrique a fait son travail — elle a démasqué un défaut.** En mesurant les pairs réellement *entendus*
(pas seulement *connus*), on a vu qu'à une foule de 160, le focus s'effondrait à **0,2** (au lieu de ~8) — et ça
ne se rétablissait pas avec le temps : **un vrai défaut, pas un artefact**. Cause : le focus **« churnait »** —
recalculé à chaque instant, l'ensemble des « 8 plus pertinents » changeait sans cesse → aucun lien plein débit
soutenu. La correction : un **focus collant** (avec hystérésis — on ne remplace un membre que si un autre est
nettement plus pertinent).

**Résultat — l'invariant tenu, mesuré.** Quand la foule double (80 → 160) : le **focus reste borné** (~9), la
**conscience grandit avec la foule** (68 → 134, le LOD de toute la foule), et le **débit reçu reste plat** (~44 →
~40 Ko/s). C'est exactement la preuve recherchée : la couverture monte sans que le débit explose.

**Confirmé à l'écran.** Le rendu à deux tiers (proches détaillés + foule d'imposteurs bon marché) a été vérifié
sur une vraie foule (80 fenêtres ouvertes en même temps) : **bien plus de 64 silhouettes visibles, sans chute de
performance** (D24).

## Étape 3 — Faire tenir la foule lointaine, et une décision imposée par la mesure

À très grande échelle, même à débit plat, la **fraîcheur** d'un lointain s'effondre (une mise à jour par minute) :
la foule lointaine devient une purée figée. L'idée : remplacer N flux individuels lointains par **quelques flux de
résumé** par région (« cellule »).

**La mesure a tranché contre la conception de départ.** Le premier design donnait à chaque cellule un **chef élu**
(le plus petit identifiant connu dans la cellule), qui produisait le résumé. Mesuré au banc : c'était **le mur
dominant**. Comme chaque nœud connaît un sous-ensemble différent de la cellule, les nœuds n'étaient pas d'accord
sur « qui est le chef » → ils **rejetaient des résumés pourtant légitimes**. La perception s'effondrait de **91 %
à 10 %** à mesure que la foule grandissait.

**La faute d'élégance, et sa correction.** On avait emprunté à l'autorité d'un objet (où un maître *doit* trancher
la physique) un modèle de **chef unique élu**. Mais **percevoir une foule n'est pas un acte d'autorité — c'est un
constat.** Il n'y a rien à trancher. On a donc **retiré le besoin de chef** : un résumé n'est plus « la parole d'un
chef », c'est un **paquet de preuves** — un échantillon de **positions signées** (auto-certifiantes, comme l'état
d'un joueur). On ne vérifie plus *qui* envoie, on vérifie **les signatures à l'intérieur**, et on **réunit** les
échantillons vérifiés reçus de **plusieurs sources indépendantes**.

Ce geste **dissout** les attaques par construction : inventer des fantômes est impossible (chacun porte sa
signature), et **cacher des gens est borné** (réunir des preuves est *monotone* — un menteur peut omettre, jamais
retrancher à ce que les autres ont vu).

**Résultat mesuré.** Une fois le chef retiré, la densité se restaure : **~87 % à 1 000 nœuds, à débit reçu plat**.
Au-delà (2 000, 5 000), la perception est bridée non par ce mécanisme mais par un **second mur, distinct** : la
**lenteur de la découverte** à l'amorçage (49 % à 5 000, et qui montait encore en fin de mesure).

**La densité, sécurisée sans rouvrir le mur.** Le « nombre » d'une cellule pourrait être gonflé par un menteur. La
parade reprend le principe anti-éclipse : on garde **un compte par sous-réseau distinct** et on retient un
**quantile haut** (le 3ᵉ plus grand) → gonfler exige des **adresses IP diverses** (ressource rare), pas du calcul
gratuit ; et omettre ne baisse jamais ce quantile. (À ce stade, banqué : on n'y revient qu'au moment du test sur
de vraies adresses IP.)

## Ce que ces résultats ne prouvent pas (honnêteté de méthode)

- La **couverture** compte les pairs *connus*, pas toujours *entendus à temps* — c'est optimiste.
- La **fraîcheur ressentie** d'un lointain n'a pas été chiffrée en direct.
- **« 55 000 » n'est jamais mesuré directement** : densité mesurée à ~1 000–2 000, extrapolée au-delà.
- L'**anti-inflation par sous-réseau** n'est prouvé qu'en logique et en simulation ; les **vraies adresses IP**
  diverses (et le résidu « botnet ») restent à tester.

## Ce qui reste — l'inclusivité (phase suivante)

Maintenant que la foule est visible, le front suivant est de **n'exclure personne** : aire d'intérêt **bilatérale**
(le receveur annonce un budget que les émetteurs respectent, D3), **dégradation gracieuse** (raréfier les lointains
avant les proches), **parent qui agrège** pour les très faibles, et **détection du relais « trou noir »** (D4/D5).
S'y ajoute le redesign **par pertinence sociale** plutôt que par simple proximité (D29) et le **détail adaptatif**
continu (D30).

---

*Faits techniques tenus à jour au fil des mesures. Les chiffres de coût et de débit : [revue de l'état](etat-du-projet.md).*

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Chantier foule dense** — une étape des parcours **⚙️ Le code** et **🧭 Tout comprendre**.

**Continuer le fil :**
- ⚙️ *Le code* · 🧭 *Tout comprendre* → **[Chantier robustesse](chantier-robustesse.md)**

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
