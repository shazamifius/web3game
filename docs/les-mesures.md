# Les mesures, en équations — comment on chiffre « vivant » et la qualité d'un lien

> Le principe du projet : *une preuve = un chiffre reproductible.* On ne dit pas « ça a l'air fluide » ou « le lien
> est mauvais » — on **mesure**, avec des formules simples, déterministes, rejouables. Cette page rassemble **tous les
> calculs** derrière les verdicts : la fidélité d'un mouvement perçu, la qualité d'un lien réseau, la nature d'une
> perte, et la redondance. Chaque formule est suivie de ce qu'elle capte **et de ce qu'elle ne prouve pas**.
>
> Voir aussi : [le chantier « vivant »](chantier-vivant.md) (le récit) · [le chantier réseau](chantier-reseau.md) ·
> [le glossaire](glossaire.md) · [comment rejouer les mesures](TESTS.md).

---

## 1. Le mouvement perçu est-il « vivant » ? — trois mesures séparées

On joue une trajectoire **vraie** $p^*(t)$ (analytique, donc connue exactement), on la fait passer par un canal réseau
réaliste, et on reconstruit la position **perçue** $\hat p(t)$ côté récepteur. On compare les deux courbes par **trois**
grandeurs distinctes (on refuse une note unique : « vivant » a trois dimensions qui se règlent séparément).

### 1.1 Fidélité de forme $F$ (et fraîcheur $d_{\text{eff}}$)

L'erreur de tracé **une fois le retard compensé** — « est-ce le bon geste, juste en retard ? ». On cherche le décalage
$d$ qui aligne au mieux les deux courbes, et on garde l'erreur résiduelle :

$$F = \min_{0 \le d \le d_{\max}} \sqrt{\frac{1}{N}\sum_{t} \lVert \hat p(t) - p^*(t-d) \rVert^2}$$

- Le balayage de $d$ se fait sur une grille de **2 ms**. Le $d$ qui réalise ce minimum est la **fraîcheur**
  $d_{\text{eff}}$ : le retard réellement perçu. $F \approx 0$ signifie « ce que je vois EST ce qui a été joué, juste
  en retard de $d_{\text{eff}}$ ».
- $F$ est en mètres (on l'affiche en cm) ; $d_{\text{eff}}$ en secondes (cible $\le 500$ ms, ambition $\approx 150$ ms).

### 1.2 Fluidité $J$ (le « jerk ») et les sauts

Le **jerk** = la norme de la 3ᵉ dérivée discrète de la position perçue (un trou mal comblé pique) :

$$J = \sqrt{\frac{1}{M}\sum_i \left\lVert \frac{\hat p_i - 3\hat p_{i-1} + 3\hat p_{i-2} - \hat p_{i-3}}{\Delta t^3} \right\rVert^2}$$

où $\Delta t = 1/f_{rx}$ est le pas d'affichage. On compte aussi les **sauts** (téléports visibles) :
$n_{\text{sauts}} = \#\{\, i : \lVert \hat p_i - \hat p_{i-1}\rVert > \text{seuil}\,\}$ (seuil 0,5 m entre deux images).
La référence est le **jerk naturel** de la trajectoire elle-même : une reconstruction parfaite l'atteint, jamais moins.

### 1.3 Le verdict

$$\textbf{vivant} \iff F \le \varepsilon \ \text{ ET }\ d_{\text{eff}} \le 500\,\text{ms} \ \text{ ET }\ J \le \tau \ \text{ ET }\ n_{\text{sauts}} = 0$$

Seuils calibrés sur les premiers runs : $\varepsilon = 2$ cm (fidélité) et $\tau = 2 \times J_{\text{naturel}}$
(jerk). Sinon le verdict précise la cause : *saccadé* ($J$ ou sauts), *flou* ($F$), *en retard* ($d_{\text{eff}}$).

### 1.4 Comment on reconstruit $\hat p(t)$ (interpolation / prédiction)

Le récepteur affiche l'instant $t - d_{\text{interp}}$. Entre deux états reçus $(p, v)$ encadrants $a$ et $b$, on
interpole par une **spline de Hermite cubique** (qui respecte les vitesses aux extrémités → reproduit *exactement* une
ligne droite). Avec $s = (t - t_a)/\Delta t$ et $\Delta t = t_b - t_a$ :

$$\hat p = h_{00}\,p_a + h_{10}\,\Delta t\, v_a + h_{01}\,p_b + h_{11}\,\Delta t\, v_b$$
$$h_{00}=2s^3-3s^2+1,\quad h_{10}=s^3-2s^2+s,\quad h_{01}=-2s^3+3s^2,\quad h_{11}=s^3-s^2$$

Quand l'état « après » n'est pas encore arrivé, on **extrapole** — c'est là que l'**ordre de prédiction** compte. Avec
$\delta = t - t_a$ :

| ordre | ce qu'on connaît | extrapolation |
|---|---|---|
| 0 | position | $\hat p = p_a$ (on tient la position) |
| 1 | position + vitesse | $\hat p = p_a + v_a\,\delta$ (tangente) |
| 2 | + accélération | $\hat p = p_a + v_a\,\delta + \tfrac{1}{2}\,a\,\min(\delta, H)^2$ |

L'accélération $a = (v_a - v_{\text{prev}})/(t_a - t_{\text{prev}})$ est **estimée localement** par différence finie des
deux dernières vitesses reçues — *donc rien à ajouter au format réseau.* L'horizon $H = 0{,}15$ s plafonne le terme
quadratique pour qu'un trou long ne le fasse pas exploser. *(Résultat mesuré : l'ordre 2 divise $F$ et $J$ par ~6× à bas
délai — cf. [le chantier vivant](chantier-vivant.md).)*

