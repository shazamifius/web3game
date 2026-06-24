# ✨ PRÉSENCE VIVANTE — le rejeu incarné (ch. 13, D27, l'âme du projet)

> Pourquoi un avatar doit SEMBLER vivant ; l'option A (interpolation incarnée), déjà codée et validée.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

## Chapitre 13 — PRÉSENCE VIVANTE : le rejeu incarné (D27, l'âme du projet)

> **Posé le 24 juin 2026, après que l'utilisateur l'a OBSERVÉ de ses yeux.** Le relais-retour réparé,
> deux humains se connectent enfin via un vrai mobile — mais **le déplacement ne se SENT pas humain à
> travers l'écran.** C'est D27 « la forteresse vide » rendue tangible : on a la plomberie, pas la VIE.
> Ce chapitre conçoit l'outil que l'utilisateur juge **obligatoire**, AVANT de coder (méthode : le mur
> sur le papier). *C'est un chantier de RESSENTI → les options finales, c'est LUI qui les tranche.*

### 13.0 — L'idée (mot pour mot, à ne pas trahir)
**PAS** lisser les inputs. **PAS** réduire la latence (surtout pas). Mais : **capturer le déplacement de
l'individu à plusieurs points dans le temps, puis le REJOUER pour tout le monde**, pour obtenir le vrai
« ah, on voit que c'est un humain qui bouge ». Ambition : **rendre plus vivant que si on avait 0 ms.**

### 13.1 — Pourquoi le « 0 ms exact » est MORT (le diagnostic)
1. **Interpolation linéaire** entre échantillons (20 Hz) → segments à vitesse constante = mécanique.
2. **Aucune motricité biologique** : un vrai corps a du poids, de l'anticipation, du report d'appui, des
   micro-corrections, une respiration, un balancement, des pas (IK), un regard qui vit.
3. **Le gel sur perte/jitter** : un paquet en retard → la capsule fige ou « claque » → l'illusion meurt à
   l'instant. **C'est LÀ que le 0 ms échoue** : à 0 ms avec une statue figée sur un paquet perdu, c'est mort.

### 13.2 — L'intuition clé (sa philosophie « le vivant se FABRIQUE côté client »)
La **précision de position** n'est PAS ce qui rend une présence vivante — la **QUALITÉ DE MOUVEMENT
BIOLOGIQUE** l'est. Donc on TROQUE un petit retard de rejeu (un tampon, ASSUMÉ) contre un rendu bien plus
riche et augmenté de vie. Parce qu'on rejoue depuis un tampon, on peut : lisser en SPLINE (pas linéaire) ;
ajouter une vie procédurale **verrouillée en phase sur le mouvement réel** (report d'appui, cadence de pas
selon la vitesse, balancement à l'arrêt, micro-mouvements de tête, respiration) ; et **ne JAMAIS figer**
(sur perte, la vie procédurale continue). *C'est ainsi qu'on bat le 0 ms : le 0 ms fige, nous on respire.*

### 13.3 — LE MUR à voir maintenant (les garde-fous, sinon on fait pire que mieux)
1. **Budget vérité-vs-vie.** Le rendu embelli doit rester à distance BORNÉE de la position autoritaire,
   sinon désync (je pousse une balle en X, les autres me voient en X+dérive). Budget = retard de tampon
   (~100-200 ms) + déviation spatiale max. **La vie procédurale est de FAIBLE amplitude** (balancement de
   cm, pas de mètres) : elle ment sur le COMMENT du mouvement, jamais sur le OÙ du corps.
2. **Vallée de l'étrange.** Un mouvement ajouté FAUX (démarche désynchro de la vitesse, balancement en
   sprint) est PIRE qu'un robot. → la vie procédurale est **pilotée par le signal de mouvement réel**
   (vitesse → cadence ; virage → inclinaison ; arrêt → respiration). Verrouillée en phase, jamais libre.
3. **Autorité / anti-triche : RENDU SEULEMENT.** La couche vivante est EN AVAL de toute validation et ne
   reflue JAMAIS dans l'état autoritaire (sinon on rouvre les triches de position/téléport). Un pair
   malveillant ne peut rien en tirer : ça n'affecte que le rendu LOCAL de poses déjà validées. (Cohérent
   Règle 1 : le cœur Rust ne bouge pas — toute cette couche vit côté client UE.)
4. **Dégradation sur perte = LE point.** Sur paquet en retard/perdu : extrapoler brièvement (vitesse
   décroissante) + garder la vie d'attente ; au retour d'une vraie pose, **revenir en douceur (ressort)
   sur la spline — jamais de claquement.** Le retard de tampon donne exactement cette marge.
