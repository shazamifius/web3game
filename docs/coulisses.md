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
