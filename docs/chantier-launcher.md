# Chantier launcher — passer d'un monde à l'autre

*[English version](en/chantier-launcher.md)*

> Un réseau pair-à-pair, ça ne se voit pas. On peut prouver qu'il transporte une présence vivante entre
> machines distantes (c'est fait) sans avoir encore rien qui **ressemble à une plateforme**. Or la promesse
> lointaine du projet — un univers de mondes où l'on passe de l'un à l'autre — a besoin, à un moment, d'un
> **geste concret** : entrer dans un monde, franchir un portail, se retrouver dans un autre. Cette page
> montre ce premier geste, comment il est orchestré sans jamais laisser d'écran noir, **et ce qu'il ne
> prouve pas encore**.
>
> Voir : [l'état chiffré](etat-du-projet.md) · [les coulisses](coulisses.md) (enquête n°5) · [le registre des doutes](doutes.md).

---

## 1. Le problème : deux mondes, aucun trou entre les deux

Faire tourner deux jeux séparément est facile. Le difficile, c'est la **transition** : un joueur qui traverse
un portail ne doit **jamais** voir le vide. Pas d'écran noir, pas de fenêtre morte qui reste, pas de « les
deux mondes tournent en même temps et rament ». La bascule doit être **invisible** — sinon l'illusion d'un
univers continu s'effondre au premier passage.

Et comme tout le reste du projet, ça se fait **sans moteur de jeu central qui arbitre** : c'est une petite
application locale, le **launcher**, qui tient les mondes et orchestre le passage.

## 2. Trois pièces

| Pièce | Ce que c'est |
|---|---|
| **Le launcher** | Une application native (Rust, faite main). Elle lance les mondes, dessine l'interface, et joue le rôle de **régisseur** de la bascule. C'est le seul chef d'orchestre. |
| **Deux mondes Unreal** | Un **hub** (le carrefour, avec un portail) et une **île** (le monde d'arrivée). Deux jeux distincts, chacun branché au cœur réseau par le pont *sidecar*. |
| **Un protocole de navigation** | Une petite conversation entre chaque monde et le launcher (une socket locale) : « je suis prêt », « affiche-toi », « mets-toi en veille », « ferme-toi ». Le launcher décide ; les mondes obéissent. |

## 3. La bascule, et son invariant

Quand le joueur franchit le portail, le hub le dit au launcher. Le launcher ne se contente pas de tuer le
hub et de lancer l'île — ce serait le trou noir garanti. Il suit une **machine à états** avec une règle
d'or, un invariant qui n'est **jamais** violé :

> **On ne ferme jamais l'ancien monde tant que le nouveau n'a pas prouvé qu'il affiche une image.**

Concrètement : le launcher prépare l'île en arrière-plan pendant que le hub reste visible ; il attend que
l'île annonce « j'ai rendu ma première image » ; **alors seulement** il promeut l'île et met le hub en
veille. Si l'île n'y arrive pas (elle plante, elle traîne trop), le hub reste — on ne se retrouve jamais
devant rien. C'est la même philosophie que le reste du projet : **ne jamais casser ce qui marche avant que
le remplaçant ait fait ses preuves.**

## 4. Ce qui a été prouvé

- **Fluide en jeu réel.** Le juge, ici, n'est pas un chiffre : c'est l'**œil**. En conditions réelles, la
  bascule a été jugée fluide — on franchit le portail, on est sur l'île, sans à-coup perçu.
- **La machine à états tient sous la pression.** Sa logique a été mitraillée par une simulation : **200 000
  pas** de séquences de bascule aléatoires, quatre invariants vérifiés à chaque pas (dont « jamais deux
  mondes actifs », « jamais fermer l'ancien avant l'image du nouveau »), **zéro violation** — et on a prouvé
  que le filet *mordait* en réintroduisant exprès les bugs corrigés.

Mais cette deuxième preuve est aussi une **leçon d'humilité**, et elle mérite d'être racontée entière : ces
200 000 pas verts ont un temps validé une garantie **fausse**, parce que la simulation reposait sur une
hypothèse erronée du monde réel (l'exécutable d'un paquet Unreal est un simple *amorceur* qui lance le vrai
binaire puis se termine — le launcher « tuait » donc un processus déjà mort). Seule une campagne sur la
**vraie machine** l'a montré. Le récit complet est dans **[les coulisses, enquête n°5](coulisses.md)** — il
dit mieux que tout le reste comment ce projet travaille.

## 5. Ce qui reste, honnêtement

- **L'île est un cul-de-sac.** On y arrive, mais il n'y a **pas encore de portail de retour** : la seule
  sortie est de fermer la fenêtre. Un aller sans retour n'est pas une plateforme — c'est le prochain geste.
- **Le contenu est épars.** Les mondes sont des décors de démonstration, pas des lieux à habiter. La
  question du *« est-ce qu'on a envie d'y rester ? »* (le doute D27, cf. [chantier vivant](chantier-vivant.md))
  reste entière, et elle ne se règlera pas côté réseau.
- **La bascule est prouvée entre deux mondes, sur une machine.** Le passage à plusieurs (des amis qui se
  retrouvent en changeant de monde ensemble) est un cran au-dessus, pas encore franchi.

Autrement dit : le **mécanisme** du passage tient et il est propre. Ce qui manque n'est pas la plomberie —
c'est le **monde au bout**. C'est un chantier assumé, pas un acquis déguisé.

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **le chantier launcher** — la partie du projet qu'un joueur *voit*, là où le réseau invisible
devient un geste concret.

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
