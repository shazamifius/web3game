# Le « sans serveur », en clair

> L'idée centrale du projet, **en mots simples** : on part d'un jeu en ligne classique, on enlève le serveur
> central, et on regarde — problème par problème — par quoi le remplacer. Sans jargon.
>
> Pour le code, voir [l'architecture](ARCHITECTURE.md) ; pour les mesures chiffrées, la [revue de l'état](etat-du-projet.md).

---

## Le point de départ : le modèle « normal »

Presque tous les jeux en ligne fonctionnent pareil : un **serveur central** au milieu. Ton jeu lui parle, il parle
aux autres joueurs, il détient la vérité (ta position, ton score), il tranche les conflits. C'est simple, et ça
marche très bien.

Mais ce serveur a un prix :
- **il coûte cher** — plus il y a de joueurs, plus il faut de machines puissantes ;
- **il appartient à quelqu'un** — qui peut tout voir, tout couper, fermer le service du jour au lendemain ;
- **c'est un point unique de panne** — s'il tombe, tout le monde tombe avec lui.

La question de ce projet est simple : **et si on l'enlevait ?**

## Ce qui casse quand on enlève le serveur (et comment on le répare)

Sans serveur, les joueurs doivent se débrouiller **directement entre eux** : c'est le « **pair-à-pair** » (P2P).
Six problèmes apparaissent alors. Les voici, un par un, avec leur solution.

### 1. « Qui es-tu ? » — sans compte sur un serveur

Normalement, c'est le serveur qui sait que ce compte, c'est toi. Sans lui, comment prouver son identité ?

**La solution : ton identité, c'est une clé.** Comme une clé d'appartement, mais numérique (exactement le principe
d'une **clé SSH**). Tu **signes** chaque message avec ; n'importe qui peut vérifier la signature, mais personne ne
peut l'imiter. Du coup **personne ne peut se faire passer pour toi**, et **aucun annuaire central ne « décide » qui
tu es** : ton identité se prouve toute seule.

### 2. « Comment se trouver ? » — sans annuaire central

Deux joueurs qui ne se connaissent pas doivent quand même réussir à se parler. Sans annuaire, comment ?

**La solution : un simple « point de rendez-vous » qui fait les présentations** — comme quelqu'un qui, à une fête,
présente deux invités, puis s'efface : ils discutent ensuite directement. Une fois les présentations faites, on
pourrait **éteindre le rendez-vous**, la partie continue. *(C'est le seul morceau encore un peu « central » — l'astérisque honnête du « sans serveur ». On y revient à la fin.)*

### 3. « Les box bloquent tout » — le NAT

Ta box Internet **cache** ton ordinateur et **bloque** les connexions qui viennent de l'extérieur (c'est le « NAT »,
et c'est normal : ça te protège). Problème : pour du pair-à-pair, on a justement besoin que les autres puissent
t'atteindre.

**La solution : le « hole punching »** — une petite ruse où les deux box ouvrent un passage **au même moment**, par
surprise. Ça marche dans la plupart des cas. Et quand c'est impossible (certaines connexions mobiles), un autre
joueur sert de **relais** : il recopie les paquets, sans pouvoir les modifier (ils restent signés — cf. point 1).

### 4. « On ne peut pas parler à 55 000 personnes » — l'aire d'intérêt

Si chacun parlait à tout le monde, le trafic exploserait (à 55 000 joueurs, c'est ingérable).

**La solution : tu ne parles à plein débit qu'à tes voisins immédiats (~32).** La foule lointaine, tu la perçois en
**basse fidélité** (un résumé, des silhouettes), pas en détail. Résultat : ton coût réseau reste **borné** — il ne
dépend **pas** du nombre total de joueurs, seulement de ton petit voisinage. *(Les chiffres : [revue de
l'état](etat-du-projet.md) — ~34 Ko/s par nœud, stable quel que soit N.)*

### 5. « Qui a raison sur un objet partagé ? » — sans arbitre

Deux joueurs attrapent le même objet « en même temps ». Sans serveur-arbitre, qui gagne ?

**La solution : « Own + Shields ».** Pour chaque objet contesté, **un seul** joueur fait autorité à un instant donné
(l'« Own »). Les autres (« Shields ») **vérifient** ce qu'il annonce et peuvent le **destituer** s'il triche.
Si l'Own part ou se tait, l'autorité **migre** à un autre. Pas de serveur central : juste des règles que tout le
monde applique de la même façon.

### 6. « Qui empêche de tricher ? » — sans modérateur

Pas de modérateur central. Alors qui arrête les tricheurs ?

**La solution : plusieurs couches.** Les **signatures** (point 1) empêchent déjà de falsifier les messages des
autres. S'y ajoutent : des **règles de plausibilité** (on ne se téléporte pas à l'autre bout de la carte d'un coup),
une **réputation partagée** (un tricheur repéré est mis en sourdine pour tous), et le principe de **corroboration**
(on ne croit pas un joueur sur parole quand il parle d'un *groupe* : on recoupe avec ce que d'autres rapportent).

## Une précision : ici, « web3 » ne veut pas dire crypto

Le mot « web3 » est souvent associé aux cryptomonnaies. **Dans ce projet, ça n'a rien à voir.** « web3 » est pris au sens
**décentralisé** + **identité que tu possèdes vraiment** — et c'est tout. **Pas de token, pas de blockchain, pas de
spéculation.** La seule « crypto » présente, ce sont les **signatures cryptographiques** (comme une clé SSH) pour
prouver l'identité — la même chose qui sécurise déjà ta connexion à un site web. Rien à voir avec de l'argent.

## Alors, « sans serveur », ça veut dire quoi exactement ?

Le sens honnête : **pas de serveur central qui détient la vérité et arbitre le jeu.** La logique vit chez les
joueurs. **L'astérisque** (point 2) : un petit point de rendez-vous aide encore aux **présentations** au démarrage —
on peut l'éteindre une fois les joueurs connectés, et le décentraliser totalement reste un chantier ouvert. Donc la
formule exacte est « **pas de serveur central qui fait autorité** », pas « zéro serveur ». *(Cette honnêteté sur les
limites, c'est une règle du projet — cf. les [murs assumés](etat-du-projet.md).)*

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Le « sans serveur », en clair** — la première étape des parcours **🌱 Découvrir** et **🧭 Tout comprendre**.

**Continuer le fil :**
- 🌱 *Découvrir* · 🧭 *Tout comprendre* → **[L'état du projet, chiffré](etat-du-projet.md)**

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
