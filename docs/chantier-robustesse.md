# Chantier robustesse & longévité — généraliser l'autorité, durer, traverser les NAT difficiles

> Au-delà du cœur réseau : généraliser l'autorité d'un objet unique à des milliers, tenir sur de longues sessions,
> et surtout **faire se rejoindre deux joueurs même quand leurs box refusent la connexion directe**. On y montre,
> sans le cacher, **un bug qu'aucun test en laboratoire ne pouvait voir — et que seul un vrai joueur dehors a
> révélé**.
>
> Voir : [les doutes traités](doutes.md) (surtout D11, D12, D17, D16) · [l'état chiffré](etat-du-projet.md) · [l'idée en clair](comprendre-le-p2p.md).

---

## 1. Généraliser l'autorité des objets partagés

Toute la machinerie d'autorité a d'abord été écrite pour **un seul objet** (une « orbe » de test). Un vrai monde en
a des milliers (portes, scores, projectiles). Le programme :

- **Un registre générique** `{id, type, règle d'autorité, état}` : chaque type d'objet branche sa propre règle,
  en réutilisant la machinerie existante (D12).
- **La reprise d'autorité par quorum** : quand le « maître » d'un objet se tait, la reprise n'est valide que si un
  quorum de voisins confirme — un attaquant seul ne peut pas voler l'objet pendant le silence (D11). *Une approche
  sans maître (résolution par rang déterministe) est par ailleurs préférée là où c'est possible : rien à voler.*
- **Un horodatage signé** pour arbitrer les « courses » (qui a touché en premier ?) là où un jeu compétitif
  l'exige (D13), et un **anti-triche de mouvement plus fin** (vitesse + accélération + cohérence) (D18).

## 2. Durer, et ne pas grossir avec la foule

**Tenir la mémoire (D16).** Les fiches par pair s'accumuleraient sur une longue session : une **éviction par durée
de vie** rend la place des absents (jamais celle d'un actif).

**Une vérification que rien n'explose à l'échelle.** Question légitime : à 5 000 joueurs, est-ce que de trop gros
paquets deviennent ingérables ? **Réponse mesurée, pas argumentée : aucun paquet ne grandit avec le nombre de
joueurs** — toutes les structures sont plafonnées par construction. Le détail :

| Type de paquet | Taille max | Grandit avec N ? |
|---|---|---|
| État joueur (signé) | 182 octets | non (fixe) |
| Présentation initiale (cap 32 voisins) | ~1221 octets | non (plafonné) |
| Carte de découverte (cap 16) | ~740 octets | non (plafonné) |
| Résumé de cellule v1 (cap 16) | ~759 octets | non (plafonné) |
| Résumé de cellule v2 (avec preuves signées) | ~3672 octets | non (plafonné) |

Tout reste sous la taille sûre d'un paquet UDP (~1200 octets), **sauf le résumé v2** (le format à preuves
signées) : il la dépasse → il faudra le **découper en morceaux indépendants** avant de l'utiliser à grande échelle.
C'est une **dette identifiée d'avance** (pas une surprise du jour de l'échelle), et le format le permet déjà
(chaque preuve est auto-certifiante, donc voyage seule).

## 3. Le relais pour les NAT difficiles (D17) — et un bug révélé par le réel

**Le problème.** Le hole-punching échoue sur les NAT les plus fermés (dits « symétriques »). Ces joueurs-là ne
peuvent pas se connecter en direct → il leur faut un **relais** (un tiers qui recopie les paquets). *Cadrage honnête :
c'est un **repli** pour la minorité non-perçable, pas le chemin normal — et notre [sonde de lien](chantier-reseau.md)
a depuis montré que ce cas est plus rare qu'on ne le croyait (un mobile grand public qu'on pensait symétrique s'est
révélé perçable).*

**La sécurité est gratuite, par construction.** Un état de jeu reste **signé de bout en bout** : le relais ne peut
que **porter** les octets, **jamais les falsifier**. Il n'y a donc rien de neuf à prouver côté confiance — juste à
border l'amplification (un relais en entrée = un seul en sortie, débit plafonné, destinataire forcément inscrit).

**Prouvé en réel.** Deux humains, deux vrais réseaux qui **ne se perçaient pas** en direct, **se voient bouger via le
relais**. Le juge est neutre : le journal du serveur montre le trafic relayé **dans les deux sens**. *C'est la première fois que le code fait entrer deux humains réels dans le
même espace via Internet — un premier élément de réponse à la « forteresse vide » (D27).*