Enfin, une **réconciliation amortie** (ressort « critically damped », façon *SmoothDamp*) peut lisser le saut d'une
correction tardive : la position affichée poursuit la cible sans dépassement, en $\approx \text{smooth\_time}$ secondes.

---

## 2. Quelle est la qualité d'un lien ? — ce que « l'œil dirait », chiffré

Chaque état réseau porte un numéro de séquence $\text{seq}$ **monotone** (l'anti-rejeu). Du point de vue d'un
observateur, la suite des couples $(\text{recv\_ms}, \text{seq})$ reçus suffit à tout déduire, sans 3D et sans humain.

Sur la plage observée, $\text{expected} = \text{seq}_{\max} - \text{seq}_{\min} + 1$.

### 2.1 Perte APPARENTE vs perte RÉELLE (la distinction qui évite de se mentir)

$$\text{loss\_pct (apparente)} = \max\!\left(0,\ 1 - \frac{\text{reçus}}{\text{expected}}\right)$$

⚠️ **Cette perte apparente est trompeuse** : un pair *lointain* n'est rafraîchi qu'à basse cadence (par le mécanisme de
champ de vision), pendant que l'émetteur incrémente son seq à plein débit pour tout le monde. Ce pair voit donc
seq 1, 11, 21… → la perte apparente le dit « 90 % perdu » alors que **rien** ne l'est : l'émetteur n'a simplement pas
*envoyé* ces seq-là. On corrige en **inférant la cadence** :

- $\text{base} = \text{médiane des sauts de seq consécutifs}$ (robuste : une vraie perte fait un saut $\approx$ double) ;
- pour chaque saut $g$ : $k = \max(1, \text{round}(g/\text{base}))$ créneaux d'émission attendus ; $\text{slots} \mathrel{+}= k$,
  $\text{missing} \mathrel{+}= k-1$ ;

$$\text{real\_loss\_pct} = \frac{\text{missing}}{\text{slots}}, \qquad \text{cadence\_step} = \text{base}$$

Saut $\approx 1$ pas = normal ; $\approx 2$ pas = un envoi *vraiment* perdu. **C'est `real_loss_pct` (pas la perte
apparente) qui dit la vérité d'un lien.**

### 2.2 Ré-ordonnancement, gigue, fraîcheur

