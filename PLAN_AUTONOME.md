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
- _(ce que j'ajouterai si je dois stopper un pas)_