**Le bug que seul le réel pouvait montrer.** Au premier essai, l'expérience était **asymétrique** : A voyait B,
mais B ne voyait pas A. Cause : en recevant un état *relayé*, le code ouvrait quand même un « trou » de connexion
directe → A se croyait connecté en direct, n'abandonnait jamais le perçage, et ne relayait donc jamais en retour.
**Aucun test en laboratoire ne pouvait le voir** : sur une seule machine, le perçage réussit toujours, donc le cas
« relais » n'arrive jamais. Correctif : n'ouvrir le trou direct que si le paquet ne vient pas du relais. *La valeur
d'un humain dehors, démontrée noir sur blanc.*

**Le relais porte aussi les objets du monde.** Au-delà de l'avatar, l'**enveloppe de relais a été généralisée**
pour transporter n'importe quel paquet déjà signé (un objet partagé, par exemple) — sans changer d'un octet le
transport de l'avatar déjà prouvé. Vérifié en réel : à travers le NAT, deux joueurs partagent **un seul objet, avec
un maître cohérent** des deux côtés.

**Mesurer le lien plutôt que le supposer (28 juin).** Pour savoir si ces liens distants sont *vraiment* vivants, on a
écrit un **instrument de mesure** : un agent autonome que des volontaires lancent, qui relève la fraîcheur, la perte,
la gigue et le ré-ordonnancement des liens qu'il perçoit, et renvoie les chiffres. Deux leçons en sont sorties :

- **Une fausse alerte instructive.** Les premiers relevés criaient « 89 % de perte » sur le chemin relais. Enquête :
  ce n'était pas le réseau, mais **notre propre économie de bande passante mal mesurée** (l'instrument comparait le
  filet basse-fréquence reçu au plein débit émis). L'instrument sait désormais **séparer « pas envoyé exprès » de
  « envoyé puis perdu »**. Récit complet : [les coulisses](coulisses.md).
- **Le verdict, une fois l'instrument honnête.** Sur de vrais liens distants (plusieurs pays, certains en CGNAT), la
  présence est **vivante** : fraîcheur **p95 ~200–335 ms** (< 500 ms), **perte réelle ~0**. Deux durcissements tirés
  du réel sont restés : une **tolérance au silence** côté rendez-vous portée de **5 s à 20 s** (un lien CGNAT se
  ré-enregistre sans cesse — à 5 s on l'évinçait à tort), et une **redondance d'émission** qui a, depuis, mûri en
  **redondance ADAPTATIVE** : un nœud ne dédouble que s'il a **mesuré** une perte *aléatoire* avec de la marge (gain
  prouvé sur banc : `30 % → 9 %` à deux copies), et **jamais** sur un lien qui *sature* (où dupliquer aggrave). Le
  détail de cette enquête : [le chantier réseau](chantier-reseau.md) et le doute **D36**.

## 4. Au-delà (différé, assumé)

- **La voix spatiale de proximité** : un chat vocal pair-à-pair, spatialisé, qui profite du chiffrement et de
  l'inclusivité (la voix s'adapte au lien). Gros morceau, volontairement repoussé.
- **La portabilité entre moteurs** : extraire un cœur réseau portable pour différents moteurs 3D. Reporté — le pont
  local actuel suffit à le prouver (deux moteurs partagent déjà le même espace).

## Ce que ce chantier ne prouve pas (honnêteté)

- Le relais est, à ce stade, **centralisé** (le point de rendez-vous relaie) : c'est un repli assumé, à
  **décentraliser** ensuite (ce qui rouvre la question de l'incitation à relayer, D4).
- Il est prouvé **à deux joueurs**, pas à l'échelle.
- La mesure dehors porte sur le **substrat** (la présence transportée, vivante) — **pas encore sur le ressenti**
  d'humains qui bougent et jouent ensemble (D27 « la forteresse vide » s'allège, sans se fermer).
- La généralisation de l'autorité (registre, quorum, horloge) est en grande partie **planifiée**, pas encore bâtie.

---

*Faits techniques tenus à jour au fil des mesures. Les chiffres de coût et de débit : [revue de l'état](etat-du-projet.md).*

---

### 🧭 Se repérer — où que vous commenciez, vous êtes au bon endroit

Vous lisez **Chantier robustesse** — une étape des parcours **⚙️ Le code** et **🧭 Tout comprendre**.

**Continuer le fil :**
- ⚙️ *Le code* · 🧭 *Tout comprendre* → **[Comment lancer & tester](TESTS.md)**

**Les portes** (sautez, revenez, changez à tout moment) :
🌱 [Découvrir](comprendre-le-p2p.md) · ⚙️ [Le code](ARCHITECTURE.md) · 🔎 [Juger vite](etat-du-projet.md) · 🧭 [Tout comprendre](README.md) · 📖 [Glossaire](glossaire.md) · 🗺️ [La vitrine](../README.md)
