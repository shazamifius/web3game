# Les coulisses — nos problèmes, et comment on les a résolus

> **Ce document est un peu inclassable**, et c'est volontaire. Ce n'est ni un mode d'emploi, ni une
> liste de fonctionnalités. C'est un **carnet de bord** : les murs sur lesquels on a buté, les fausses
> pistes qu'on a suivies, et **par quoi on est passé pour comprendre ce qui se jouait vraiment**.
>
> Pourquoi le garder ? Parce que sur ce projet, la règle est simple : **une preuve = un chiffre
> reproductible, et un doute n'est jamais caché.** Les meilleures avancées ne sont pas venues de code
> qu'on a écrit, mais de **questions honnêtes** (« et si on se trompait ? ») et de **mesures qui ont
> corrigé le plan**. Ce carnet raconte ces moments-là. Il grandira au fil des enquêtes.

---

## Enquête n°1 — « le relais perd 89 % des paquets » (28 juin 2026)

### Le symptôme
Notre instrument de mesure (un agent déployé sur de vraies machines, derrière de vrais réseaux) rapportait
un chiffre alarmant : sur le **chemin relais** — celui qu'emprunte une machine derrière un réseau verrouillé
(« CGNAT ») qui ne peut pas se connecter en direct —, on mesurait **~89 % de paquets perdus**. À l'écran, les
avatars distants paraissaient saccader, presque morts. Le doute fondateur du projet (« et si on avait bâti une
belle infrastructure dans laquelle deux humains ne se retrouvent jamais *vraiment* ? ») pointait son nez.

### Les fausses pistes
On a d'abord soupçonné le **plafond anti-abus du relais** : peut-être bridait-il l'émetteur. Vérification dans le
code : le relais autorise **30 paquets/seconde**, l'émetteur n'en produit que **20**. Marge suffisante. **Hypothèse
écartée — par la mesure, pas par l'intuition.** On a aussi tenté un correctif « force brute » : envoyer chaque
position en **triple** (redondance). Mesuré en réel : la perte tombait de 89 % à 68 %, exactement comme le prédit le
calcul (`0,89³ ≈ 0,68`). Ça « marchait »… mais ça ne nous disait toujours pas **pourquoi** on perdait autant.

### L'enquête
Plutôt que d'empiler des pansements, on est retournés lire le code — le **juge neutre**. Trois questions : comment
l'instrument calcule-t-il la « perte » ? Le numéro de série des messages est-il global ou par destinataire ?
L'émetteur bride-t-il vraiment ce qu'il envoie aux machines lointaines ?

### La révélation
Le code a tranché, et le verdict était inattendu : **il n'y avait quasiment aucune perte.**

Notre moteur de présence répartit son attention : il envoie le **plein débit (20 fois/seconde)** aux ~8 personnes
les **plus pertinentes** autour de soi, et un simple **filet (2 fois/seconde)** à tous les autres — volontairement,
pour économiser la bande passante. C'est le cœur de notre approche « foule à coût borné ».

Or notre instrument comparait ce que la machine lointaine recevait (**2/seconde**) au compteur **global de
l'émetteur (20/seconde)**. Il voyait 2 messages sur 20 arriver et concluait « 90 % perdus » — alors que les 18
autres **n'avaient jamais été envoyés, à dessein**. Le « 89 % de perte » n'était pas une panne réseau : c'était
**notre propre économie de bande passante, mal mesurée**. Le relais n'était pas cassé. Le lien CGNAT allait peut-être
très bien.

### La correction (élégante, pas brute)
On a appris à l'instrument à **séparer deux choses qu'il confondait** : « pas envoyé exprès » et « envoyé puis
perdu ». Il **infère désormais la cadence réelle** (le rythme effectif des messages reçus) et mesure la perte
**relative à cette cadence** : un intervalle normal = sain ; un intervalle « double » = un message vraiment perdu.
Deux chiffres distincts remontent maintenant — la perte **apparente** (vs plein débit) et la perte **réelle**. Un
test déterministe le prouve : un flux bridé **sans aucune perte** affiche bien `loss ≈ 88 %` *apparent* mais
`vraie perte = 0`.

