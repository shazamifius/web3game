# 📖 Glossaire — chaque terme, en une phrase

*[English version](en/glossaire.md)*

> Une page de secours, à garder sous le coude. **Il n'y a pas de question bête** : si un mot d'une autre page vous
> arrête, il est probablement ici, expliqué simplement. Les termes sont regroupés par thème, du plus général au plus
> précis.

---

## L'idée générale

- **Pair-à-pair (P2P)** — les ordinateurs des joueurs se parlent **directement** entre eux, sans passer par un
  ordinateur central qui détiendrait la vérité.
- **Serveur central** — dans un jeu classique, l'ordinateur unique au milieu qui voit tout, tranche tout, et que
  ce projet cherche justement à **enlever**.
- **« web3 » (ici)** — pris au sens **décentralisé + identité que tu possèdes** ; **pas** de cryptomonnaie, pas de
  blockchain, pas de token.
- **Latence** — le délai entre l'instant où quelque chose se passe et celui où vous le voyez. Le projet vise
  **≤ 500 ms** pour qu'une présence se sente « vivante ».

## Les connexions (réseau)

- **NAT** — le mécanisme de votre box qui partage une seule adresse publique entre vos appareils ; il **bloque par
  défaut** les connexions entrantes (utile pour la sécurité, gênant pour le P2P).
- **Hole punching** (« percer le NAT ») — la ruse par laquelle deux box ouvrent un passage **au même moment** pour
  laisser passer une connexion directe.
- **NAT « cône » vs « symétrique »** — un NAT **cône** garde le même port public quelle que soit la destination →
  **perçable** ; un NAT **symétrique** en change à chaque fois → **non perçable**, il faut un relais.
- **CGNAT** — un NAT géré par l'opérateur (typique du mobile), où **plusieurs abonnés partagent** une même adresse
  publique. Un CGNAT peut être perçable (cône) ou non (symétrique) — **on le mesure, on ne le suppose pas**.
- **STUN** — une petite question posée à un serveur public (« quelle adresse vois-tu de moi ? ») qui permet de
  **déduire le type de NAT** sans rien installer.
- **Relais** — quand deux joueurs ne peuvent pas se percer, un troisième **recopie** leurs paquets entre eux ; il
  ne peut **pas les modifier** (ils restent signés).
- **Rendez-vous** — le point d'amorçage minimal qui **présente** les joueurs au départ, puis peut s'effacer (le
  seul morceau encore un peu « central », assumé).
- **Gigue** (*jitter*) — l'irrégularité du délai : les paquets n'arrivent pas à un rythme régulier. Forte gigue =
  présence saccadée.
- **Fraîcheur** — l'âge de la dernière information reçue d'un voisin ; on la résume souvent par son **p95** (voir
  plus bas). Sous 500 ms = « vivant ».
- **Perte aléatoire vs congestion** — perdre des paquets **au hasard** (bruit) n'est pas pareil que perdre parce
  que le lien **sature** (congestion) ; le remède diffère, d'où l'intérêt de **mesurer** la nature de la perte.
- **Bufferbloat** — quand un lien saturé gonfle ses files d'attente : la latence explose **avant** que la perte
  n'apparaisse. Un signe de congestion.
- **Redondance / `p^K`** — envoyer une information en **K exemplaires** ; si la perte est *aléatoire* de
  probabilité `p`, la perte résiduelle tombe en `p^K` (utile). Sur un lien **saturé**, dupliquer **aggrave** —
  d'où une redondance **adaptative** (on ne duplique que là où ça aide).
- **netem** — l'outil du noyau Linux qui **injecte une vraie perte/latence** sur une connexion, pour tester un
  protocole dans des conditions réseau contrôlées.

## L'identité et la confiance

- **Signature / Ed25519** — chaque message est scellé avec votre **clé privée** ; n'importe qui vérifie le sceau,
  personne ne peut l'imiter. (Comme une clé SSH.)
- **Clé publique / PeerId** — votre identité **est** votre clé publique, portée dans chaque paquet : elle se prouve
  toute seule, sans annuaire central.
- **Preuve de travail** (*PoW*) — un petit calcul à fournir pour créer une identité, afin de rendre la fabrication
  d'identités en masse **coûteuse** (anti-Sybil).
- **Attaque Sybil** — fabriquer une foule de **fausses identités** pour peser plus lourd qu'on ne devrait.
- **Attaque éclipse** — **isoler** une victime en l'entourant de nœuds complices.
- **Framing** (accusation mensongère) — faire **bannir un innocent** ; la parade : ne compter que des accusateurs
  **crédibles et distincts**, pas des têtes.
- **Quorum / BFT** — exiger qu'un **nombre suffisant** de participants distincts soient d'accord avant d'agir, pour
  tolérer une minorité de tricheurs (*Byzantine Fault Tolerance*).
- **Own + Shields** — pour un objet partagé, **un seul** joueur fait autorité (l'**Own**) ; les autres
  (**Shields**) vérifient et peuvent le destituer s'il triche.

## La foule et la perception

- **Aire d'intérêt (AoI)** — le principe qui **borne** votre coût : vous ne parlez à plein débit qu'à un petit
  voisinage, quelle que soit la taille totale de la foule.
- **Focus vs conscience** — le **focus** = les quelques pairs suivis à plein débit (détaillés) ; la **conscience**
  = tous les autres, perçus en **basse fidélité** (silhouettes). Le focus est borné, la conscience non.
- **Cellule / résumé** — pour une foule lointaine, on remplace N flux individuels par **quelques résumés** de
  région (un échantillon de positions signées).
- **p95** — une façon honnête de résumer une mesure : la valeur sous laquelle tombent **95 %** des cas (on regarde
  donc le « presque pire », pas la moyenne flatteuse).

## Les ponts et les outils

- **Sidecar** — le **pont local** entre le cœur réseau (en Rust) et le moteur 3D (Unreal), via une simple socket :
  c'est ce qui rend le cœur **indépendant du moteur**.
- **Agent de mesure** — un petit **instrument autonome** qu'un volontaire lance chez lui : il rejoint le réseau,
  sonde son lien, et **remonte des chiffres honnêtes** sur la vivacité des liens distants.
- **Headless** — « sans interface graphique » : le cœur réseau tourne en pur calcul/texte, la 3D vit ailleurs.

---

*🗺️ [Revenir à la vitrine](../README.md) · 🧭 [La documentation, par où vous voulez](README.md)*