$$\text{reorder\_pct} = \frac{\#\{\text{seq qui reculent}\}}{\text{reçus} - 1}$$

$$\text{jitter\_ms} = \frac{1}{n}\sum_i \big| g_i - \bar g \big| \quad (g_i = \text{intervalles inter-arrivées})$$

**Fraîcheur** (la grandeur reine du « vivant ») : on balaie le temps par pas de `tick_ms` et on note l'âge du dernier
état reçu — une dent de scie, 0 juste après une arrivée, qui monte jusqu'à la suivante. On en sort les centiles
$\text{fresh\_p50}$, $\text{fresh\_p95}$ et le pire cas $\text{fresh\_max}$ (centile = rang le plus proche sur la série triée).

### 2.3 Le verdict de lien

$$
\textbf{vivant} \iff \text{fresh\_p95} \le 500\,\text{ms}
$$

Sinon : *MORT(silencieux)* si zéro reçu ; *lointain (basse-fidélité)* si $\text{cadence\_step} \ge 4$ **ou**
$\text{real\_loss\_pct} \le 20\%$ (un lien lent mais propre n'est pas mort, juste peu rafraîchi) ; *MORT(>500 ms)* sinon.
Le seuil de 20 % vient d'une observation : les liens sains ($\approx 0\%$) et les liens CGNAT lossy (50–80 %) forment
**deux populations nettes** → tout seuil dans $[10, 40]\%$ les sépare ; on prend 20 % (marge confortable).

---

## 3. Quelle est la NATURE d'un lien ? — la sonde (sans dépendance externe)

### 3.1 Type de NAT (perçable ou non) par STUN

On interroge **deux** serveurs STUN publics depuis une **seule** socket et on compare l'adresse publique vue :

$$
\text{NAT} =
\begin{cases}
\textbf{cône (perçable)} & \text{si même IP:port vue des deux serveurs}\\
\textbf{symétrique (CGNAT)} & \text{si le port public diffère}\\
\text{indéterminé} & \text{si moins de deux observations}
\end{cases}
$$

Un NAT symétrique refait un mapping par destinataire → le perçage direct échoue → **relais obligatoire**. Le RTT médian
et la gigue sont mesurés « gratuitement » sur les mêmes aller-retours STUN (aucun serveur en plus).

### 3.2 Nature de la perte : congestion vs aléatoire

Une courte rafale à débit croissant produit une courbe de paliers $(\text{Mbps}, \text{perte}, \text{RTT})$. Par palier :

$$\text{perte} = 100\cdot\frac{\text{envoyés} - \text{reçus}}{\text{envoyés}}, \qquad \text{Mbps} = \frac{\text{pps}_{\text{eff}} \cdot \text{taille} \cdot 8}{10^6}$$

On compare la **base** (palier le plus bas) au **pic** sur tous les paliers (le bufferbloat culmine souvent *avant* le
débit max) :

- **congestion** si la perte grimpe ($\Delta_{\text{perte}} \ge 5\%$) **ou** le RTT grimpe ($\text{RTT}_{\max} \ge 1{,}5\times\text{RTT}_{\text{base}}$ et $+30$ ms au moins) ;
- **aléatoire** si la perte est haute mais *plate* ($\text{base} > 5\%$ et $\Delta < 5\%$) ;
- **sain** sinon.

Cette distinction décide la suite : on ne traite pas une congestion comme un bruit aléatoire.

---

## 4. La redondance, et pourquoi elle est ADAPTATIVE

Sur des pertes **indépendantes** de probabilité $p$, envoyer $K$ copies (via le relais) ne perd un paquet que si ses
$K$ copies sont toutes perdues :

$$\text{perte résiduelle} = p^K \qquad\Longrightarrow\qquad K = \left\lceil \frac{\ln(\text{cible})}{\ln(p)} \right\rceil \ \text{(borné)}$$

Mais ce gain $p^K$ ne vaut que pour de la perte **aléatoire**. Sur un lien **congestionné**, dupliquer *aggrave* la
saturation (leçon prouvée en réel). D'où la décision **adaptative**, branchée sur la sonde §3.2 :

| nature du lien | redondance $K$ |
|---|---|
| sain | 1 (inutile) |
| aléatoire (avec marge) | $K = \lceil \ln(\text{cible})/\ln(p)\rceil$ → gain $p^K$ |
| congestion | **1** (surtout pas dupliquer) |
| indéterminé | 1 (prudent) |

*(Le gain $p^K$ sur perte aléatoire est prouvé en labo sur un vrai lien `netem` ; cf. [chantier réseau](chantier-reseau.md).)*

Note d'orientation : le type de connexion grossier est aussi **déduit de l'IP publique** vue côté serveur
(100.64.0.0/10 = CGNAT opérateur ; 10/8, 172.16/12, 192.168/16 = réseau local ; sinon publique) — une heuristique, que
la sonde STUN affine ensuite.

---

## 5. Ce que ces chiffres NE prouvent PAS (honnêteté)

- **Un banc peut INVALIDER une hypothèse, jamais à lui seul VALIDER le réel.** Le banc « vivant » est un *instrument*
  déterministe : il trace des compromis et réfute des idées fausses, mais le vrai juge du « est-ce vivant ? » reste un
  humain qui bouge avec un autre via le vrai Internet (le doute D27 reste ouvert).
- Les profils de lien du banc sont *inspirés* des mesures réelles, **pas encore branchés en direct** sur la sonde.
- `real_loss_pct` infère la cadence d'une médiane : sur très peu d'arrivées, il s'abstient plutôt que d'inventer.
- Le verdict de fraîcheur est un seuil pratique (500 ms), pas une frontière physique ; il sépare bien deux populations
  observées, c'est tout ce qu'on lui demande.

---

*🗺️ [Revenir à la vitrine](../README.md) · 📚 [Sommaire de la doc](README.md) · 📖 [Glossaire](glossaire.md)*