### Ce que ça nous a appris (la preuve a redessiné le plan)
1. **Le vrai sujet n'est pas la perte, c'est la pertinence.** Si une personne avec qui l'on **interagit** se
   retrouve classée « lointaine », elle reçoit le filet (2/s) et paraît morte. Le bon correctif n'est donc pas
   d'envoyer plus fort, mais de **donner le plein débit à qui compte vraiment**, même loin. C'est notre chantier
   suivant.
2. **Pour le « filet » (2/s), on n'enverra pas des positions figées mais des *trajectoires*** — une description
   compacte du mouvement, rejouée de façon vivante chez les autres. Quelques messages par seconde suffisent à
   décrire une courbe, et c'est **naturellement résistant à la perte**.
3. **Un correctif force brute peut masquer la vraie question.** La redondance « marchait » en chiffres, mais
   réparait un faux problème. On ne construit une mécanique complexe que **lorsqu'une mesure l'exige** — jamais
   « au cas où ».

### La suite — vérifié dehors (28 juin 2026)
Le carnet disait « à suivre » : voici la suite. On a **re-mesuré dehors**, avec l'instrument corrigé, sur de **vrais
liens distants** (des volontaires sur plusieurs pays, certains derrière le NAT le plus dur, le CGNAT). Le verdict
neutre du journal serveur : présence **vivante**, fraîcheur **p95 ~200–335 ms** (sous notre seuil de 500 ms),
**perte réelle ~0**, verdict « vivant ». Autrement dit : une fois l'instrument honnête, **le lien CGNAT allait bien**
— c'était bien notre mesure qui mentait, pas le réseau.

