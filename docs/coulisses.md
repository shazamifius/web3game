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

*Statut honnête : le mécanisme est prouvé par le code et un test ; la confirmation finale « il ne reste aucune
vraie perte sur le lien réel » se fait en re-mesurant dehors, avec l'instrument corrigé. À suivre dans ce carnet.*