5. **Coût (55K = boussole).** Surtout du GPU/anim (côté UE). Réseau : au plus une pose un peu enrichie —
   doit rester BORNÉ par avatar (sinon on casse l'invariant « charge/nœud indépendante de N »).

### 13.4 — Le mécanisme (par avatar distant, côté UE)
1. **Tampon d'ingestion** : anneau des K dernières poses {pos, vitesse, yaw, pitch, t} (~0,5 s d'histoire).
   *(Le sidecar fournit déjà pos+vitesse à 20 Hz ; UE accumule l'histoire à mesure qu'elle arrive.)*
2. **Tête de rejeu** : on rend à `maintenant − RETARD` (~120 ms), en lisant le tampon en **spline
   Catmull-Rom** (courbe naturelle, pas de lerp linéaire).
3. **Analyse de mouvement** : du tampon, dériver vitesse, changement de cap, état arrêt/marche.
4. **Couche de vie procédurale (pilotée par 3)** : locomotion (cadence marche/course ∝ vitesse, IK des
   pieds au sol) ; inclinaison (report d'appui dans les virages/accélérations) ; attente (respiration +
   léger report + regard occasionnel, **graine par identité** → chacun est constamment « lui ») ; tête
   (micro-bruit yaw/pitch autour de la vraie direction de regard).
5. **Gestion de perte** : si la tête de rejeu dépasse le tampon, MAINTENIR (extrapolation décroissante) +
   garder la vie d'attente ; au retour d'une pose fraîche, revenir en ressort sur la spline (pas de snap).

### 13.5 — Les OPTIONS (le carrefour — c'est TOI qui tranches, c'est du ressenti)
- **Option A — « Interpolation incarnée » (mini, risque faible) :** spline + respiration/balancement
  d'attente + survie à la perte. **Zéro nouvelle donnée réseau, 100 % UE.** Le plus rapide ; probablement
  ~80 % du gain ressenti. *Se teste DÈS les capsules actuelles (pas besoin de vrais avatars).*
- **Option B — « Corps animé » (moyen) :** A + blendspace de locomotion complet (marche/course/virage) +
  IK des pieds + inclinaison. Demande un avatar humanoïde riggé → se MARIE avec l'étape « vrais avatars ».
  Gros gain, plus de travail moteur (ton domaine).
- **Option C — « Intention partagée » (riche, change le contrat) :** B + le cœur envoie un petit indice
  d'INTENTION (cible du regard, drapeau de geste) → les distants ANTICIPENT (mènent le mouvement) → rendu
  EN AVANCE sur le tampon, qui **bat vraiment le 0 ms en ressenti**. Coût réseau à MESURER avant (borne MTU
  + invariant 55K). La plus ambitieuse.
- **Ma reco :** commencer **A** (prouve la thèse à pas cher, sur les capsules d'aujourd'hui) → puis **B**
  quand on pose les vrais avatars → **C** seulement si A/B ne suffisent pas déjà à « sentir l'humain ».

### 13.6 — Comment on saura que c'est VIVANT (l'honnêteté de mesure)
La (b) « fraîcheur ressentie » n'était jamais mesurable en headless — et « vivant » est INTRINSÈQUEMENT un
jugement humain. **Le juge = TOI + quelques testeurs** : un A/B aveugle (« lequel ressemble le plus à une
personne ? ») entre lerp-brut et rejeu-incarné, idéalement à quelques humains. On ne teste PAS « vivant »
en unitaire. MAIS on peut prouver en déterministe les **sous-propriétés garde-fous** : ne-jamais-figer-sur-
perte (test : on drope des paquets, on assert que la tête de rejeu continue de bouger) ; déviation bornée
(assert |rendu − autoritaire| < budget). Ces tests gardent le mécanisme ; le ressenti, lui, reste ton arbitrage.

### 13.7 — Où ça se branche dans le plan
Chemin B (polir pour des centaines) tient, MAIS la présence vivante est ce qui rend les centaines DÉSIRABLES
(des centaines de capsules mortes < une poignée d'humains vivants). Reco d'ordre : **vrais avatars + repère
partagé (chemin B étape 3) ET option A en parallèle** (A ne dépend pas des avatars) → puis B sur les vrais
avatars. La foule-LOD (étape 2) et la présence vivante se complètent : LOD = combien on en montre ; présence
= à quel point chacun semble humain. *Petit pas, preuve d'abord — mais ici la « preuve » finale est un humain
qui dit « oui, là je sens quelqu'un ».*

---