Deux durcissements tirés de l'enquête sont restés, parce qu'ils servent **un vrai cas, pas un faux** : une
**tolérance au silence** côté rendez-vous portée de **5 s à 20 s** (un lien CGNAT se ré-enregistre sans cesse — à
5 s on l'évinçait à tort), et une **redondance d'émission** *optionnelle* (envoyer un état en double via le relais),
gardée **sous le coude** pour les cas avérés — pas allumée « au cas où ».

*Statut honnête : le mécanisme est prouvé par le code, un test déterministe **et** une mesure dehors. Ce qui reste
ouvert n'est plus la perte, mais la **pertinence** (donner le plein débit à qui compte, même loin) — le chantier
suivant. Et ça mesure le **substrat** (la présence transportée), pas encore le **ressenti** d'humains qui jouent
ensemble (D27, « la forteresse vide », s'allège sans se fermer).*

---

## Enquête n°2 — « certains liens lointains sont *morts* » (28 juin 2026)

### Le symptôme
L'instrument corrigé tournait dehors, et la plupart des liens distants étaient **vivants**. Mais certains
revenaient avec un verdict brutal : **`MORT (>500 ms)`**, fraîcheur p95 ~950 ms. Le doute repointait : aurait-on,
là, de vrais liens qui ne passent pas ?

### La belle hypothèse (la « saison 2 » de l'enquête n°1)
Le réflexe, après la n°1 : *c'est encore notre propre économie de bande passante, mal mesurée.* Notre moteur
envoie le **plein débit** à un petit cercle (focus) et un **filet basse fréquence** (~2/seconde, palier
« conscience ») à tous les autres. Un pair dans ce filet est **frais à ~500 ms par conception** — le seuil plat
« > 500 ms = mort » le condamnerait à tort, exactement comme le « 89 % perdu » de la n°1. Élégant… et on a corrigé
le verdict dans ce sens.

### Le twist (la mesure a, encore, corrigé le récit)
Sauf que la donnée disait autre chose. Les fenêtres « mortes » n'avaient pas une cadence basse (un filet à 2/s) —
elles avaient **zéro réception**. Le même lien alternait, sur un cycle de ~7 secondes, entre **plein débit (frais
~200 ms)** et **silence total (~950 ms)**. Pas « basse fidélité par conception » : **bimodal — tout ou rien.** Le
filet « conscience » n'atteignait tout simplement **pas** ces pairs hors-focus.

### Ce qu'on a corrigé — et ce qu'on a, honnêtement, seulement *révélé*
On a rendu le verdict **conscient de la cadence ET de la réception**, en trois états au lieu de deux :
**vivant** · **lointain (basse fidélité)** — bridé exprès, donc *vivant* même au-delà de 500 ms · **mort
(silencieux)** — connu mais **zéro paquet reçu** cette fenêtre. Et l'instrument **affiche désormais toujours la
réception** (`recv:0` = silence rendu visible), pour qu'on ne confonde plus « volontairement discret » et
« réellement muet ».

Mais soyons nets : **ça n'a rien réparé du fond.** Ça a rendu l'instrument *honnête*, et du coup le vrai mur
apparaît en pleine lumière : **pourquoi un pair hors-focus devient-il complètement silencieux**, au lieu de rester
vivant à basse fidélité via le filet ? C'est une question d'**inclusivité de l'aire d'intérêt** (le faible, le
lointain, doivent rester *perçus*) — le prochain chantier de fond, côté cœur. *Comme la n°1 : la meilleure avancée
n'est pas le code écrit, c'est la question rendue visible. À suivre.*

---

## Enquête n°3 — « le 4G n'était pas le mur qu'on croyait, et la redondance n'est pas gratuite » (29 juin 2026)

### Le symptôme
Pour un lien trop fermé pour se connecter en direct, on passe par un **relais**, et un relais **perd des paquets**.
Idée de bon sens pour compenser : envoyer chaque position **en double**. La théorie est même rassurante — si la perte
est aléatoire de probabilité `p`, deux copies ne se perdent toutes les deux qu'avec une probabilité `p²`, trois avec
`p³`, etc. Sauf qu'à l'essai sur un **vrai lien mobile**, la redondance n'a **pas** aidé : elle a **empiré** la perte.
Le calcul disait « mieux », la réalité disait « pire ». Il manquait quelque chose.

### Les deux croyances de départ
On portait, sans les avoir vérifiées, deux hypothèses confortables : **(1)** « un lien mobile 4G/5G, c'est du NAT
*symétrique* — le cas dur, à relayer d'office » ; **(2)** « dupliquer aide toujours, c'est juste une question de
combien de copies ». Deux croyances raisonnables… et toutes les deux fausses, comme la mesure allait le montrer.

### Le twist (la mesure a, deux fois, corrigé le récit)
On a construit une **sonde de lien** qui tourne dans l'instrument, sans aucune dépendance externe. Elle fait deux
choses. D'abord, elle détermine le **type de NAT** en interrogeant deux serveurs publics depuis une seule socket
(même adresse publique vue des deux côtés = NAT *perçable* ; adresse différente = *symétrique*). Verdict : le
téléphone grand public testé était **perçable en direct** — pas symétrique du tout. **Croyance n°1 : réfutée.**

Ensuite, la sonde **caractérise la perte** : elle envoie une courte rafale à **débit croissant** et regarde comment
le lien réagit. Sur la fibre, tout reste plat (~28 ms, ~0 perte) : lien *sain*. Sur le lien mobile, la latence
**gonfle avec le débit** — de ~60 ms à plus de **100 ms** — puis la perte apparaît au débit le plus haut. Ce n'est
pas du bruit aléatoire : c'est de la **congestion** (le lien sature, ses tampons débordent). Et là tout s'éclaire :
quand la perte vient de la saturation, **deux copies = deux fois plus de trafic = encore plus de saturation**. La
formule en `pᴷ` suppose des pertes *indépendantes* ; sur un lien congestionné, elles ne le sont pas. **Croyance
n°2 : réfutée.**

### La correction — mesurer, puis n'aider que là où ça aide
La réponse n'est pas « plus de copies » ni « jamais de copies », mais **adaptatif** : chaque nœud lit sa propre
sonde et **ne duplique que s'il a mesuré une perte aléatoire avec de la marge**. On l'a vu se produire en conditions
réelles — un lien mobile congestionné a, **de lui-même**, choisi de **ne pas** dupliquer (ne pas aggraver ce qui
sature). Restait à prouver l'autre moitié : que sur une *vraie* perte **aléatoire**, dupliquer aide bien. Comme aucun
de nos liens de test n'avait ce profil, on a injecté une perte aléatoire connue avec l'outil du noyau (`tc netem`),
dans un espace réseau jetable (sans toucher la machine), et fait passer le **vrai** mécanisme à travers. Résultat :
perte **30 % → 9 %** avec deux copies, **2,8 %** avec trois — exactement la courbe `pᴷ`. Et sur une perte *corrélée*
(en rafale), le gain s'effondre, comme prévu. La boucle est bouclée : on sait **quand** la redondance paie, et on ne
l'allume que là.

### Ce que ça nous a appris
1. **La mesure prime sur l'intuition — encore.** Deux croyances de bon sens (« 4G = symétrique », « dupliquer aide
   toujours »), deux fois corrigées par un instrument qu'on a pris la peine de construire. C'est devenu la signature
   de ce projet.
2. **Un réseau qui se connaît lui-même.** Avant d'agir sur un lien, on le **mesure** : type de NAT, latence, gigue,
   nature de la perte. Une stratégie aveugle aide au hasard ; une stratégie qui mesure d'abord aide **juste**.
3. **La redondance est un outil ciblé, pas une baguette magique.** Elle sauve un lien à perte aléatoire et nuit à un
   lien saturé : la même action, deux effets opposés selon le terrain. D'où l'importance de **diagnostiquer avant de
   soigner** — et le doute **D36** (la vraie diversité des connexions) reste grand ouvert.

---

## Enquête n°4 — « la moitié de ces liens “morts” étaient vivants » (29 juin 2026)

### Le symptôme
L'instrument tournait dehors, et une part obstinée des liens distants revenait avec le verdict **`MORT
(>500 ms)`** : leur dernière nouvelle était trop vieille pour le seuil « vivant » du projet. Après les
enquêtes n°1 et n°2, on avait appris la méfiance — mais cette fois le verdict semblait solide : 500 ms,
c'est 500 ms.

### La fausse évidence
Un seuil de fraîcheur juge un **symptôme** (l'âge de la dernière nouvelle), pas une **cause**. Or deux
réalités très différentes produisent le même symptôme : un lien **réellement cassé** (qui perd la majorité
de ses paquets), et un lien **parfaitement sain mais volontairement peu rafraîchi** (notre propre économie
de bande passante envoie un simple « filet » basse fréquence aux pairs lointains — c'est un choix, pas une
panne). Le verdict plat condamnait les deux d'un même mot.

### L'enquête — rejouer, plutôt que re-mesurer
On a corrigé le verdict pour qu'il lise **la perte réelle** (la mesure qui distingue « pas envoyé exprès »
de « envoyé puis perdu », héritée de l'enquête n°1) : un lien vieux mais qui livre fidèlement ce qu'il
promet est *lointain* — vivant en basse fidélité —, pas *mort*. Et pour le prouver, plutôt que lancer une
nouvelle session live (dont chaque comparaison souffre de facteurs parasites : foule différente, moment
différent — une leçon déjà payée), on a **rejoué le nouveau verdict hors-ligne sur 685 liens réels déjà
enregistrés** : mêmes données, seul le code du jugement change. Zéro variable parasite.

### La révélation
**52 % des « morts » (66 liens sur 127) étaient vivants** — des liens sains, simplement bridés exprès.
Et les deux populations se séparent **sans recouvrement** : les réhabilités ont une perte réelle médiane
de **0 %** (tous entre 0 et 20), les vrais morts une médiane de **60 %** (tous entre 21 et 90). Le seuil
de 20 % ne coupe pas une distribution continue en deux : il tombe dans le **vide entre deux nuages** — il
sépare deux réalités physiques distinctes. *(La formule exacte du verdict à trois états : [les mesures, en
équations](les-mesures.md).)*

### Ce que ça nous a appris
1. **Un verdict doit lire la cause, pas le symptôme.** La fraîcheur seule mélangeait « cassé » et
   « discret par conception » ; la perte réelle les sépare proprement.
2. **Rejouer sur des données constantes vaut mieux que re-mesurer.** Une nouvelle session aurait comparé
   deux moments différents ; le rejeu compare deux *jugements* sur la même réalité — la preuve la plus
   propre qu'on ait produite.
3. **Troisième fois que le menteur était l'instrument, pas le réseau** (après le « 89 % de perte » et les
   « silencieux » de l'enquête n°2). C'est devenu un réflexe : avant de croire un chiffre alarmant, on
   instruit le procès de la mesure elle-même.

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Les coulisses** — une étape des parcours **🔎 Juger vite** et **🧭 Tout comprendre**.

**Continuer le fil :**
- 🧭 *Tout comprendre* → **[Le journal de développement](journal.md)**
- 🔎 *Juger vite* → ✓ vous êtes au bout de ce parcours. Et la suite ? ⚙️ [le code](ARCHITECTURE.md) ou 🧭 [tout comprendre](README.md).

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
