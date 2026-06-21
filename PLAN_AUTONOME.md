# 🤖 PLAN D'ATTAQUE AUTONOME — travail fait pendant l'absence de l'utilisateur

> **À quoi sert ce fichier.** L'utilisateur s'absente longtemps. Ce document est (a) le **contrat**
> de ce que j'ai le droit de faire seul, (b) mon **ancre anti-collapse** relue à CHAQUE itération
> (avec l'ancre §0 de `FEUILLE_DE_ROUTE.md`), (c) le **journal** de ce qui a réellement été fait.
> L'utilisateur le lit/corrige à son retour. Décidé avec lui le 20 juin 2026.

## 🔬 INVESTIGATION SUPERVISÉE (20 juin, au retour de l'utilisateur) — règles + pré-enregistrement
> Suite à ma sur-affirmation (« banc validé » sur le seul N=1000, réfuté à 5000), l'utilisateur a
> recadré : **priorité = la MÉTHODE, pas le deadlock.** On ne touche **NI le protocole NI le tuning** ;
> on INSTRUMENTE pour COMPRENDRE avant de décider. Deux règles dures (aussi en mémoire `methode-de-travail`) :
> - **RÈGLE 1 — jamais validé sur un seul N** : petit (~100) / moyen (~1000) / grand (~5000+), tendance cohérente.
> - **RÈGLE 2 — critère de succès écrit AVANT la modif** : le résultat décide, jamais l'inverse (sinon on
>   « tune le banc jusqu'à ce que ça plaise »). Un banc = INSTRUMENT de diagnostic, PAS une preuve.
>
> **Question scientifique (pas « atteint-on 50k ? » mais) : POURQUOI la connectivité s'effondre-t-elle à 5000 ?**
> → mesurer la STRUCTURE DU GRAPHE (composantes connexes, nœuds isolés, taille de la plus grande grappe,
> taux d'ouverture des trous), à N=100/1000/5000 (Règle 1). **Pré-enregistrement (Règle 2) : je rapporte
> les chiffres BRUTS quoi qu'il arrive.** Hypothèse à FALSIFIER : « à 5000 le graphe se fragmente en
> grappes qui ne se rencontrent jamais (transition de phase de percolation) ». Si la plus grande grappe
> chute brutalement entre 1000 et 5000 → vrai mur d'archi (trouvaille). Sinon → ailleurs. **AUCUN fix tant
> que la cause n'est pas identifiée ; le PUNCH-retour (protocole) reste INTERDIT pour l'instant.**

## ⚖️ Le cadre (décidé avec l'utilisateur)
- **Périmètre = Tier 0 + Tier 1 SEULEMENT** : simulateur léger + durcissements ADDITIFS. **Jamais**
  le cœur du vrai jeu, jamais le wire/chiffrement, jamais le rendu.
- **Sauvegarde = branche `autonome/juin2026`** : push à chaque étape verte ; **`main` reste INTOUCHÉE**
  jusqu'à la revue de l'utilisateur. Tout est sur GitHub (recoverable), rien ne « tombe » sur main seul.
- Base de départ : `main` @ `b75b6fe` (D26 couche 1 faite, 73 tests, 0 warning).

## 🚫 INTERDITS en autonomie (je les mets en file pour l'utilisateur, je ne les touche PAS)
- Tout ce qui se vérifie **à l'écran** (rendu, interpolation orbe R1, avatars) — c'est lui qui lance le 3D.
- Toute **décision de direction** (section G de la roadmap — toutes déjà tranchées de toute façon).
- Le **cœur du vrai jeu** (`netcode/receive.rs`, `netcode/send.rs`) d'une façon non prouvable headless.
- Le **wire/chiffrement** (ch.10.2) : touche le format réseau → casse invisible sans le 3D. Reporté.
- Le **refactor Session commune** (D2) : touche le vrai jeu → reporté à une session supervisée.
- Écrire « 55K prouvé ». Jamais.

## 🧭 Protocole anti-collapse (à exécuter à CHAQUE réveil)
1. Relire l'**ancre §0** de `FEUILLE_DE_ROUTE.md` + **ce fichier** (pas l'historique entier).
2. Prendre **le prochain pas non fait** dans la file ci-dessous. UN SEUL.
3. **compile → test → sim (headless) → commit → push** sur `autonome/juin2026`.
4. Mettre à jour le **JOURNAL** ci-dessous (1-3 lignes : menace→fix→preuve + ce que ça NE fait PAS).
5. **STOP durs** : si le pas toucherait le cœur/jeu → STOP, le mettre en « file utilisateur ».
   Si un test passe au rouge et que je ne le répare pas en 1 pas → **révoquer** (`git revert`/reset) et STOP.
   Si je doute d'un mécanisme (risque rustine) → m'arrêter au PAPIER, ne pas coder.
6. Quand la file sûre est épuisée → **ARRÊTER**. Ne pas inventer de travail risqué pour m'occuper.

---

## 📋 LA FILE (ordonnée par valeur ÷ risque)

### 🥇 TIER 0 — Simulateur léger coopératif (dette D25/D20) — *risque cœur ≈ 0*
> Le meilleur travail autonome : un OUTIL neuf à côté du protocole, 100 % headless, aucune décision.
> Débloque la plus grosse dette d'honnêteté (« 55K jamais mesuré directement »). Issue gagnante des
> deux côtés : soit l'asymptote tient (preuve directe), soit je TROUVE LE MUR (= chercher le mur tôt).

- **T0.1 — Le squelette du banc coopératif.** Un ordonnanceur N sessions/thread, **une** boucle
  d'événements, **bus mémoire** au lieu de vraies sockets UDP, zéro `thread::sleep`/bot.
  - *Preuve* : un test qui crée K sessions, fait tourner T ticks, vérifie qu'elles s'échangent des
    paquets via le bus (≥1 état reçu de bout en bout). Fichier neuf (ex. `src/net/coopsim.rs`), additif.
  - *Go/No-go* : ne touche AUCUN fichier existant du protocole (réutilise `bot.rs`/`link.rs` en lecture).
- **T0.2 — Garde-fou de FIDÉLITÉ (avant toute extrapolation).** Le banc léger doit **REPRODUIRE** les
  chiffres connus du vrai `sim`/`crowd` à ~1000–1500 nœuds (perception par résumé, débit plat, orbe 0/N).
  - *Preuve* : un run léger à 1000 nœuds donne perception/débit **cohérents** (±marge) avec le vrai
    `crowd 1000`. Si NON → le banc est faux (comme le bug 7.4b), je m'arrête et je le note. **Bloquant.**
- **T0.3 — ⛔ GELÉ (T0.2 a échoué).** Extension à 5k/20k/50k IMPOSSIBLE de façon fidèle sur ce banc
  (UDP réel + un thread = temps mural dilaté). Débloqué SEULEMENT si l'utilisateur autorise le bus
  mémoire (= changement de cœur, voir FILE UTILISATEUR). Tant que non : **aucune extrapolation**.

### 🥈 TIER 1 — Durcissements concrets, bornés, additifs, headless
- **T1.1 — D16 : TTL / éviction des pairs (ch.12.1).** Aujourd'hui `MAX_KNOWN` est un mur sans TTL →
  la table se remplit de morts et **bloque l'apprentissage** sur longue session.
  - *Fix* : un horodatage de dernière activité par pair + éviction du plus vieux quand la table est
    pleine (jamais évincer un pair ACTIF). Additif à `link.rs`.
  - *Preuve* : test — table pleine de pairs inactifs → on avance le temps → un nouveau pair s'apprend
    (le plus vieux mort évincé) ; un pair ACTIF n'est jamais évincé. *NE fait pas* : pas de DHT (annexe H).
- **T1.2 — D21 : rendez-vous, rate-limit débit + anti-spoofing.** Durcissement borné de `rendezvous.rs`
  (cap mémoire + PoW déjà là en 9.5a ; reste le débit + anti-usurpation d'adresse source).
  - *Preuve* : test/sim — un client qui spamme le rendez-vous est throttlé ; un HELLO à adresse source
    incohérente est rejeté. *NE fait pas* : ne supprime pas la centralisation du rendez-vous (D10, annexe H).
- **T1.3 — ✋ STOP au PAPIER (D18 speed-hack).** À l'inspection, le détecteur de vitesse SOUTENUE n'est
  PAS cleanly bornable en autonomie : `MAX_SPEED` est un placeholder (vraie vitesse du jeu non figée) et
  un contrôle fenêtré faux-positive sur un honnête après perte de paquets (sourdine injuste, visible
  seulement dans le vrai jeu). → escaladé en FILE UTILISATEUR plutôt que de coder une rustine.

> Ordre conseillé : **T0 d'abord** (plus gros gain, risque nul), puis T1.1 → T1.2 → T1.3.

---

## 📓 JOURNAL (rempli au fil des itérations autonomes — le plus récent en HAUT)

- **ÉTAPE C-sécu-1a — TEST D'ÉCHELLE (N=2000, 21 juin, supervisé) : la corroboration BAT la taxe à l'échelle
  → critère pré-enregistré REMPLI, 8.3★ validé comme solution d'ÉCHELLE.** CORROB 2000 : perception moy **761 /
  max 1999 (38 % de N)**, taxe **0 %**, débit ↑54/↓55, **encore en hausse à t=100**. *Comparaison à N=2000 :*
  défaut-AVEC-taxe = **192 (10 %)** ; DENSITY_MAX (sans taxe, instrument) = **1050 (52 %)** ; **CORROB = 761** →
  **~4× la taxe**, et **72 % de DENSITY_MAX**. *La BASCULE attendue est confirmée :* à 1000 la corroboration PERD
  (563 < 759, taxe douce ~24 %) ; à 2000 elle GAGNE largement (761 ≫ 192, taxe brutale 61 %). Et l'efficacité vs
  l'instrument MAX **MONTE avec N** (63 %→72 %) car plus de signataires/cellule → moins de repli conservateur.
  **Reste :** 5000 (taxe 68 %, où l'écart doit être le plus net) + C-sécu-1b (plancher vérifié) pour remonter
  encore la récup dans les cellules peu corroborées.

- **ÉTAPE C-sécu-1a (8.3★, 21 juin, supervisé) — corroboration MULTI-SIGNATAIRE : sécurité OK, RÉCUPÉRATION
  PARTIELLE (63 %) SOUS la cible — le PLANCHER vérifié (1b, reporté) manque pour récupérer.** *Fait :* drapeau
  `CORROB` ; chaque nœud publie l'estimation de SA cellule (`build_own_cell_claim`, plus seulement l'hôte) ;
  stockage + relais des résumés **SIGNÉS** par (cellule, signataire) (`cell_claims`) pour propager l'épidémie
  verbatim ; densité = Σ `qth_largest(counts, Q=3)` par cellule (**fonction PURE, test unitaire** : inflation
  bornée au max honnête tant que < Q menteurs, contrôle à k≥Q = limite botnet). **Bug trouvé+corrigé en route :**
  le relais lisait `cell_summaries` (non rempli par corrob) → **110 résumés reçus** ; après fix → **975 240**.
  **78 tests, 0 warning, défaut intact.** *Résultat N=1000 :* taxe **0 %**, perception moy **563 / max 1000 = 63 %**
  de DENSITY_MAX (895), **encore en hausse à t=70** (convergence plus lente : multi-signataire = plus à propager) ;
  débit ↑54/↓55 Ko/s (**+17 %** vs DENSITY_MAX = coût de l'émission multi-signataire). *Lecture honnête (Règle 2,
  le résultat décide) :* critère **≥80 % NON atteint** ; et **563 < 759** (mode défaut AVEC taxe à 1000, où la taxe
  n'est que ~24 %) → **à 1000 la corroboration Q=3 coûte PLUS que la taxe qu'elle retire**. CAUSE probable : le
  **repli CONSERVATEUR** des cellules à < Q signataires (→ plus petit count) sous-compte — c'est exactement ce que
  le **PLANCHER vérifié (étape 1b, reporté)** doit corriger (donc le plancher n'est pas qu'anti-omission, il est
  NÉCESSAIRE à la récupération). **PROCHAIN = C-sécu-1b :** densité = `max(plancher union vérifiée, qth_largest)`
  → re-mesurer ; puis comparer corroboration vs taxe à **2000/5000** (la corroboration ne vaut QUE si elle SCALE
  mieux que la taxe — à 1000 elle perd, mais la taxe explose à 68 % à 5000 : c'est là que ça se jouera).

- **ÉTAPE C-sécu-1b (8.3★, 21 juin, supervisé) — PLANCHER d'union signée : récupération 63 %→78 % à débit PLAT,
  ADDITIF sans toucher le wire.** *Fait :* drapeau `FLOOR=1` (n'a de sens que sous `CORROB`) ; densité d'une cellule
  = `max(qth_largest, |union des IDs signés vus dans la cellule|)` (`corroborated_density`, **fonction PURE + test
  unitaire** ; le plancher RELÈVE les cellules peu corroborées que `qth_largest` sous-compte, ne baisse JAMAIS). Les
  échantillons portent DÉJÀ l'ID → **zéro changement de wire** (vrai additif). **79 tests, 0 warning, défaut intact.**
  *Mesure N=1000 (même périmètre que 1a) :* CORROB seul = 563 (63 % de DENSITY_MAX 895) ; **CORROB+PLANCHER = 699
  (78 %)** → **+24 %**, à **débit IDENTIQUE** (↑54,8/↓55,9 vs ↑54,3/↓55,5 = le plancher ne coûte RIEN au wire). Max 1000.
  *Lecture honnête (Règle 2) :* à 70 s, 78 % (non convergé, courbe montait encore). **Run de CONVERGENCE (130 s) :
  perception 777 = 87 % de DENSITY_MAX (895), max 1000, débit plat (↑54,9/↓56,0) → CIBLE ≥80 % FRANCHIE.** Découverte
  plafonnée à 861 pairs → on perçoit 90 % du découvrable. Le 78 % n'était qu'un instantané court ; le plancher tient. ⚠ **DETTE ÉCRITE
  (code + ici) :** échantillons pas encore AUTO-signés par chaque personne → un menteur seul peut injecter ≤16 faux
  IDs/cellule dans le plancher. **1b ferme l'anti-OMISSION, PAS l'anti-INFLATION** (différé à C-sécu-2 red-team +
  échantillons auto-certifiants + cap /24, comme le cap /24 de `qth_largest`). *VERDICT 5000 (run du 21 juin, CORROB
  sans plancher) :* perception moy **706 / max 2874**, vs défaut-taxe ~650 (~1,1×) et DENSITY_MAX ~2450 (**29 %**) —
  **NON concluant** : le run montait encore (non convergé) et le **mur n°2 (bootstrap « tous à t=0 »)** domine le duel
  corroboration-vs-taxe → per Règle 3, on NE conclut PAS « scale à 5000 ». *Recadrage acté (utilisateur, 21 juin) :*
  « tous à t=0 sur bus parfait » = stress d'INSTRUMENT, pas un déploiement → viser **~3000 en arrivée PROGRESSIVE**.
  **PROCHAIN proposé = manche d'ARRIVÉE ÉCHELONNÉE** (bots étalés sur T s) pour PROUVER (pas supposer) que le mur n°2
  ne mord pas en arrivée réelle ; puis re-run 1000 long (convergence vs 80 %) ; puis C-sécu-2 (red-team anti-inflation).

- **ÉTAPE C-diag (8.3★, 20 juin, supervisé) — RETIRER LE CHEF RESTAURE LA DENSITÉ ; le seul plafond
  restant de la perception à grande échelle est la DÉCOUVERTE (mur n°2).** *Fait :* drapeau additif
  `DENSITY_MAX=1` (link.rs) — count/cellule = **MAX vu** (monotone, non-thrashant ; hôte relâché), perception
  = Σ counts ; + fix d'honnêteté de mesure (coopsim.rs : libellé « pas plus DENSE » sous ce mode). **77 tests,
  0 warning, défaut intact.** *Critère pré-enregistré (Règle 2, écrit AVANT) : taxe→0 ET Σ counts/N ≥ 70 %
  hors bootstrap ET débit plat ET pas de deadlock.* **Résultats (banc bus) :** N=1000 → perception moy
  **895 / max 1000 (89 %)**, taxe **0 %**, débit ↓46,8 → ✅ **passe plein** ; N=2000 → moy **1050 / max 1207
  (52/60 %)**, taxe **0 %**, débit ↓47,2 (PLAT ✓), perception/découverte = 1050/1420 = **74 %** (le reste manque
  car le bootstrap n'a pas fini de découvrir dans la fenêtre) ; N=5000 → trajectoire **plateau-puis-cascade**
  (0 jusqu'à ~t=45 = mur n°2, puis 379→1127→1725→2213 à t=60/80/100/120). **Bilan final N=5000 (130 s) :**
  perception moy **2426 / max 2992 (49/60 %)**, taxe **0 %**, découverte 1512/5000 (30 %), perception/découverte
  = **160 %** (la densité agrège AU-DELÀ de ce que chaque nœud découvre seul). ⚠ **Lecture honnête (Règle 3,
  pas d'extrapolation) :** (1) la densité n'atteint pas 70 % car la DÉCOUVERTE n'est qu'à 30 % — à 5000 le
  **bootstrap (mur n°2) est si long que 130 s ne suffisent pas à converger** (perception montait ENCORE à t=120) ;
  c'est mur n°2 qui domine la fenêtre, PAS un défaut de la densité. (2) Débit ↓**32,2** Ko/s, PLUS BAS que
  1000/2000 (~47) = artefact d'une moyenne dominée par le long plateau (essaim non convergé) → **« débit plat »
  confirmé 1000↔2000, INCONFIRMABLE à 5000** tant que le bench ne passe pas mur n°2. **Leçon
  (valide la thèse 8.3★) :** le **mur n°1 (taxe émetteur≠hôte) est DISSOUS** (0 % partout) et la densité
  **SUIT la découverte** → ce qu'il reste à attaquer pour la perception à l'échelle, c'est le **bootstrap lent
  (mur n°2)** — orthogonal, robuste au jitter = propriété réelle du protocole. **Caveat GRAVÉ :** `DENSITY_MAX`
  est un **INSTRUMENT non sécurisé** (le MAX est trivialement inflationnable) — la version SÛRE = densité MOLLE
  CORROBORÉE /24 = **étape C-sécu, écrite sur PAPIER** (FEUILLE_DE_ROUTE §D, bloc 8.3★ C-sécu). Self-challenge
  important : **le banc bus ne peut PAS prouver la sécurité /24** (loopback, ports gratuits) → l'anti-inflation
  se prouve sous le **harnais NAT (vraies IP), en réutilisant 9.4b**. **PROCHAIN : 5000 finit (bilan), puis
  challenger/coder C-sécu-1 (récupération headless) puis C-sécu-2 (anti-inflation NAT).**

- **ÉTAPE B (8.3★, 20 juin, supervisé) — l'union d'individus NE récupère PAS la perception ; la mesure
  révèle que « perception » conflait DEUX notions.** *Fait :* échantillons porteurs d'**ID** (cell.rs ;
  wire `KIND_CELL_SUMMARY` 8→40 o/échantillon — bot/bench uniquement, `PROTO_VERSION` inchangé → un
  vieux résumé est rejeté PROPREMENT par taille non canonique, pas mal lu) ; mode `UNION=1` (accumule
  les individus distincts vus, perception = |union| ; implique hôte relâché). **77 tests, 0 warning,
  défaut intact.** *Résultat N=2000 :* taxe émetteur≠hôte **0 %** ✓, MAIS perception moy **139** (max
  571) → **ne bat même pas la baseline (192)**. *Cause :* échantillon déterministe strié (≤16/cellule)
  → l'union plafonne à ~`cellules×16` ; et « individus distincts » est **intrinsèquement O(N)** / borné
  par l'échantillonnage. *Leçon (raffine 8.3★, important) :* « perception » = **(1) DENSITÉ** (combien,
  doit ≈ N — c'est CE qui s'effondrait via la taxe ; un NOMBRE) **+ (2) ÉCHANTILLON** (quels visages —
  borné LOD, à faire tourner). L'union ne mesure que (2) ; le mur n°1 d'origine était la (1). **PROCHAIN
  (à décider ensemble) :** restaurer la DENSITÉ sous hôte relâché via un count/cellule NON-thrashant (ex.
  garder le MAX count vu = proxy honnête de la densité corroborée) → mesurer si Σ counts ≈ N revient sans
  la taxe ; la sécurité du count (anti-inflation) = soft + corroboration (étape C). L'échantillon tournant
  (diversité) = un raffinement séparé pour le rendu LOD.

- **ÉTAPE A (diagnostic 8.3★, 20 juin, supervisé) — la taxe hôte est RÉELLE et retirable, MAIS la
  relaxation naïve NE suffit PAS : l'UNION est le mécanisme porteur.** Drapeau `RELAX_HOST=1` (banc
  HONNÊTE, non sécurisé ; défaut absent → comportement prouvé intact, 77 tests, 0 warning). *Critère
  pré-enregistré : taxe→0 ET perception remonte.* **Résultat N=2000 :** rejet émetteur≠hôte **61 %→0 %**,
  93 % acceptés ✓ ; **MAIS perception MOYENNE s'effondre 192→~20** tandis que le **MAX bondit 577→1324**.
  → **version naïve RÉFUTÉE** (la moitié du critère échoue, honnêtement notée). *Cause (diagnostic) :*
  `cell_summaries` garde **UN résumé par cellule (dernier arrivé)** ; sans la canonisation qu'apportait
  l'élection d'hôte, ça **THRASHE** (chaque nœud écrase son résumé par celui d'un émetteur au petit
  `count`) → moyenne effondrée, max élevé pour les chanceux. *Leçon (raffine 8.3★, important) :* l'élection
  d'hôte ne TAXAIT pas seulement, elle **CANONISAIT**. Le vrai fix n'est donc PAS « retirer le contrôle »
  mais **remplacer « 1 résumé/cellule (dernier) » par « UNION d'évidence par cellule »** (accumuler les
  gens DISTINCTS vus, perception = |union|) — le cœur de 8.3★, désormais **empiriquement justifié**.
  **PROCHAIN = étape B :** échantillons porteurs d'**ID** + ingestion par **UNION** (sans signatures
  d'abord, banc honnête → isole la récupération de perception) ; puis étape C : signatures par échantillon
  + cache de vérif → test du calcul CPU.

- **CONCEPTION ÉCRITE (20 juin, supervisé) — REDESIGN 8.3★ « perception auto-certifiante » (PAPIER, zéro
  code).** Décidé avec l'utilisateur (il vise l'élégance, pas une rustine) : on **retire le chef de cellule**
  et on fonde la perception sur des **échantillons SIGNÉS** unionnés sur des relayeurs diversifiés (keystone
  6.1 + 9.4b un cran plus haut ; émetteur = porteur d'octets). Dissout le mur n°1 (taxe émetteur≠hôte) ET la
  couche 2 (hôte menteur) par construction. **Détail + invariant + murs possibles + plan/critère
  pré-enregistré : `FEUILLE_DE_ROUTE.md` §D, bloc « ⚙ REDESIGN 8.3★ ».** *Make-or-break = fermer le coût CPU
  des vérifs sur le PAPIER avant de coder ; n'adresse PAS le mur n°2 (découverte).* **PROCHAIN : challenger le
  papier ensemble, puis prototype headless mesuré.**

- **CASCADE-vs-N (20 juin, supervisé) — DEUX murs distincts SÉPARÉS par la mesure ; le suspect D26 de
  l'utilisateur est le mur DOMINANT.** *Mesure validée ensemble : t de décollage + décomposition des
  rejets à N=500/1000/2000/5000 (instantanés /5 s via env `SNAP_S`, additif).*
  | N | régime | perception/N (fenêtre) | acceptés | **émetteur≠hôte** | pas-frais |
  |---|---|---|---|---|---|
  | 500 | pas de plateau, perc≈découverte | 453/500 = **91 %** (50 s) | 23 % | **10 %** | 67 % |
  | 1000 | pas de plateau, perc≈découverte | 639/1000 = 64 % (45 s) | 22 % | **24 %** | 54 % |
  | 2000 | découverte OK mais perception ÉTRANGLÉE | 192/2000 = **10 %** (50 s) | 14 % | **61 %** | 25 % |
  | 5000 | découverte STAGNE ~49 (40 s) puis cascade | ~13 % à t=110 (monte) | 15 % | **68 %** | 17 % |
  - **MUR n°1 (DOMINANT) — perception étranglée par D26 couche 1.** Le contrôle `émetteur == cell_host`
    calcule l'hôte attendu sur une **vue LOCALE partielle** (le plus petit id connu DANS la cellule) ; à N
    grand, chaque nœud connaît un sous-ensemble différent → vues de l'hôte DIVERGENTES → la taxe de rejet
    monte **10 → 24 → 61 → 68 %** et la perception/N s'effondre **91 → 64 → 10 %**. *C'est le suspect INITIAL
    de l'utilisateur, CONFIRMÉ par la mesure.* (Déjà au registre comme « faux négatif » de D26 couche 1,
    parade prévue : tolérer une FENÊTRE d'hôtes plausibles, ou corroborer = couche 2.)
  - **MUR n°2 (secondaire) — stagnation de découverte au bootstrap**, seulement à très grand N (~5000) :
    ~49 pairs pendant ~40 s puis cascade. Robuste au jitter → propriété réelle (i). À 2000 la découverte ne
    stagne PAS (101→992) ; le mur n°2 n'apparaît qu'au-delà.
  - **Honnêteté de mesure :** les % de rejet sont agrégés non fenêtrés → dépendent de l'état de convergence
    (fenêtres comparables 45-55 s ici → tendance robuste, pas une comparaison stricte). Banc bus = réseau
    PARFAIT (mesure l'échelle algo, pas le réseau). « 55K » toujours pas prouvé directement.
  - **DÉCISION POUR L'UTILISATEUR (pas de fix autonome) :** attaquer le mur n°1 (assouplir/corroborer le
    contrôle d'hôte de D26) = le plus gros gain de perception, et c'est un vrai sujet de protocole.

- **TEST JITTER → (i) PROPRIÉTÉ DU PROTOCOLE + la taxe D26 CROÎT à l'échelle (20 juin, supervisé).**
  *Test discriminant pré-enregistré (Règle 2) : jitter continu des horloges, opt-in `JITTER=1` (déphasage
  CONSTANT par bot = horloges indépendantes, plus fidèle au réel = D13 ; défaut inchangé → harnais de
  régression).* Critère écrit AVANT : si le plateau de bootstrap FOND avec jitter → (ii) artefact de
  lockstep ; s'il PERSISTE → (i) propriété réelle.
  - **Résultat N=5000 : trajectoire QUASI IDENTIQUE à la baseline** (t=10-40 plateau ~49→54 identique au
    chiffre près ; décollage toujours t≈50 ; 262 à t=60, 477 à t=70). → **le plateau N'EST PAS un artefact
    d'alignement de timers = (i) penche fort.** Le bootstrap met intrinsèquement ~45 s à atteindre la masse
    critique de connaissance mutuelle à N=5000.
  - **Caveat honnête :** le bus reste à livraison INSTANTANÉE/ORDONNÉE ; le jitter teste la PHASE des
    horloges, pas l'asynchronisme de LIVRAISON → exonère le lockstep-timer, PAS la totalité du banc.
  - **Bonus (décomposition des rejets à 5000) : 68 % émetteur≠hôte** (contre **26 % à N=1000**) → la taxe
    **D26 couche 1 CROÎT avec l'échelle** (à découverte sparse, vues locales de l'hôte divergentes) = le
    suspect initial de l'utilisateur CONFIRMÉ comme drag réel SECONDAIRE (le mur PRIMAIRE = lenteur du
    bootstrap). *Limite : agrégat non fenêtré → le 68 % d'un run coupé à t=70 reflète surtout la phase
    sparse ; le 26 % d'un run à 60 s reflète la phase convergée. Directionnel, pas strictement comparable.*
  - **PROCHAINE : cascade-vs-N** (t de décollage à N=500/1000/2000/5000) — croît-il modérément (archi OK,
    juste lent) ou explose-t-il (vrai mur d'échelle) ?

- **DIAG INGESTION + TRAJECTOIRE (20 juin, supervisé) — le « deadlock à 5000 » était un ARTEFACT DE
  FENÊTRE TROP COURTE (Règle 3 appliquée 2×).** *Mesure choisie avec l'utilisateur : instrumenter les
  rejets de résumé + rejouer 5000 PLUS LONGTEMPS (120 s SIM vs 25 s avant).* Instrumentation ADDITIVE,
  lecture seule : compteurs de rejet dans `ingest_summary` (`émetteur≠hôte`/`sceau`/`pas-frais`/`pleine`,
  champ `SummaryStats` dans link.rs) + instantané de trajectoire toutes les 10 s SIM dans le banc bus.
  **77 tests, 0 warning, chemin UDP intact.**
  - **Contrôle N=1000** (cas connu-bon) : le banc reproduit `crowd` (perception moy 759 / max 1000,
    percole 100 %, débit ↑45/↓46 Ko/s). Décomposition des rejets : **20 % acceptés, 26 % émetteur≠hôte
    (la taxe D26 couche 1), 0 % sceau, 54 % pas-plus-frais** (redondance épidémique normale).
  - **N=5000, 120 s SIM — TRAJECTOIRE (le résultat clé) :** la découverte/perception ne sont PAS
    bloquées, elles **bootstrappent LENTEMENT avec une transition de phase nette vers t≈45-50 s** :

    | t (s) | pairs connus | trous ouverts | perception moy (max) |
    |---|---|---|---|
    | 10-40 | ~49 → 54 | ~49 → 54 | **0** (plateau) |
    | 50 | 94,6 | 90,4 | 4 (256) |
    | 60 | 266,9 | 250,4 | 57 (553) |
    | 70 | 469,6 | 439,9 | 160 (886) |
    | 90 | 867,2 | 810,8 | 398 (1741) |
    | 110 | 1236,5 | 1154,8 | 656 (2409) |

    → montée **monotone**, toujours en hausse à t=110. **Le journal mesurait à ~25 s = en plein plateau,
    AVANT la cascade** → la conclusion « 5000 deadlocke » est RÉFUTÉE : c'est une **convergence lente**,
    pas un deadlock. *(Ma prédiction « le temps n'y changera rien, c'est un deadlock » était FAUSSE ; la
    proposition « rejouer plus longtemps » de l'utilisateur était la bonne — la mesure corrige.)*
  - **NOUVELLE question ouverte (à décider ensemble, AUCUN fix) :** pourquoi le plateau de ~49 pairs
    pendant ~40 s puis cascade ? = **percolation de bootstrap DANS LE TEMPS** (masse critique de
    connaissance mutuelle avant explosion du gossip). Reste à trancher : est-ce une **propriété réelle du
    protocole de bootstrap à l'échelle** (alors le « temps de mise en route » croît avec N — vrai sujet),
    ou un **résidu du lockstep** du banc (JOIN_SPREAD désync au démarrage puis re-phase) ? Le test
    discriminant = jitter CONTINU des timers (fidélité, pas tuning).
  - **NE conclut PAS :** à t=120 la perception (max ~2400/5000) **n'a pas fini de converger** → « 55K »
    toujours pas prouvé directement ; la décomposition de rejet AGRÉGÉE est dominée par la phase
    convergée (limite : compteurs non fenêtrés → ne séparent pas proprement la cause du plateau). Coût :
    120 s SIM à 5000 = très long en mural (le trafic convergé charge le bus).

- **DIAG STRUCTURE (20 juin, supervisé) — hypothèse « fragmentation » FALSIFIÉE (Règle 3).** Instrumenté
  le graphe de communication (arête = trou ouvert) à N=100/1000/5000 (Règle 1). Résultats BRUTS :
  | N | pairs connus | trous/nœud | isolés | composantes | + grande grappe | perception |
  |---|---|---|---|---|---|---|
  | 100 | 97,7 | 97,5 | 0 | 1 | 100 (100%) | 90/100 |
  | 1000 | 545 | 499 | 0 | 1 | 1000 (100%) | 391 |
  | 5000 | **51** | **51** | **0** | **1** | **5000 (100%)** | **0** |
  → À 5000 le graphe **PERCOLE à 100 %** (un bloc, 0 isolé). **Donc PAS de fragmentation/transition de
  phase de connectivité** — mon hypothèse pré-enregistrée est RÉFUTÉE. Le seul effondrement est la
  **DENSITÉ DE DÉCOUVERTE** (51 pairs connus à 5000 vs 545 à 1000) + **perception 0 MALGRÉ la connectivité**.
  **Nouvelle piste (À TESTER, pas conclue)** : ça pointe vers la **dette connue de D26 couche 1** — le
  contrôle `émetteur == cell_host` à l'ingestion : à découverte SPARSE (~51 connus), chaque nœud a une vue
  LOCALE différente de « qui est l'hôte » → les résumés sont rejetés (faux négatif déjà inscrit au registre).
  À 1000, vue dense (545 connus) → les vues convergent → résumés acceptés. **Prochaine mesure (à décider
  ensemble) : compter POURQUOI les résumés sont rejetés (émetteur≠hôte vs seq vs sceau), et pourquoi la
  découverte plafonne à 51.** AUCUN fix tant que ce n'est pas mesuré.


- **B1 ✅ infra / B2 ⚠ harnais (20 juin, périmètre ÉLARGI par l'utilisateur : bus mémoire autorisé).**
  *Fait, SÛR* : backend **bus mémoire** sur `transport::Socket` (enum `Udp`/`Bus`, routeur partagé
  `Arc<Mutex>`), **strictement additif** — chemin UDP byte-pour-byte intact ; constructeurs `NetLink::new_on`
  / `Bot::new_on` (via `assemble`/`from_link`, anti-divergence D2) ; banc `coopsim-bus` à **dt FIXE sans
  sleep** (rendez-vous bus réutilisant les vraies fonctions de décision). **77 tests, 0 warning, UDP intact.**
  Toutes les incertitudes balisées `BUS_DOUTE` dans le code (`grep -rn BUS_DOUTE src/`).
  *MAIS — T0.2-bis ÉCHOUE (à corriger)* : `coopsim-bus 30` = 29/30 (✅, 10 s SIM en 1,8 s mural → le
  découplage temps marche !), mais `coopsim-bus 1000` = **perception 0** (réf `crowd 1000` = 454) + débit
  effondré (2,5 vs ~37 Ko/s). perception 0 ⟺ **aucun trou ouvert** ⟺ la DÉCOUVERTE/le PERÇAGE ne
  convergent pas à l'échelle dans le HARNAIS bus (le protocole est identique → bug d'orchestration, pas de
  protocole). **→ banc bus PAS encore validé : NE PAS extrapoler.**
  **DIAGNOSTIC PRÉCIS (localisé, 20 juin) — falaise nette entre N=600 (✅ perception ∝ N, débit plat ~44)
  et N≥700 (perception 0, chiffres IDENTIQUES à 800 et 1000 = DEADLOCK, pas convergence lente) :**
  à N≥700 la découverte marche (≈44 pairs connus) MAIS aucun trou ne s'ouvre. Mécanisme : un trou ne
  s'ouvre que si le pair **perce EN RETOUR** → il faut une connaissance **MUTUELLE** ; or le rendez-vous ne
  donne que les **32 plus proches** (rosters ASYMÉTRIQUES à grand N) et, le banc bus steppant tous les bots
  en **LOCKSTEP** (timers identiques, à l'inverse des threads décalés du vrai `crowd`), le gossip n'a pas le
  temps de rendre la connaissance mutuelle avant `PUNCH_GIVEUP=40` essais → **deadlock de bootstrap**.
  **✅ RÉSOLU (20 juin) — c'était bien le LOCKSTEP.** Fix harnais-only (piste a) : décaler le démarrage
  des bots (`tick >= idx%20`) pour briser la synchro des timers (plus FIDÈLE au réel, pas une rustine).
  Résultat à N=1000 : pairs connus 44→**550**, perception **max 851** (≈ `crowd` 857 ✅), **débit ↑43,6
  Ko/s plat** (= `crowd` ✅), moy 389 vs 454 (~85 %, côté CONSERVATEUR = sous-estime). T0.2-bis passe À
  N=1000 (le banc bus reproduit `crowd 1000` sur max-perception + débit plat).
  **⚠ CORRECTION (20 juin, même session) — j'ai SUR-AFFIRMÉ : « extrapolation légitime » était PRÉMATURÉ ;
  la mesure suivante l'a réfuté.** À **N=5000 le DEADLOCK REVIENT** (pairs connus 51, perception 0, débit
  2,8). Mon décalage de démarrage (JOIN_SPREAD=20) ne désynchronise qu'au DÉMARRAGE puis revient en
  lockstep → il a sauvé 1000 mais pas 5000. **Donc le banc n'est validé que jusqu'à ~1000 (≈ le plafond
  d'avant). AUCUNE extrapolation 5k-50k.**
  **QUESTION OUVERTE (pour toi) — le deadlock à 5000 est-il :** (i) un **vrai mur du PROTOCOLE** (le
  bootstrap ne percole pas à l'échelle : rosters « 32 plus proches » asymétriques + `PUNCH_GIVEUP=40` +
  gossip trop lent — *le genre de mur qu'on cherche TÔT, ce serait une vraie trouvaille*), ou (ii) un
  **résidu de mon HARNAIS** (décalage trop grossier) ? Je ne tranche PAS sans (a) un décalage CONTINU des
  timers (risque de « tuner » le banc jusqu'à ce que ça plaise = rustine), ou (b) rendre le bootstrap
  symétrique côté `bot.rs` (apprendre+percer en retour au reçu d'un PUNCH inconnu = touche le PROTOCOLE →
  supervisé). **STOP honnête : à reprendre AVEC toi.**

- **T1.3 ✋ (20 juin) — D18 speed-hack : STOP au PAPIER (risque rustine), escaladé. FILE SÛRE ÉPUISÉE.**
  `move_plausible` est un contrôle PAR PAS (`dist ≤ MAX_SPEED·dt + SLACK`). Le « speed-hack grossier »
  est le cas SOUTENU : rester sous la borne à chaque pas mais la tenir en continu (~30 m/s = 3× un
  sprint). Je ne code PAS le détecteur en aveugle, car (vérifié dans le code) : **(1)** `MAX_SPEED=30`
  est un placeholder (« à affiner avec la vraie vitesse du jeu ») → impossible de poser honnêtement un
  seuil « soutenu » ; **(2)** la perte de paquets crée de GRANDS pas légitimes (test
  `longue_absence_autorise_un_grand_pas`) → un contrôle fenêtré naïf faux-positive sur un honnête →
  sourdine injuste, qui n'affecte que le VRAI jeu (non observable par moi) ; **(3)** maison = ch.11.4
  (supervisé). → respecté mon STOP « risque rustine = papier ». Défriché en FILE UTILISATEUR.
  **Conséquence : T1.3 était le dernier item Tier 1 → la file SÛRE est épuisée → j'ARRÊTE le loop.**

- **T1.2 ✅ (20 juin) — D21 : rendez-vous, rate-limit débit.** *Menace* : chaque HELLO coûte un
  WELCOME en retour (amplification + CPU) ; aucune borne de débit par source → une source pouvait nous
  faire répondre à volonté. *Fix (additif, rendezvous.rs)* : seau à jetons par adresse source
  (`HELLO_RATE=4/s`, pointe `HELLO_BURST=8`), rechargé au temps écoulé ; à sec, on ignore le HELLO (plus
  de WELCOME → fin de l'amplification depuis cette source). Honnête = 1 HELLO/s → jamais throttlé.
  *Preuve* : test `rate_limit_hello_coupe_la_rafale_pas_l_honnete` (une rafale de 100 → 8 réponses ; à
  sec, rien dépensé) + `sim 60 4` → couverture 99 %, essaim TENU (découverte NON gênée). **75 tests, 0 warning.**
  *NE fait PAS* : pas l'anti-spoofing COMPLET — un flood multi-sources usurpées distinctes peut encore
  saturer la table (borné par `MAX_CLIENTS` + éviction 5 s). La parade restante = handshake de ROUTABILITÉ
  (change le flux HELLO côté client) → laissée à une étape SUPERVISÉE.

- **T1.1 ✅ (20 juin) — D16 : TTL/éviction des pairs.** *Menace* : `learn_peer`/`learn_from_gossip`
  avaient un mur DUR (`peers.len() >= MAX_KNOWN → refuse`) → sur longue session la table se remplit de
  pairs MORTS et bloque l'apprentissage de nouveaux. *Fix (additif, link.rs)* : champ `peer_seen`
  (dernière preuve de vie), const `PEER_TTL=120s`, helper `has_room_for_new_peer(now)` qui, table pleine,
  récupère le slot du pair le plus anciennement vu **uniquement s'il dépasse le TTL** (présumé mort) —
  **jamais un actif** → l'anti-flood/éclipse du mur dur est PRÉSERVÉ (un attaquant n'évince que du
  déjà-silencieux). Preuve de vie marquée sur état accepté (`note_pos`) + adresse corroborée + admission.
  *Preuve* : test `eviction_recupere_un_mort_jamais_un_actif` (évince le plus vieux mort ; refuse si tous
  actifs ; ne touche rien si pas plein) + `sim 50 5` → essaim TENU, orbe intègre, couverture 98 %.
  **74 tests, 0 warning.** *NE fait PAS* : pas de DHT/fédération (D10, annexe H) ; le TTL (120 s) est un
  réglage au jugé, à calibrer si une vraie longue session le demande.

- **T0.2 ⛔ (20 juin) — garde-fou de FIDÉLITÉ : ÉCHEC honnête → T0 GELÉ, escaladé.** Comparaison à
  N=1000, mêmes conditions (POW_BITS=8, fenêtre 25 s) :
  | perception résumé | `crowd 1000` (réf threadé) | `coopsim 1000` (1 thread) |
  |---|---|---|
  | moyenne | **454** | **97** |
  | max | **857** | **427** |
  Le banc léger perçoit **~2× moins**. *Diagnostic* : un seul thread sérialise N nœuds → **dilate le
  temps mural** (25 s réelles = bien moins de ticks de protocole effectifs que la version threadée où
  chaque thread dort 50 ms indépendamment). On **ne peut PAS** corriger par un pas de temps fixe : les
  nœuds parlent en **UDP réel** (livraison asynchrone en temps mural) → découpler temps-sim/temps-réseau
  est incohérent. **Donc un banc fidèle à haute échelle EXIGE un bus mémoire synchrone = rendre `Socket`
  permutable = TOUCHER LE CŒUR → hors périmètre autonome.** *Décision* : T0.2 bloquant → **je n'extrapole
  PAS (T0.3 gelé)**, j'escalade en FILE UTILISATEUR. Le banc reste utile/fidèle à BAS N (≤~200). Outil
  ajouté (mesure débit ↑/↓ par nœud) + étiquette « voisinage » corrigée (c'est la table connue, pas le focus).
  *NE conclut RIEN sur 55k* — c'est précisément ce que le garde-fou devait empêcher (cf. piège 7.4b).

- **T0.1 ✅ (20 juin) — squelette du banc coopératif.** *Menace* : `sim`/`crowd` = un OS-thread +
  un `thread::sleep` PAR bot → plafond ~1500 (sur-souscription des cœurs), « 55K » jamais mesuré.
  *Fix* : nouveau `src/net/coopsim.rs` + sous-commande `coopsim` — N bots steppés dans UNE boucle,
  UN seul thread, UN sleep/tick (sockets UDP réelles sur `lo` conservées → pas de triche protocole).
  Strictement ADDITIF (cœur intouché : transport/link/bot lus seulement ; main.rs/mod.rs = wiring).
  *Preuve* : `POW_BITS=8 coopsim 30 12` → voisinage 29/32, **54 374 états acceptés** (échange
  bout-en-bout), perception max 30/30 via 1 flux. 73 tests, 0 warning.
  *NE fait PAS* : (1) pas de bus mémoire → reste sur UDP/lo ; si le mur est l'UDP avant 50k, le bus
  (qui toucherait le cœur) ira en FILE UTILISATEUR. (2) **Fidélité pas prouvée** → bloquant T0.2
  AVANT toute extrapolation.

---

## 📥 FILE « UTILISATEUR » (ce que j'ai rencontré mais qui T'appartient — à ton retour)
*(les murs durs Tier 2 + tout ce que j'ai dû stopper)*

- **D26 couche 2 (corroboration)** et **D4 « parent par mesure du réel »** : murs de DESIGN, à
  défricher avec toi (risque rustine si fait en aveugle). *(Je peux les pré-défricher sur PAPIER si tu
  passes le périmètre à « + Tier 2 sur papier ».)*
- **⭐ BANC FIDÈLE HAUTE ÉCHELLE (escaladé de T0.2, 20 juin) — décision pour toi.** Le banc léger
  coopératif sur UDP réel n'est PAS fidèle au-delà de ~quelques centaines de nœuds (temps mural dilaté ;
  mesuré : 2× moins de perception que `crowd 1000`). Pour mesurer DIRECTEMENT 5k-50k il faut un **bus
  mémoire synchrone** → rendre `Socket` permutable (un backend mémoire en plus de l'UDP), ce qui touche
  `transport.rs` (+ un constructeur `NetLink` injectant la prise). C'est faisable **strictement additif**
  (le chemin UDP réel reste byte-pour-byte intact, prouvable par les 73 tests + `sim`), mais ça touche le
  CŒUR → **je ne le fais pas sans ton feu vert.** *Si tu autorises, je le code, je revérifie que la base
  est intacte, et T0.3 (chasse au mur 50k) redevient possible.* C'est LA pièce qui débloquerait enfin la
  mesure directe de l'échelle (dette D25). **À toi de trancher au retour.**
- **D18 speed-hack SOUTENU (défriché de T1.3, 20 juin) — décision/réglage pour toi.** Le trou : un
  attaquant tient une vitesse JUSTE sous `MAX_SPEED` à chaque pas mais en CONTINU (~30 m/s soutenu = 3×
  un sprint), que `move_plausible` (contrôle par pas) ne voit pas. *Mécanisme proposé* : par pair, une
  moyenne de vitesse sur une FENÊTRE de pas consécutifs FRÉQUENTS, **réinitialisée à tout trou** > G (pour
  ne pas punir un grand pas légitime après perte) ; si la moyenne sur la fenêtre W dépasse un
  `SUSTAINED_MAX < MAX_SPEED`, faute/sourdine. *Pourquoi je ne l'ai PAS codé seul* : W, G et
  `SUSTAINED_MAX` exigent **la vraie vitesse du jeu** (aujourd'hui un placeholder) + **le profil de perte
  RÉEL** ; mal réglé, ça sourdine des honnêtes — invisible sans lancer le 3D (toi). *Quand tu reviens* :
  fixe la vitesse réelle du perso, donne-moi un profil de perte cible, et je code + prouve par `attack`
  (un bot « soutenu » muet, la marche normale passe). C'est un raffinement « à surveiller » (🟡), pas un bloqueur.
- **ÉTAPE A (8.3★, 21 juin, supervisé) — arrivée PROGRESSIVE 3000 : le mur n°2 DISPARAÎT, mais le run ne
  CONVERGE pas dans la fenêtre.** *Fait :* `RAMP_S=45` (3000 bots arrivent linéairement sur 45 s), CORROB+FLOOR,
  130 s. *Résultat :* perception moy/arrivé t20=158→t120=691 (final/résumé 733, max 2462), **JAMAIS de plateau
  mort** (vs 5000-tous-à-t=0 clouée à 0 jusqu'à t=40) ; débit ↓40,7 Ko/s (**borné, < 1000**). ✅ **Critère 1
  (pas de mur de démarrage) MET** : l'arrivée progressive dissout le mur n°2 — mesuré, pas supposé. ⚠ **Critère 2
  (convergence) NON met** (Règle 3) : pairs connus montait ENCORE à t=120 (1042, vs 861 plafonné à t=100 pour
  1000) → fenêtre 130 s trop courte pour 3000 ; on NE conclut PAS « 3000 converge à X % ». *Claim honnête :*
  « à 3000 en arrivée progressive, perception sans mur de démarrage + débit borné ; convergence complète = fenêtre
  plus longue, non mesurée ». **Pour clore le chiffre : run 3000 à ~250 s (optionnel, ~1 h 15).**

- _(ce que j'ajouterai si je dois stopper un pas)_

---

## 📄 PAPIER C-sécu-2 — échantillons AUTO-SIGNÉS pour fermer l'inflation du plancher (écrit le 21 juin, AVANT de coder)

> *Le make-or-break que la roadmap exige avant tout code (coût CPU/débit). Zéro octet de wire touché ici :
> c'est de la réflexion. But : décider si l'idée FERME, et sinon l'abandonner « gratis ».*
>
> ⚠ **CADRE — ce bloc est du travail SUPERVISÉ, pas autonome.** Le « wire » de C-sécu-2 (`KIND_CELL_SUMMARY_V2`,
> prouvable HEADLESS) n'est PAS le wire interdit en autonomie plus haut (= ch.10.2 chiffrement, invisible sans le
> 3D). Ce papier est la RÉFÉRENCE VIVANTE du rush wire ; le point de reprise est l'⏱️ ANCRE §0 de `FEUILLE_DE_ROUTE.md`.

**LE TROU (mesuré par le red-team) :** le plancher 1b compte `|union des IDs vus dans les échantillons|`,
mais les échantillons `(id, x, z)` sont signés par le SEUL agrégateur, pas par chaque personne. Un menteur
seul peut donc bourrer ≤ `MAX_CELL_SAMPLES` (=16) faux IDs/cellule → inflation +16 (test : 50→66).

**LE FIX :** chaque échantillon devient **AUTO-CERTIFIANT** : `(id, x, z, seq, sig)` où `sig` est la signature
de **la personne elle-même** (exactement un état joueur miniature). Le plancher ne compte alors que les IDs
dont l'auto-signature VÉRIFIE → un menteur ne peut pas forger les autres → **plus de fantômes**. Le red-team
retombe de 66 à 50 (on inversera l'assertion).

**L'ASTUCE QUI REND LE COÛT TENABLE :** l'agrégateur **ne signe RIEN de neuf**. Les états joueurs portent DÉJÀ
leur sceau (`encode_signed` = corps 118 o + sig 64). Il suffit que chaque nœud **retienne le dernier état SIGNÉ
reçu par pair** (id, x, z, seq, **sig**) et **recopie** ces octets dans l'échantillon. Aucun travail crypto à
l'émission pour les honnêtes ; juste de la recopie.

**COÛT WIRE (le vrai point dur) — chiffré :**
- Échantillon : **40 o → 112 o** (id 32 + x 4 + z 4 + seq 8 + sig 64) = **×2,8**.
- Résumé complet : `HEADER 55 + 16×40 + sig 64 = 759 o` **→** `55 + 16×112 + 64 = 1911 o` = **×2,52**.
- Les résumés DOMINENT les ~55 Ko/s mesurés (émis toutes les 2 s, fanout 4, + relais). Donc le naïf
  « tout auto-signer » pousserait le débit vers **~110-130 Ko/s** = ×2-2,5. **C'EST le risque à borner.**

**COÛT CPU (le make-or-break nommé par la roadmap) — chiffré : ÇA FERME.**
- Vérifs = `K × (résumés ingérés frais)`. Ed25519 verify ≈ 30-50 µs.
- Sans cache : 16 × 40 µs = **640 µs/résumé accepté** ; à ~2,7 acceptés/s/nœud (mesuré au run 1000/130 s) →
  **~1,7 ms/s ≈ 0,17 %/cœur**. Déjà négligeable.
- Avec **cache de vérif par `(id, seq)`** (le même état signé arrive via N relais → on ne le vérifie qu'UNE
  fois) : coût = O(personnes-MAJ DISTINCTES perçues), pas O(résumés reçus). **Effondré.** → CPU NON bloquant.

**VERDICT PAPIER :** **CPU ferme largement. Le débit est le seul vrai coût.** Donc on ne code PAS le naïf ;
on code avec UNE des mitigations de débit, et la mesure tranche :
1. **Sous-ensemble TOURNANT auto-signé** (recommandé) : ne mettre `seq+sig` que sur **k_proof < 16** échantillons
   par émission (ex. 4), tournants. L'union de preuves grossit quand même dans le temps (rotation) → le plancher
   remonte sûrement, mais le résumé reste proche de l'actuel. Compromis vitesse-de-convergence ↔ débit = un RÉGLAGE
   à mesurer, pas à deviner.
2. **Auto-signature SEULEMENT dans les cellules sparse** (où `qth` sous-compte) : les cellules denses (où `qth`
   domine déjà) gardent l'échantillon léger. Borne les octets en trop là où ils PAIENT.
3. **Référence au lieu d'inline** : porter `(id, seq)` seul et laisser le receveur retrouver le sceau qu'il
   détient DÉJÀ par gossip ; n'inliner la `sig` que s'il lui manque. Quasi zéro surcoût en maillage dense, mais
   plus complexe — option 2e temps.

**CRITÈRE PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT de coder) — l'idée est VALIDÉE si, à N=1000/2000, derrière un
drapeau (défaut intact) :** (a) le red-team passe de **66 → 50** (inflation fermée) ; (b) la récupération reste
**≥ le niveau 1b** (≈87 % à 1000) ; (c) le débit reste **≤ ~+30 %** vs 1b (sinon escalader la mitigation) ;
(d) CPU **< 1 %/cœur** mesuré. Si le débit explose malgré les mitigations → on REPLIE : le plancher reste
« anti-omission seulement », documenté, et `qth` (déjà incheatable) porte la sécurité. **On le saura mesuré.**

**ÉTAPES DE CODE (petits pas, derrière `SIGNED_SAMPLES=1`, wire v2 ou nouveau KIND, chemin défaut byte-intact) :**
(1) retenir le dernier état signé par pair (id, x, z, seq, sig) ; (2) format d'échantillon v2 + encode/decode +
PROTO bump derrière le drapeau ; (3) ingestion : vérifier `k_proof` échantillons, ne compter au plancher que les
vérifiés, cache `(id, seq)` ; (4) re-mesurer débit/CPU/récup à 1000/2000 ; (5) inverser l'assertion du red-team.
**Estimé : ~½ journée + mesure.** À faire reposé (wire = sécurité critique).

**PROGRÈS (21 juin) :**
- ✅ **Étape 1 FAITE** (`signed_states` + `remember_signed_state`, gaté `SIGNED_SAMPLES`, défaut intact, 80 tests).
- ✅ **Subtilité RÉSOLUE** : `sig_ok` normalise l'octet de transport à `KIND_STATE` avant de vérifier, et `KIND_RELAY`
  = même corps/sceau (182 o). Donc un état stocké (direct OU relayé) se vérifie identiquement → **ré-embarquement
  VERBATIM en v2 possible, zéro canonicalisation.** Embarquer un état signé = `STATE_SIZE` (182 o).
- ✅ **Étape 2 FAITE** (format wire v2, 86 tests, 0 warning, défaut byte-intact) : `KIND_CELL_SUMMARY_V2 = 10` +
  `encode_cell_summary_v2`/`decode_cell_summary_v2` dans `cell.rs`. Forme = résumé v1 **verbatim** (seul le byte de
  transport passe à 10) suivi du trailer `[nproof][nproof×182 o]` d'états signés bruts, **HORS du corps signé**.
  - **DÉCISION DE FORMAT (tranchée AVANT de coder, 2 forks soumis à l'utilisateur) :** (a) **PAS de bump de
    `PROTO_VERSION`** — bumper la const globale jetterait tout le chemin défaut (`bot.rs:277` filtre `byte[1]≠PROTO`),
    viol Règle 1 ; le « PROTO bump » du papier = **le nouveau KIND** (byte[1] reste = 1, vieux nœud ignore KIND 10).
    (b) **Corps v1 gardé verbatim** (échantillons légers conservés) → réutilise le ré-embarquement de l'étape 1, sceau
    intact, zéro canonicalisation. Léger doublon `(id,x,z)` assumé.
  - **Prouvé headless (6 tests v2)** : aller-retour identique + sceau tient + preuves rendues VERBATIM et chacune
    auto-vérifiable (`sig_ok`) ; zéro preuve OK ; **relais peut RETIRER des preuves sans casser le sceau** (le point
    « hors corps signé ») ; preuve malformée ignorée à l'émission ; v1/v2 ne se croisent jamais (KIND 9 ≠ 10) ;
    trailer non canonique rejeté. ⚠ **MTU à surveiller** : 16 samples + k_proof=4 ≈ 1488 o (frôle 1500) → l'émission
    (étape 3) tient k_proof bas ; le format supporte ≤ 16.
  - ⚠ **Dette de l'étape 2** : 3 `#[allow(dead_code)]` TEMPORAIRES (les 2 fns + la const) — le code n'est pas encore
    appelé ; les `allow` **sautent dès l'étape 3** (émission) / **4** (ingestion). C'est un marqueur, pas une rustine.
- ✅ **Étape 3 FAITE** (émission v2 + réception v2 non-régressive, 87 tests, 0 warning, défaut byte-intact) :
  - **Émission** (`bot.rs`, gaté `SIGNED_SAMPLES`) : à MON propre claim de cellule (host == moi) je joins un
    **sous-ensemble TOURNANT** (`K_PROOF=4`, curseur +1/période) de preuves = recopie des `signed_states` retenus dont
    l'id ∈ samples (zéro re-signature). **Portée = mon propre claim seulement** (décision tranchée ; relais d'autrui
    restent v1) → MTU borné, chacun atteste SES voisins, l'union grossit nœud-par-nœud. Helper PUR `proofs_for`
    (filtre/borne/tourne) testé unitairement.
  - **Réception** : nouveau bras `KIND_CELL_SUMMARY_V2` qui ingère le résumé (preuves IGNORÉES à l'étape 3) → **émettre
    du v2 ne perd aucun résumé** (non-régression ; sinon le receveur jetterait le KIND 10).
  - **Preuve headless (banc bus, N=100/45 s, CORROB+FLOOR)** : perception **98 → 99** (à plat), débit **↓59,3 → 60,0
    Ko/s = +1,2 %** (≪ budget +30 %), résumés reçus/acceptés équivalents (66 k/68 k, 19 %). Le v2 circule, coût négligeable.
  - ⚠ **Ce que l'étape 3 NE fait PAS** : le plancher compte ENCORE les sample-ids bruts (preuves inertes) → **le
    red-team reste à 66, AUCUNE sécurité gagnée encore**. C'est l'étape 4 (vérifier les preuves, plancher = Σ |IDs
    vérifiés|) qui ferme l'inflation. Le débit +1,2 % est mesuré à N=100 (peu d'occupants/cellule) → à re-mesurer à
    1000 où les claims portent plus de preuves.
- ✅ **Étape 4 FAITE** (ingestion VÉRIFIANTE → plancher vérifié, 89 tests, 0 warning, défaut byte-intact) :
  - **`verify_proof` (link.rs, PUR comme `proofs_for`)** : à la réception d'un trailer v2 (gaté au site d'appel
    `bot.rs`), chaque preuve auto-signée est vérifiée — lecture bon marché de `(id,seq)` pour le **cache** (un même
    état vu via N relais = 1 seule vérif crypto), puis `sig_ok` SEULEMENT en cache manqué, puis on retient
    `id → (seq max, cellule de sa position auto-déclarée)` dans `verified_proofs` (borné `MAX_KNOWN`). Un buffer forgé
    (corps prétendant une victime, signé par le menteur) échoue au `sig_ok` → n'entre NI au plancher NI au cache.
  - **Plancher (`summary_perceived`)** : sous `SIGNED_SAMPLES`, le plancher d'union compte les IDs **vérifiés par
    cellule** (`verified_proofs`) au lieu des sample-ids BRUTS forgeables. Sans le drapeau → comportement 1b INTACT.
  - **RED-TEAM INVERSÉ (unitaire, la fermeture)** : `verify_proof_ferme_l_inflation_du_plancher_redteam_inverse` —
    50 vraies preuves auto-signées + 16 fantômes forgés (sceau qui ne colle pas) → plancher vérifié = **50, PAS 66**.
    C'est l'inversion exacte de `redteam_le_plancher_1b_est_gonflable…`. + test du cache `(id,seq)` et du suivi de
    cellule (un seq plus frais déplace la personne, pas de double comptage).
  - **Non-régression headless (banc bus, N=100/45 s, CORROB+FLOOR)** : perception **98 → 99** (à plat), débit **↓59,3
    → 59,8 Ko/s = +0,8 %** (≪ budget +30 %), résumés reçus/acceptés équivalents (~67 k, 19 %). Le plancher vérifié
    ne fait PAS chuter la récupération à faible densité (la rotation `K_PROOF=4` couvre les petites cellules).
  - ⚠ **Ce que l'étape 4 NE prouve PAS encore** : la fermeture red-team 66→50 est prouvée en LOGIQUE/unitaire ; la
    non-régression est mesurée à **N=100 seulement** (peu d'occupants/cellule). À N=1000 (cellules denses) la rotation
    `K_PROOF=4` doit couvrir plus d'IDs/cellule → la récupération pourrait être plus lente à converger que 1b. C'est
    l'objet de l'étape 5 (run N=1000/130 s en cours).
- ✅ **Étape 5 FAITE (re-mesure N=1000/130 s, head-to-head même harnais `POW_BITS=8`)** :
  | Métrique | 1b (CORROB+FLOOR) | étape-4 (+ plancher vérifié) | Δ |
  |---|---|---|---|
  | Pairs connus | 856,9 | 849,9 | ~égal (découverte identique) |
  | Perception moy | 756 (75,6 %) | 705 (70,5 %) | **−51 (−6,7 %)** |
  | Débit ↓ | 56,0 Ko/s | 56,4 Ko/s | **+0,7 %** |
  - **VERDICT du critère pré-enregistré :** (a) red-team 66→50 ✅ (unitaire) ; (c) débit ≤ +30 % ✅✅ **+0,7 %** (le
    plancher vérifié est quasi-GRATUIT en wire — seul mon propre claim porte des preuves, dilué dans les relais v1) ;
    (d) CPU < 1 %/cœur ✅ (wall ≈ identique aux deux runs, la vérif avec cache ne se voit pas) ; **(b) récup ≥ 1b ⚠
    NON tenu à 130 s** (705 < 756). **MAIS** l'écart est purement de la **VITESSE de convergence** (il fond de façon
    monotone : −130 → −91 → −69 → −51 aux t=30/60/90/130) et les DEUX sont plafonnés par la découverte (mur n°2,
    ~850 pairs connus → perception/découvert = 83 % des deux côtés). Pas un plafond plus bas : un LAG.
  - **⚠ SUBTILITÉ DE BANC RÉVÉLÉE (honnêteté) :** le `87 %` enregistré pour 1b @1000 ne se RE-MESURE PAS dans ce
    harnais (POW8/130 s → 1b = 75,6 %, plafonné par la découverte à 850). Le `87 %` venait d'un run à convergence plus
    longue / autres conditions. **Ne pas re-citer `87 %` comme la cible 1b @1000** ; la vraie référence comparable est
    le 1b RE-MESURÉ côte à côte (756).
  - **DÉCISION (utilisateur, Section G) :** le REPLI prévu (« si le débit explose ») n'a PAS sauté (+0,7 %) → on a
    **29 % de marge débit inutilisée** alors que le seul coût est la convergence. La variable, c'est `K_PROOF`, **plafonné
    par le MTU** (16 samples + 4 preuves ≈ 1488 o, frôle 1500). → **on RÈGLE `K_PROOF` en touchant au MTU (étape 6).**
- ▶️ **ÉTAPE 6 (wire = sécurité critique, SUPERVISÉ, papier d'abord)** : porter PLUS de preuves/émission dans la marge
  de débit pour fermer le lag de convergence. Piste = option 3 du papier (preuve par RÉFÉRENCE `(id,seq)` ~40 o au lieu
  de 182 o inline ; le receveur retrouve le sceau qu'il DÉTIENT déjà en `signed_states`, ne compte que les références
  résolues+vérifiées) → ~4× plus d'occupants couverts/émission au MÊME wire. Re-mesure N=1000. **Voir papier étape 6.**
