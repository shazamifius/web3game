# Feuille de route détaillée — web3game

> **But de ce document.** Le `README.md` est le *résumé* (où on en est, comment lancer).
> CE fichier est le *plan complet et honnête* : tous mes doutes d'ingénieur, et le
> programme détaillé pour les résoudre. Il est volontairement long. On l'écrit
> AVANT de coder, pour avoir une carte claire du gros chantier qui vient.
>
> **Comment le lire.** Section **B** = ce qui est fait. Section **C** = l'inventaire
> des doutes (D1…D21), c'est le cœur. Section **D** = le programme (chapitres 7→14),
> chaque chapitre ferme des doutes précis. Section **E** = comment tester *pour de
> vrai* avec une seule machine. Section **F** = l'ordre conseillé. Section **G** =
> les décisions qui t'appartiennent.
>
> **Règle d'or rappelée.** On ne vise pas l'inviolable absolu (ça n'existe pas). On
> vise : *chaque attaque devient soit impossible, soit chère, soit attribuable et
> bannie* — et *chaque joueur, du 0 connexion au 2 Gb/s, a la meilleure expérience
> possible pour SON lien*.

---

## 0. ▶️ POINT DE REPRISE (lis ça en premier, surtout si nouvelle session)

> ### 🎯 LE POURQUOI + RÈGLES + PLAN D'ATTAQUE (posé le 20 juin 2026 — lire EN PREMIER)
> **Le but ultime :** un **événement P2P sans serveur réunissant ~55 000 personnes** sur Unreal Engine, pour fêter
> le « départ du web3 » (décentralisé / identité possédée — PAS un token) ET prouver qu'on casse la limite du
> serveur. **55K = la jauge réelle de la plus grande salle de concert au monde**, jamais réuni dans un jeu (les MMO
> shardent ; Fortnite = instances de ~100 ; EVE ~6-8K sur gros serveurs). Version réalisable = **présence par LOD**
> (~32 voisins nets + la marée en imposteurs agrégés).
>
> **LES 5 RÈGLES (le contrat de travail) :**
> 1. **La base qui marche est INTOUCHABLE.** Chaque pas : compile → test → preuve → commit → push. Si un pas menace
>    le cœur, on ne le fait pas. *L'humiliation ne serait pas d'échouer à 55K — ce serait d'abîmer la base.*
> 2. **Petits pas, lentement.** Une seule chose prouvable à la fois.
> 3. **55K = BOUSSOLE, pas échéance.** On bâtit l'archi qui *pourrait* y aller ; on ne court pas après le nombre.
> 4. **SECRET jusqu'à solide.** On ne pitche personne (même les contacts VRChat/Vocaloid/Miku) tant que ce n'est pas
>    prouvé — code expérimental, un mur invisible nous attend peut-être.
> 5. **Chercher le mur TÔT et SUR LE PAPIER**, avant de coder par-dessus. *(Maths pour l'asymptote ; on NE bâtit PAS
>    le gros simulateur D25 maintenant — 55K n'étant pas le but, le prouver n'est pas urgent.)*
>
> **LE PLAN D'ATTAQUE (3 horizons) :**
> - **🟢 H1 — `10.1 identité persistante` ✅ FAIT (20 juin).** Clé sauvée `~/.web3game/<profil>.key` (perms 600),
>   rechargée au lancement → même identité entre sessions (D14 fermé). `new` éphémère intact (simu/bots) ;
>   `new_persistent` pour le vrai jeu (`a.key`≠`b.key`). 68 tests, 0 warning. **PROCHAINE = H2.**
> - **🟡 H2 — défricher SUR LE PAPIER les 2 murs les plus probables** avant de coder Phase B : **D4** (incitation au
>   relais = problème de mécanisme) et **D26** (agrégateur/parent menteur). Si l'un est un vrai mur, le voir gratuit.
> - **🟠 H3 — `Phase B inclusivité`** (D3/D4/D5) une fois H2 défriché. Puis NAT (D17) → voix → chiffrement → Unreal.
>
> **🎮 DÉCISION MOTEUR DE PRÉSENTATION (posée le 22 juin 2026, avec l'utilisateur).** Bevy est jugé pénible à itérer
> côté rendu/game-feel → on passera à **Unreal Engine**, MAIS **PAS maintenant : réseau d'abord** (on finit d'abord
> `C-sécu-2 ét.6`, ordre roadmap respecté). Le switch ne touche QUE la présentation : **le CŒUR RÉSEAU Rust reste
> INTOUCHABLE** (règle n°1 — la « base » = le cœur P2P, jamais Bevy ; cf. README « la logique resterait la même avec
> n'importe quel moteur 3D »). **Pont retenu = SIDECAR (option B)** : le cœur Rust tourne en process séparé (il fait
> déjà du réseau → quasi rien à réécrire), Unreal = client mince sur socket local ; migration éventuelle vers un
> binding FFI/cdylib (option A) plus tard SI la frontière IPC gêne. **Frontière mesurée (`grep bevy src/net`)** :
> cœur PUR engine-agnostic (~3 800 l : crypto/wire/gossip/transport/message/cell/aoi/coopsim/attack/rendezvous…) =
> ne bouge pas ; GLU Bevy (~4 400 l, surtout `netcode/` + `orb.rs` + `skin.rs`) = à refaire côté Unreal. Étapes
> essentielles le moment venu : (1) extraire le cœur pur en crate `web3core` ; (2) figer un contrat d'interface
> minimal (poll avatars distants {id,pos,couleur,pseudo} · push ma position · état orbe) ; (3) rebâtir hub/portails/
> arcade/île/avatars/nameplates dans Unreal. **Tant que ét.6 n'est pas close, on ne démarre PAS Unreal.**

> ### ⏱️ ÉTAT COURANT + PROCHAINE ACTION (L'ANCRE anti-dérive ; maj 21 juin 2026)
> *Lire CE bloc + le 🎯 ci-dessus suffit pour reprendre au bon niveau. Tout ce qui suit dans §0
> est un JOURNAL d'archive — ne le relire qu'au besoin, via `grep`. Anti-collapse : on s'ancre ici,
> on ne se rejoue pas tout l'historique à chaque session.*
>
> **OÙ ON EN EST :** chapitres **0→7 faits**, **ch.9 (confiance dure) tenu**, **ch.8 « foule dense » Phase A
> BOUCLÉE** (FOCUS ~32 / CONSCIENCE LOD / gossip / résumés de cellule), **10.1 identité PERSISTANTE FAIT**
> (D14 fermé). **Investigation 8.3★ EN COURS** (perception à grande échelle) : un **banc bus mémoire** (D25,
> dette de harnais levée) mesure désormais jusqu'à 5000+ nœuds sur un PC. Il a **séparé DEUX murs**, et le
> redesign **« perception auto-certifiante » RETIRE le chef de cellule** (l'élection d'hôte de D26 couche 1
> était le mur dominant — mesuré, pas supposé). **80 tests, 0 warning, cœur + chemin UDP byte-pour-byte intacts.**
>
> **⚙ 8.3★ — LES DEUX MURS DE LA PERCEPTION (mesurés au banc bus, 20-21 juin) :**
> - **MUR n°1 — la taxe `émetteur≠hôte` (D26 couche 1) = DISSOUS.** L'ingestion n'acceptait un résumé que si
>   `émetteur == cell_host` (le plus petit id connu de la cellule) ; à grand N les vues divergent → rejet
>   **10→68 %**, perception/N s'effondre **91→10 %**. **Étape C-diag (`DENSITY_MAX`) :** on retire l'élection,
>   count/cellule = MAX vu → taxe **0 %**, densité RESTAURÉE : **N=1000 → 89 % (perception 895/1000)**, débit ↓
>   PLAT (~46-47 Ko/s). Mesuré, pas argumenté.
> - **MUR n°2 — bootstrap LENT de la découverte** (~45 s à 5000 puis cascade ; robuste au jitter = propriété
>   RÉELLE du protocole). C'est désormais **LE plafond restant** de la perception à l'échelle : la densité SUIT
>   la découverte (N=2000 → 52 %, N=5000 → 49 % et MONTAIT encore en fin de fenêtre). **Orthogonal au 8.3★.**
> - **Sécurité — chantier C-sécu (densité molle CORROBORÉE, remplace l'instrument gonflable `DENSITY_MAX`) :**
>   - ✅ **C-sécu-1a** — corroboration multi-signataire (`CORROB` : Q-ième plus grand count par /24 distinct =
>     `qth_largest`, Q=3 ; fonction PURE testée : inflation bornée au max honnête tant que < Q menteurs, contrôle
>     à k≥Q = limite botnet).
>   - ✅ **C-sécu-1b** — plancher d'union signée → **récupération 87 % à convergence @1000 (cible ≥80 % FRANCHIE)**,
>     débit PLAT, ADDITIF (zéro octet de wire). ⚠ Dette laissée : échantillons pas auto-signés → un menteur seul
>     peut injecter ≤16 faux IDs/cellule (ferme l'anti-OMISSION, pas l'anti-INFLATION).
>   - 🔧 **C-sécu-2 EN COURS** — échantillons AUTO-SIGNÉS pour fermer cette inflation. Papier make-or-break écrit
>     (CPU ferme largement ; le DÉBIT est le seul vrai coût → mitigations prévues). **Étapes 1-2-3-4/5 FAITES** :
>     ét.1 = `signed_states` (gaté, ré-embarquement verbatim) ; ét.2 = format wire `KIND_CELL_SUMMARY_V2` (résumé v1
>     verbatim + trailer de preuves HORS corps signé ; PAS de bump `PROTO_VERSION` = nouveau KIND → défaut byte-intact) ;
>     ét.3 = émission+réception v2 (mon propre claim joint `K_PROOF=4` preuves tournantes) ;
>     ét.4 = ingestion VÉRIFIANTE (`verify_proof` : sceau + cache `(id,seq)` ; plancher = Σ |IDs vérifiés ∈ cellule|
>     sous `SIGNED_SAMPLES`, sinon 1b intact) ; ét.5 = re-mesure N=1000/130 s ; **ét.6 = FERMER le lag (FAIT 22 juin).**
>     ✅ **Inflation FERMÉE** (red-team inversé unitaire **50, pas 66**) **à débit quasi-GRATUIT**, CPU non visible.
>     ✅ **ÉTAPE 6 FAITE (22 juin) — C-sécu-2 BOUCLÉ. 90 tests, 0 warning.** Au lieu de l'option 3 (trailer par
>     RÉFÉRENCE, risquée), une observation du code a ouvert **6-B = AUTO-PEUPLEMENT du plancher depuis `signed_states`
>     (états signés DÉJÀ détenus, comptés par leur cellule auto-déclarée) — COÛT WIRE NUL.** Mesure head-to-head
>     N=1000/130 s (`POW_BITS=8`, `SELFPOP=0` isole l'étape-4) : **perception 781 vs étape-4 709 (+10,2 %) et vs 1b 746
>     (+4,7 %, e6 PASSE DEVANT 1b dès ~t=55 s)**, **débit +0,9 % (zéro octet ajouté)**, red-team **50** (sécu préservée
>     PAR CONSTRUCTION : `signed_states` n'a que du `sig_ok`). e6 > 1b = fidélité supérieure (compte TOUS les états signés
>     détenus d'une cellule, pas ≤16 sample-ids gonflables), PAS de l'inflation. ⚠ Dettes mineures : (i) CPU +6 % mural
>     (décodage `signed_states` sur le chemin métrique → mémoïser si ça pèse sur le frame du vrai jeu) ; (ii) un red-team
>     bout-en-bout via l'auto-peuplement durcirait la preuve. **Détail + tableau : `PLAN_AUTONOME.md` § PAPIER ÉTAPE 6.**
>   - *Caveat permanent : `qth`/plancher prouvés en LOGIQUE + récupération headless ; l'anti-inflation /24 RÉEL
>     attend le harnais NAT (vraies IP, réutilise 9.4b). Détail : §D, blocs « REDESIGN 8.3★ » + « ÉTAPE C-sécu » ;
>     papier complet : `PLAN_AUTONOME.md` § PAPIER C-sécu-2.*
>
> **3 MESURES « TROP GENTILLES » à ne pas se mentir (détail au 🧾 registre) :** (a) « couverture » = pairs
> CONNUS, pas ENTENDUS (optimiste) ; (b) la FRAÎCHEUR (âge de perception d'un lointain) jamais mesurée en direct ;
> (c) **« 55K » jamais mesuré DIRECTEMENT** — densité mesurée à 1000 (89 %), à 2000/5000 bridée par le bootstrap
> (mur n°2) → **ne JAMAIS dire « 55K prouvé » ni « converge à 5000 »**.
>
> ### 🎬 POUR LA CONCEPTION D'UNE DÉMO (ce qui est PROUVÉ vs ce qu'il ne faut PAS prétendre — maj 21 juin)
> **Démontrable, solide (à montrer sans risque) :**
> - **Jeu 3D P2P réel** : plusieurs fenêtres, avatars + pseudos + badge OWN, hole-punching NAT RÉEL (full-cone, namespaces), orbe autoritaire avec migration.
> - **Foule dense en 3D** : rendu à deux tiers (focus net + foule en imposteurs LOD), **>64 visibles sans lag** (confirmé écran à `tools/foule-3d.sh 80`).
> - **Tenue sous mauvais réseau** : 250 ms + jitter + 5 % perte + ré-ordonnancement → l'essaim tient (ch.7, `tools/sim-netem.sh`).
> - **Résistance aux attaques** (headless, `cargo run -- attack …`) : Sybil-framing ÉCHOUE, gossip-flood absorbé (0 réflexion), orbe 0 volée, tricheurs en sourdine.
> - **Échelle (headless, banc bus)** : perception d'une foule via résumés de cellule, **densité restaurée à 1000 (89 %) à débit reçu PLAT** ; coût/nœud ~0,27 Mbit/s ↑.
> - **Identité persistante** : même « compte » entre deux lancements (clé locale, comme une clé SSH).
> **À NE PAS prétendre dans la démo (honnêteté = crédibilité) :**
> - **PAS « 55 000 prouvé »** : mesuré directement jusqu'à ~1000-2000 ; au-delà = archi + extrapolation.
> - **PAS « converge à 5000 »** : à 5000 le bootstrap (mur n°2) est lent → la perception monte mais n'a pas fini dans nos fenêtres ; « débit plat » n'est confirmé que 1000↔2000.
> - **PAS « densité sécurisée prouvée »** : `CORROB` n'est prouvé qu'en *logique* + récupération headless ; le /24 anti-inflation réel attend le harnais NAT (C-sécu-2).
> - **PAS « vraiment sans serveur »** : l'amorçage passe encore par un rendez-vous (borné, démoté à l'amorçage — D10).
> - **PAS « confidentiel »** : positions en CLAIR pour l'instant (chiffrement = ch.10.2, pas fait).
>
> **✅ C-sécu-2 BOUCLÉ (22 juin, étapes 1→6).** Le plancher est désormais à la fois ANTI-INFLATION (red-team 50) ET
> de meilleure fidélité que l'ancien 1b gonflable (781 vs 746 @1000), à coût wire nul. Lag de convergence fermé.
>
> 🏦 **C-sécu est BANQUÉ — on n'en fait PLUS (décision 22 juin, avec l'utilisateur).** État connu, propre, on y
> reviendra. **On ne lance PAS C-sécu-3.** Le raisonnement (le sien, validé) : *continuer à dorer ce sous-système,
> alors que la moitié du script n'existe pas, est un double gâchis* — (a) on polit trop tôt, (b) on polit contre des
> hypothèses que la moitié non-écrite va DÉPLACER (la corroboration /24 suppose un modèle d'adresses que le NAT réel
> va changer ; le timing gossip est réglé sur un réseau parfait que la vraie perte/jitter va bouger). Donc tout poli
> d'aujourd'hui serait à RE-faire. Dette C-sécu laissée explicite : /24 anti-inflation jamais testé sur de vraies IP ;
> red-team 6-B prouvé en unitaire seulement ; mémoïser `floor_counts_by_cell` si le décodage pèse (CPU +6 % banc).
>
> 🟠 **PIÈCE À TÉMOIN — l'orbe est CASSÉE (et c'est l'argument fait chair).** Un sous-système magnifiquement poli
> (autorité unique, migration d'hôte déterministe, preuve de contact anti-vol, 12 tests headless verts) a
> SILENCIEUSEMENT cassé quand on a empilé scènes + île + gravité : l'orbe est restée clouée à `(0,1.5,0)` dans le
> repère MORT de la salle (`ORB_START` + boîte `ROOM_SIZE`/`ROOM_HEIGHT`, [orb.rs:31](src/net/orb.rs#L31),
> [orb.rs:304-332](src/net/orb.rs#L304-332)), pendant que le joueur a déménagé sur l'île (`IslandSpawn.pos`). Enterrée
> sous le terrain / sous l'eau / à 30 m → invisible. **Pas réparée exprès** : son game-feel part vers Unreal, la
> ré-ancrer dans Bevy serait du jetable. Témoin du « ce qu'on a poli devra RE-être poli », et du fait que les tests
> HEADLESS ne voient pas l'intégration visuelle. → couche présentation (étape 4).
>
> 🆕 **D27 — « la forteresse vide ».** Le doute le plus dangereux du projet, jusqu'ici ABSENT du registre (26 doutes
> techniques, 0 de périmètre) : *ai-je bâti une forteresse réseau magnifique dans laquelle DEUX humains réels ne se
> sont jamais retrouvés via le vrai Internet ?* Sa preuve : la mesure (b) « fraîcheur ressentie » — la SEULE grandeur
> qui décide « espace vivant vs mort » — est la SEULE jamais chiffrée, et le banc bus headless en est STRUCTURELLEMENT
> incapable (grandeur humaine sur lien réel avec perte/jitter). La cadence anti-collapse a donc un angle mort câblé :
> elle ne prouve QUE ce que le simulateur sait prouver. D27 ne se ferme que dehors. → c'est l'objet du PLAN ci-dessous.
>
> **PLAN D'ACTION — REPRISE (par ordre, décidé le 22 juin ; 55K = boussole, pas échéance) :**
> 1. **[LE squelette de bout en bout — la base, AU SENS LITTÉRAL] D17 : deux humains, deux vrais NAT, le vrai
>    Internet, qui se voient bouger. Laid accepté.** Aujourd'hui deux inconnus sur deux box ne peuvent PAS se
>    connecter (hole-punching parqué) → ce n'est pas du poli, c'est la PORTE D'ENTRÉE de la maison, et elle n'existe
>    pas. « Finir la base » = D17, pas C-sécu-3. Ce test attaque d'un coup : D17 NAT (marche ou mur, découvert cheap
>    maintenant), (b) fraîcheur en direct (ferme D27), saut d'orbe (R1) et jitter — les 4 vraies mauvaises expériences.
>    **On pré-enregistre AVANT le test** : ms de retard médian = « mort », taux de connexion par TYPE de NAT — et on
>    inclut de l'HOSTILE (lien mobile, deux NAT symétriques) sinon on ne fait que déplacer l'auto-félicitation.
> 2. **[présentation] SWITCH UNREAL via SIDECAR (B)** — décidé le 22 juin, cf. bloc 🎮 plus haut. APRÈS que la base
>    sache connecter 2 humains. Le cœur réseau Rust (intouchable) devient un démon ; Unreal = client mince sur socket
>    local. Étapes : extraire crate `web3core` → contrat d'interface minimal (poll avatars / push ma position / état
>    orbe) → rebâtir hub/portails/arcade/île/avatars (l'orbe y renaît, ré-ancrée).
>
> **PROCHAINE ACTION CONCRÈTE = PAPIER « test 2-humains » d'abord** (critère pré-enregistré, dispositif : 1 PC + 1
> téléphone en partage de connexion, ou un VPS à 5 €), AVANT la moindre ligne. *(Pas Unreal pour ça — l'orbe Bevy
> suffit pour VOIR le jitter et le saut.)*
>
> ⚡ **PIÈGE DE BANC À RETENIR (sinon on perd 1 h) :** le minage PoW coûte **~3 s à N=1000 sous `POW_BITS=8`** mais
> **~50 min au défaut 18 bits** (≈3 s/identité × 1000). TOUJOURS lancer les bancs `coopsim-bus N≥1000` avec `POW_BITS=8`
> en tête (orthogonal à la mesure perception/débit). Et écrire la sortie DIRECTEMENT dans un fichier (`> f.txt`), JAMAIS
> via `| grep > f` (grep bufferise par blocs → blackout des snapshots ; le binaire Rust, lui, line-buffer son stdout).
>
> 🚀 **DÉMARRAGE PROCHAINE SESSION — le PAPIER « RELAIS » (D17, papier d'abord, comme C-sécu) :**
> - **Le test 2-humains a TOURNÉ (22 juin).** Verdict mesuré : **bootstrap Internet ✅** (2 humains / 2 réseaux
>   se trouvent via le rendez-vous public) ; **perçage NAT direct ❌** des deux côtés → **mur D17 MESURÉ**. La
>   fraîcheur (b) / D27 n'a PAS pu être chiffrée : sans perçage, ils ne se voient pas bouger. → Le relais débloque.
> - **⚠ Confound à NE PAS blanchir :** B était derrière la MÊME box que le serveur (hairpin → `192.168.1.254`).
>   Donc seul l'échec de A (NAT symétrique mobile) est un VRAI mur ; l'échec de B est un artefact de topo.
>   → Le relais est cadré **REPLI** (minorité non-perçable), pas chemin nominal. *(détail : § PAPIER 12.3.)*
> - **▶️ Petits pas (cf. PAPIER 12.3 écrit le 22 juin) :** (0) UNE mesure propre B-sur-un-autre-réseau (~0 code) pour
>   dimensionner le repli ; (1) papier-wire (format demande de relais + seuil de fraîcheur) ; (2) repli minimal
>   derrière un flag (v1 = rendez-vous relaie une paire qui a abandonné — hook `punch_abandoned`), défaut intact,
>   0 warning ; (3) preuve A(mobile)↔B : ils se voient BOUGER + fraîcheur (b) chiffrée en ms.
> - **Critère pré-enregistré (figé dans le PAPIER 12.3 AVANT de coder) :** deux pairs au perçage abandonné des DEUX
>   côtés se voient néanmoins BOUGER via relais, ET on chiffre enfin la fraîcheur (b) en ms. Échec admis si pas de
>   mouvement relayé, ou fraîcheur cassant le game-feel.
> - **Acquis réutilisé :** le primitif relais existe et est durci (`mark_as_relay`/`KIND_RELAY` + bornes
>   `RELAY_RATE`/`RELAY_CAP`/`MAX_RELAY_FANOUT`). L'état relayé reste signé → sécu OK par construction.
>
> **APRÈS le relais :** la preuve réordonne le plan. Si (b) relayée tient → interpolation R1 (si le saut domine) ou
> switch Unreal. Si (b) trop dégradée → v2 relais décentralisé (D4). *Petit pas, preuve d'abord, dehors cette fois.*
>
> ──────────── JOURNAL DÉTAILLÉ ci-dessous (archive — relire au besoin via `grep`) ────────────

**Historique des chapitres (archive élaguée le 21 juin — détail intact ailleurs).**
- **Chapitres 6→9 (faits, ✅) :** ch.6 refonte béton ; ch.7 confrontation au réel (mauvais réseau,
  coût/nœud chiffré, NAT multi-joueurs prouvé sous namespaces) ; ch.8 « foule dense » Phase A (gossip,
  AoI deux tiers, résumés de cellule frais) ; ch.9 confiance dure (anti-Sybil réglable, quorum pondéré,
  anti-éclipse /24, réhabilitation). **État détaillé → §B (lieux) et §D (programme) ; audit D1→D26 → tête de §C.**
- **Journal autonome 20-21 juin (C-diag, C-sécu, banc bus, redesign 8.3★) :** vit dans `PLAN_AUTONOME.md`
  (JOURNAL + § PAPIER C-sécu-2). On ne le re-duplique plus ici (hygiène anti-collapse).
- **Passe de VALIDATION par simulation** (proposée par l'utilisateur : rejouer chaque attaque/simu et coller
  le chiffre avant le ch.10) : **à faire** — non encore tenue en tant que passe dédiée.
- **Le cœur dur de D9 (Sybil + éclipse + framing) est tenu** (détail §D, ch.9).

> ### 🧾 REGISTRE DE DETTES OUVERTES (lis-moi — l'antidote à l'enfermement)
> *Les choses qu'on SAIT incomplètes mais qu'on a laissées passer. Quand je coche « ✓ FAIT »,
> les limites se font oublier : ici elles ont le droit de pousser contre le plan. À vider au fil
> de l'eau. La réalité a toujours raison contre ce document.*
> - **⚖️ LES 3 MESURES « TROP GENTILLES » (honnêteté de méthode, inscrites le 20 juin — à ne pas
>   laisser se noyer ; aussi résumées dans l'⏱️ ÉTAT COURANT en tête de §0) :**
>   **(a) « Couverture » compte les pairs CONNUS, pas ENTENDUS.** On sait qu'un nœud existe, pas
>   qu'on reçoit ses paquets frais → optimiste. La vraie inclusivité (D3) = « est-ce que je reçois
>   À TEMPS ? ». *Parade : une variante « entendus » de la métrique.*
>   **(b) La FRAÎCHEUR n'est jamais mesurée en direct.** « perception ∝ N à débit plat » prouve
>   seulement *indirectement* que les résumés sont assez frais ; on n'a pas l'ÂGE moyen de perception
>   d'un lointain (avec vs sans résumés). Tant qu'on ne le chiffre pas, le « 1/N tué » reste un argument.
>   **(c) Le banc plafonne à ~1500 nœuds** (un thread OS/bot, cf. D25). « Perception ∝ N » est MESURÉ
>   à ~1000-2000, ARGUMENTÉ au-delà. Honnête tant qu'on n'écrit JAMAIS « 55K prouvé ». Le piège =
>   oublier l'astérisque.
>   *Aucune n'invalide un résultat ; ce sont les 3 endroits où, si on se ment un jour, ça commencera là.*
> - **D23 (ch.8.1b) — gossip-DoS : FERMÉ et prouvé**, MAIS ⚠️ **PANNEAU ATTENTION — NE PAS OUBLIER EN
>   PARTANT :** la réflexion n'est pas *impossible*, elle est *bornée*. Un attaquant qui **mine sans
>   cesse de NOUVELLES identités PoW** peut relancer une rafale de ~10 s à chaque fois → coût attaquant
>   LINÉAIRE (un PoW par rafale), plus une amplification. Tant que la PoW est un « jouet » (16 bits, cf.
>   **D6**), ce coût reste faible. **Deux verrous restent à poser** : (1) durcir/adapter la PoW (**9.1**) ;
>   (2) **corroboration multi-informateurs** d'une carte avant de percer (**8.8**, relie **D9**). *Autres
>   résidus assumés : multi-sources contourne partiellement le rate-limit ; NAT symétrique abandonné par
>   l'abandon de perçage (→ relais, D17) ; pas de plafond GLOBAL de perçage (jugé inutile pour l'instant).*
> - ~~**D24 — foule visible plafonnée à 64**~~ → **FERMÉ (8.2c, confirmé à l'écran le 19 juin)** : capture
>   utilisateur à `foule-3d.sh 80` = ~8 avatars détaillés (têtes + pseudos) + une foule d'imposteurs LOD,
>   bien plus que 64 visibles, sans lag de rendu. Le rendu à deux tiers marche.
> - ⚠️ **PANNEAU — l'orbe en foule dense (observé au 8.2c, NON corrigé, volontairement) :** en 3D à 80
>   fenêtres sur UN PC, la balle saute/revient (« avance par à-coups »). **Diagnostiqué = artefact mono-PC**
>   (80 GPU sur une machine → bas FPS → `dt` énorme → le dead-reckoning `pos += vel*dt` fait des pas géants
>   qui dépassent puis re-snappent ; + maître affamé → migrations en boucle). **Sur 80 machines réelles, ça
>   disparaît** → on NE corrige PAS (la simu ne reflète pas la réalité ici). **✓ CONFIRMÉ (19 juin) :
>   `foule-3d.sh 8` = orbe PARFAITEMENT fluide → l'artefact mono-PC est prouvé, doute levé.** **Deux vrais
>   résidus exposés malgré tout, à traiter au bon chapitre, PAS maintenant :**
>   **R1** l'orbe n'est pas interpolée (snap à chaque paquet → à lisser, comme les avatars ; petit correctif futur) ;
>   **R2** la migration peut split-brain si les listes de pairs diffèrent en grande foule → c'est **D11** (→ ch.11.2,
>   migration confirmée par quorum).
> - **Mesure (ch.8.2) — la couverture compte les pairs CONNUS, pas ENTENDUS** : un peu optimiste.
> - **Foule dense JAMAIS testée en vrai 3D** : seul le headless le prouve (2 joueurs OK en 3D, pas 200).
> - **Réglages gossip arbitraires** (`GOSSIP_PERIOD 0.5 s`, `FANOUT 4`, `MAX_CARDS 16`) : choisis au
>   jugé, pas dérivés. À calibrer si la convergence ou le coût l'exigent.
> - **Pas d'éviction de pairs** (`MAX_KNOWN` est un mur sans TTL) : sur longue session, la table se
>   remplit de morts et bloque l'apprentissage → D16 (ch.12).
> - **D26 couche 1 (résumé AUTHENTIFIÉ, codé le 20 juin) — 3 dettes assumées :**
>   **(1) `cell_host` est ESTIMÉ localement** (à partir de `peer_pos` connu) : si je ne connais pas
>   encore l'occupant au plus petit id, je calcule un mauvais hôte attendu et je **rejette un résumé
>   pourtant légitime** (faux négatif). C'est SÛR (consultatif → je perds juste ce résumé jusqu'à
>   apprendre l'occupant, aucune corruption), mais ça peut **affamer la perception en connaissance
>   partielle**. *Parade future : tolérer une fenêtre d'hôtes plausibles, ou corroborer (couche 2).*
>   **(2) Rate-limit par `SocketAddr`, pas par `host`** : robuste contre un spammeur d'une IP, mais un
>   attaquant multi-IP/relais dilue la limite. Borné par `MAX_CELLS` de toute façon. *Parade : clé par
>   identité d'hôte si besoin.*
>   **(3) La couche 1 n'EMPÊCHE PAS l'hôte LÉGITIME de mentir** sur SA cellule (sur/sous-compter) :
>   c'est la **couche 2** (corroboration multi-informateurs, couplée à D4) — le vrai mur, plus tard.
>   *(Le client réel `netcode/receive.rs` n'ingère pas encore les résumés — seuls les bots, pour la
>   sim d'échelle ; couche 1 durcit donc le chemin SIM, là où la preuve d'échelle se joue.)*
> - **DETTE DE HARNAIS (8.3d) — le banc plafonne à ~1200-1500 nœuds sur ce PC (12 cœurs).** `sim`/`crowd`
>   lance **un OS-thread par bot** : à ~1 %cœur/nœud, au-delà de ~1500 on sur-souscrit les 12 cœurs et la
>   simu (pas le protocole) étouffe. Conséquence : le « D22 = échelle 5000 » LITTÉRAL n'est pas prouvable
>   ici. *Parade future possible (si on doute du résultat) : simulateur léger à ordonnancement coopératif
>   (N bots par thread, une seule boucle d'événements) → 5000+ tiendrait sur un PC.* Jugé NON urgent : le
>   protocole est déjà démontré (budget d'émission capé → réception O(1) en N ; perception ∝ N jusqu'à 2000).

**Méthode de travail (rappel des préférences de l'utilisateur) :** parler **français**
uniquement ; débutant Linux → toujours donner les commandes complètes **avec `cd`** ;
**critique honnête d'ingénieur, jamais de flatterie** ; **toujours exprimer ses doutes** ;
on **écrit le plan avant de coder** (cette phase de plan est faite — on peut coder le
ch. 7) ; **petites étapes** (chacune compilée + testée + prouvée en headless/simu, puis
commitée et écrite dans ce doc / le README) ; **toujours sauver sur GitHub** à chaque
étape. La vérification se fait **sans GPU** via les bots/simu (le jeu 3D, c'est
l'utilisateur qui le lance). Avant tout gros run de simu : `tools/sim-cool.sh` pousse
les ventilos au max (PC tour ASUS — sinon BIOS Q-Fan « Full Speed »).

**Deux règles anti-enfermement (ajoutées le 19 juin).** (1) **Tout « ✓ FAIT » doit lister ce
qu'il NE fait PAS** (ses limites/dettes) — et toute dette va dans le 🧾 REGISTRE ci-dessus, pas
dans un coin de tête. (2) **Cette feuille de route est une HYPOTHÈSE, pas une Écriture** : si la
réalité (une mesure, le jeu réel, une attaque) la contredit, on change le plan — jamais une
rustine pour cocher une case. Le danger à surveiller : optimiser pour cocher des cases au lieu de
coller au réel. L'utilisateur tient ce garde-fou ; je dois aussi le tenir seul.

**Comment lancer / tester :** voir le `README.md` (section « Comment lancer ») et les
modes `rendezvous | a | b | bot <nom> | attack <type> | sim <bots> <attaquants> <s>`.

---

## A. Les principes directeurs (la boussole)

1. **Aucun serveur de jeu central.** Un rendez-vous d'annuaire est toléré, mais il ne
   doit jamais devenir l'arbitre du jeu ni un point de confiance aveugle. À terme,
   même lui doit se décentraliser.
2. **Contrôle maximal, tout fait main.** Une seule boîte noire assumée : la crypto
   (`ed25519-dalek`). Tout le reste, on le comprend octet par octet.
3. **Répartition de la puissance = pilier.** Les forts aident les faibles (système de
   parent/relais). Personne n'a de pouvoir total (moindre privilège).
4. **Inclusivité radicale.** N'importe qui doit pouvoir jouer : de la pire connexion
   (quelques Ko/s, voire 0 via un parent) à la fibre 2 Gb/s. Chacun la meilleure
   expérience POUR SON lien.
5. **Honnêteté.** On préfère un MVP qui marche et des limites écrites, à une fausse
   promesse. Tout doute est un travail à planifier, pas à cacher.
6. **Plus tard, pas maintenant :** portabilité Unreal/Unity. On finit le cœur d'abord.
7. **Composer avec le RÉEL, pas inventer des stats (élégance).** *(Dégagé en H2, 20 juin 2026.)*
   Un nombre magique qui encode un **fait du monde** (rayon de l'orbe, vitesse humaine max) → on le
   garde. Un nombre qui **remplace une mesure qu'on pourrait prendre** (taille du focus, choix du
   parent, débit de relais) → on le **dérive de l'observation** (mesure passive, infalsifiable),
   jamais d'un rôle **auto-déclaré** ni d'un cap **universel**. On vise l'élégance (la meilleure
   technique), pas « ça tient ». *Discriminateur :* « ce nombre est-il un fait, ou une devinette qui
   tient lieu de mesure ? ». Unifie D3 (lien faible), D4 (relais) et le plafond de focus (`MAX_NEIGHBORS`).
   → **Passe d'élégance** : à chaque chapitre, repasser les constantes au crible « fait vs devinette ».
   ⚠ **Piège inverse (leçon de l'audit du 20 juin) :** ne PAS « mesurer » les constantes de RESSENTI
   (`INTERP_DELAY`, `SMOOTH_TIME`, `MAX_WARP`…) — elles encodent la perception humaine / le standard netcode,
   pas une mesure réseau ; les rendre dynamiques = sur-ingénierie. Un sous-agent a classé 83/124 constantes en
   « devinettes » : FAUX. Le vrai lot à dériver du réel ≈ **6** — `SEND_BUDGET_HZ`, `MAX_NEIGHBORS`, `K_FOCUS`,
   `MAX_FOCUS_DETAIL`/`MAX_AWARE`, `RELAY_RATE`/`MAX_RELAY_FANOUT` — tous des tailles de **focus/budget/relais**,
   donc **le même fix que D4** (dériver de la capacité mesurée).

---

## B. État des lieux (ce qui est FAIT)

Chapitres **0 → 5** : bac à sable 3D, transport UDP brut, netcode (interpolation /
prédiction / réconciliation), topologie & AoI (water-filling), autorité & migration
d'hôte (l'orbe), confiance & anti-triche (Ed25519, anti-rejeu, réputation locale,
rate-limit). Détail dans le `README`.

Chapitre **6 — refonte BÉTON** : **terminé**, les 10 trous de l'audit fermés/bornés.

| Sous-étape | Ce qu'elle ferme |
|---|---|
| 6.0 Bot headless + 4 attaques « rouges » | Le filet de vérification (sans GPU) |
| 6.1 Identité auto-certifiante (id = clé) | Plafond 255, collisions, **rendez-vous menteur** |
| 6.2 Preuve de travail anti-Sybil | Identité gratuite |
| 6.3 Validation de mouvement | Téléport / speed-hack |
| 6.4 Preuve de contact de l'orbe | Vol d'orbe par incréments |
| 6.5 DoS borné (éviction + relais plafonné) | Mémoire saturée, amplification réfléchie |
| 6.6 Voisinage borné (`MAX_NEIGHBORS`) | WELCOME tronqué, maillage O(N²) |
| 6.7 Réputation partagée (accusations + quorum) | Tricheur invisible pour qui ne l'a pas vu |
| 6.8 Simulation massive + essaim | **Preuve** que ça tient (50 & 300 nœuds) |

**Outils disponibles** : `cargo run -- rendezvous | a | b | bot <nom> | attack <type> |
sim <bots> <attaquants> <s>`, plus `tools/sim-cool.sh` (ventilos), `tools/test-nat.sh`
(NAT en namespaces) et `tools/sim-netem.sh` (mauvaise connexion via `tc netem`, ch.7.1).
**35 tests unitaires, 0 warning.** Le jeu 3D réel fonctionne
(vérifié à l'écran : avatars + pseudos `0000…` + badge OWN BALLE).

> **Ce que B prouve vraiment, et ne prouve pas.** B prouve la *correction* et la
> *résistance aux attaques connues*, sur localhost. B ne prouve **rien** sur le
> comportement en *vraie* condition réseau (latence, perte, NAT) ni sur l'*inclusivité*
> des liens faibles. C'est le point de départ de tout ce qui suit.

---

## C. L'inventaire des doutes (le cœur du document)

Chaque doute a : **Constat** (ce qui ne va pas / ce dont je ne suis pas sûr),
**Pourquoi ça compte**, **Gravité** (🔴 critique / 🟠 important / 🟡 à surveiller),
**Piste de correction**, **Comment on le vérifiera**. Le chapitre qui le ferme est
indiqué entre crochets `[ch. X]`.

> ### 📊 AUDIT DES DOUTES — état honnête au 21 juin (ce tableau fait FOI ; l'analyse détaillée suit)
> *Statut : ✅ fermé & prouvé · 🟡 borné/partiel (vit, à finir) · 🔴 ouvert (chapitre dédié). **9 fermés**, **11
> bornés**, **6 ouverts** (sur 26 ; maj : D16 TTL FAIT → borné ; D21 rate-limit FAIT ; D25 banc bus BÂTI ; D26 en
> refonte 8.3★). Les ouverts se regroupent PROPREMENT en chantiers, ce qui confirme le plan (voir sous le tableau).*
>
> | # | Statut | Où en est-on / qui le ferme |
> |---|---|---|
> | **D1** tests = localhost | ✅ | ch.7 : `tc netem` + NAT réel en namespaces (7.1→7.5) |
> | **D2** bot ≠ jeu | 🟡 | décisions de confiance partagées ; orchestration encore dupliquée → ch.12.2 |
> | **D3** lien faible noyé (réception) | 🔴 | inclusivité, **ch.8 Phase B** (8.4/8.6) |
> | **D4 + D5** parent (free-riding + censure, fusionnés) | 🟡 | DÉFRICHÉ en H2 → « **parent par MESURE du réel** » (capacité observée infalsifiable + affectation locale + dégradation équitable ; réputation SEULEMENT vs tricheurs). À CODER en **Phase B** |
> | **D6** PoW « jouet » | 🟡 | 9.1a : PoW **réglable**, socle 18 (mesuré) ; adaptative → annexe H |
> | **D7** framing par quorum | ✅ | 9.2 (crédibilité) + 9.4b (diversité IP) — prouvé par `attack sybil-frame` |
> | **D8** pas de réhabilitation | ✅ | 9.3 : fautes à décroissance temporelle |
> | **D9** position non vérifiée / éclipse | ✅ | 9.4a (positions corroborées) + 9.4b (cap /24) ; résidu botnet = limite P2P |
> | **D10** rendez-vous point unique | 🟡 | 9.5a (borné + PoW entrée) ; fédération + DHT → annexe H |
> | **D11** migration de l'orbe | 🔴 | autorité, **ch.11.2** (quorum) |
> | **D12** tout codé pour 1 objet | 🔴 | autorité, **ch.11.1** (registre générique) |
> | **D13** pas d'horloge commune | 🔴 | autorité, **ch.11.3** |
> | **D14** identité non persistante | ✅ | **10.1 FAIT** : clé sauvée `~/.web3game/<profil>.key`, rechargée au lancement (prouvé) |
> | **D15** tout en clair (vie privée) | 🔴 | **ch.10.2** (chiffrement X25519) |
> | **D16** fuites mémoire long terme | 🟡 | **T1.1 FAIT** : TTL/éviction des pairs (un mort cède son slot, jamais un actif) ; DHT/fédération → annexe H |
> | **D17** NAT symétrique | 🔴 | longévité, **ch.12.3** (relais/IPv6) |
> | **D18** speed-hack grossier | 🟡 | à surveiller, **ch.11.4** |
> | **D19** coût réel jamais mesuré | ✅ | 7.4/7.4b : Ko/s ↑↓, %CPU, RAM par nœud |
> | **D20** attaques combinées | 🟡 | `sim` lance plusieurs attaquants en // ; pas encore de scénario coordonné adaptatif |
> | **D21** sécurité du rendez-vous | 🟡 | 9.5a (cap mémoire + PoW) + **T1.2 FAIT** (rate-limit débit par source) ; reste l'anti-spoofing (handshake de routabilité) |
> | **D22** foule dense (aveugle > 32) | ✅ | 8.1 (plafond cassé) + 8.2 (deux tiers) + 8.2c (rendu) + 8.3d (résumés frais). **8.3★ (banc bus) :** densité restaurée à 1000 (89 %) à débit PLAT ; à 5000 bridée par la DÉCOUVERTE (mur n°2), pas par le protocole |
> | **D23** gossip = ampli DDoS | ✅ | 8.1b : prouvé par `attack gossip-flood` |
> | **D24** foule visible plafonnée 64 | ✅ | 8.2c : rendu deux tiers, confirmé à l'écran |
> | **D25** banc d'essai ; « 55k » non mesuré DIRECTEMENT | 🟡 | **banc BUS mémoire BÂTI** (dt fixe, découplé du mural) → mesure jusqu'à 5000+ ; reste : 55k non direct + mur n°2 (bootstrap) limite la convergence à 5000 |
> | **D26** agrégateur/résumé MENTEUR | 🟡 | **REDESIGN 8.3★ : on RETIRE le chef de cellule** (l'élection était le mur n°1, mesuré). C-diag : taxe dissoute, densité restaurée. **C-sécu** (en cours) : densité molle CORROBORÉE /24 (`qth_largest`) → couches 1+2 fusionnées par construction |
>
> **Les doutes 🔴 ouverts se rangent en chantiers — ça VALIDE le plan :**
> - **Inclusivité** (D3, **D4**, D5 + **D26**) → **fin du ch.8, Phase B** *(prochain gros cap)*
> - **Vie privée & identité** (D14, D15) → **ch.10**
> - **Autorité & ordre** (D11, D12, D13) → **ch.11**
> - **Longévité** (D16, D17) → **ch.12**
> - **Méta / banc** (D25) → dette de harnais (simulateur léger pour 5000+)
> Plus D22 fermé au 8.3d. **Donc : finir le ch.8 (Phase B) puis le ch.10 ferme à lui seul 6 doutes — les plus
> proches de la vision.** Le reste (ch.11/12) est du durcissement « confort », pas un bloqueur de la promesse P2P.
>
> **⭐ LES DEUX DOUTES LES PLUS DURS QUI RESTENT (nommés le 20 juin — à ne PAS laisser se noyer dans la liste) :**
> - **D4 — l'INCITATION → RETOURNÉE en problème de MESURE (défrichée en H2, 20 juin).** On a renversé le
>   cadrage : ce n'est PAS une économie (récompenser le relais) mais une **OPTIMISATION** — quand la capacité
>   est abondante, relayer coûte du bruit ; il suffit de **mesurer la capacité réelle (observée, infalsifiable)
>   et d'affecter le meilleur parent**. Plus de monnaie de réputation ; la réputation reste *seulement* contre
>   les tricheurs. D4 + D5 fusionnés en « parent par mesure du réel ». Reste à CODER + PROUVER en Phase B.
>   Conditionne D3 + D17.
> - **D10 — le rendez-vous reste la DERNIÈRE centralisation.** Le « sans serveur » garde un astérisque : l'amorçage
>   passe par un rendez-vous (le gossip réduit la dépendance, la fédération/DHT est en annexe H). Pour un « VRAIMENT
>   sans serveur », c'est le dernier nœud de confiance à décentraliser. Borné aujourd'hui, pas supprimé.

### Catégorie 1 — Le réalisme de nos tests

**D1 — Nos tests « mentent » comme localhost.** 🔴 `[ch. 7]`
*Constat :* toute la simu tourne sur une machine, sans latence, sans perte, sans NAT,
sans jitter. *Pourquoi :* la règle d'or du projet est « jamais ‘ça marche sur
localhost' » ; un netcode peut être parfait sur `lo` et s'écrouler à 150 ms + 2 % de
perte (la prédiction part en vrille, l'anti-rejeu rejette des paquets re-ordonnés,
les migrations se déclenchent à tort). *Piste :* injecter des conditions réseau
réelles avec `tc netem` sur la boucle locale, et du NAT avec `ip netns` — **une seule
machine suffit** (voir section E). *Vérif :* refaire tourner `sim` à travers netem et
mesurer que l'essaim tient (orbe intègre, débit honnête stable, pas de fausses
migrations) à 100/150/250 ms et 1/3/5 % de perte.

**D2 — Le bot de sim ≠ exactement le jeu.** 🟠 `[ch. 12]` *(risque CONFIRMÉ réel au 7.4b)*
*Constat :* `bot.rs` réécrit la boucle de réception de `receive.rs` ET d'émission de
`send.rs` (les *décisions* de confiance sont partagées, mais l'*orchestration* est
dupliquée). *Pourquoi :* un correctif dans l'un peut ne pas atteindre l'autre → divergence
silencieuse à long terme. **⚠ Preuve que le risque est RÉEL (7.4b) :** le bot émettait à
plein débit à tous, alors que le jeu répartit par AoI water-filling → on a mesuré un coût
faux (89 au lieu de 34 Ko/s) jusqu'à ce qu'on le remarque. Corrigé en faisant appeler au bot
les mêmes fonctions qu'[aoi.rs] — MAIS l'orchestration reste dupliquée (le risque demeure).
*Piste :* extraire un cœur de session commun (un seul `Bot`/`Session` que le jeu Bevy ET le
bot pilotent). *Vérif :* le jeu et le bot partagent le même module de boucle ; un test prouve
qu'ils traitent un paquet donné identiquement.

### Catégorie 2 — L'inclusivité (le cœur de la vision)

**D3 — Un lien faible ne peut PAS suivre, et ne peut pas dire « envoyez-moi moins ».** 🔴 `[ch. 8]`
*Constat :* dans une foule dense, un joueur **reçoit ~43 Ko/s** (jusqu'à 32 voisins qui
lui émettent ; l'AoI borne ce que TOI tu émets, **pas ce que tu reçois**). Un joueur à
quelques Ko/s est noyé. *Pourquoi :* c'est l'inverse exact de l'objectif « n'importe
qui peut jouer ». *Piste :* (a) un **contrôle de débit côté receveur** — chaque joueur
annonce un « budget de réception » (Ko/s) et un rayon d'intérêt ; les émetteurs en
tiennent compte (le water-filling devient bilatéral). (b) Pour les très faibles : le
**parent agrège et dégrade** — il reçoit les 32 voisins et n'envoie au protégé qu'un
résumé basse fréquence (positions échantillonnées, foule en LOD). *Vérif :* sous netem
throttlé à 5 Ko/s, le joueur reste fonctionnel (voit ses voisins immédiats, foule
lointaine en basse fidélité) et ne perd pas la connexion.

**D4 + D5 — Le parent, par MESURE du réel (fusionnés ; direction DÉFRICHÉE en H2, 20 juin).** 🟡 `[Phase B]`
*Constat (D4) :* aujourd'hui le rôle faible est **auto-déclaré** (`cargo run -- weak` → `main.rs:126`)
et le parent est choisi par **le plus petit id** (`send.rs:123`) — aucun rapport avec la réalité du
lien. Sans rien, tout le monde se déclare faible et personne ne relaie. *Constat (D5) :* un parent
qui *jette* tes paquets (sans les falsifier — la signature l'en empêche) te rend **invisible en silence**.
**Reframe décisif (H2) : ce n'est PAS une économie (récompenser le relais), c'est une OPTIMISATION.**
Quand la capacité est abondante, relayer coûte du **bruit** (un fibre relaie 3 mobiles pour
~0,04 Mbit/s, cf. 7.4b) → pas besoin d'incitation, juste d'une bonne **affectation**. *Piste retenue :*
- **(1) Capacité OBSERVÉE, pas déclarée.** Chaque nœud mesure *passivement* le débit/fraîcheur qu'il
  reçoit RÉELLEMENT de chaque pair → estime sa capacité. **Infalsifiable** (on ne simule pas des
  paquets que les voisins n'ont pas reçus) ; **pas de speed-test actif** (coûteux, vecteur d'abus).
- **(2) Affectation LOCALE.** Depuis sa cellule, chacun choisit parent = `argmax(capacité dispo ×
  proximité ÷ charge)`, recalculé en continu → **bon compromis local**, pas un optimum global prouvé.
- **(3) Pénurie réelle.** Aucun algo ne crée de bande passante : on **dégrade ÉQUITABLEMENT** (tout
  le monde baisse de LOD ensemble) au lieu d'exclure quelqu'un (minimax, principe 4).
- **(4) Réputation SEULEMENT contre les tricheurs** (principe 7) : celui qui **truque sa mesure**, et
  le **relais trou-noir** (= D5 : si mes voisins ne confirment jamais recevoir mon état via lui → il
  ment → je change de parent ; redondance multi-parents en secours).
*Rejeté explicitement :* le **tit-for-tat symétrique** façon BitTorrent (le faible n'a aucun upload à
rendre → ça l'EXCLUT), et toute **monnaie de réputation positive** (elle rouvrirait un Sybil de
*gonflage* de score que le ch.9 n'a PAS fermé — il n'a fermé que le framing négatif).
*Doute restant à PROUVER (sim, plus tard) :* la mesure passive est-elle vraiment **infalsifiable ET
assez rapide** pour suivre un mobile qui bouge ? *Vérif :* nœuds égoïstes → service dégradé ;
coopératifs → bon service ; **faibles-mais-honnêtes (témoignent sans relayer) NON exclus** ; relais
trou-noir contourné en N s. *(Remplace l'inélégance actuelle : `weak` auto-déclaré + parent = plus petit id.)*

### Catégorie 3 — Sybil & réputation

**D6 — La preuve de travail (16 bits ≈ 1 s) est un jouet.** 🔴 `[ch. 9]`
*Constat :* miner une identité coûte ~1 s ; un attaquant en fabrique des centaines en
minutes. *Pourquoi :* toute la réputation/quorum repose sur « une identité coûte cher »
— or ce n'est pas le cas. *Piste :* difficulté **beaucoup** plus haute (réglable, +
adaptative à la charge), et/ou un second facteur (coût social : il faut être *vouché*
par des pairs déjà réputés pour peser dans un quorum). *Vérif :* le temps/coût pour
fabriquer un quorum d'identités devient prohibitif (minutes→heures), mesuré.

**D7 — Le quorum d'accusation (=3) permet le *framing*.** 🔴 `[ch. 9]`
*Constat :* 3 identités minées (~3 s avec D6 non corrigé) suffisent à faire bannir un
innocent par quorum. *Pourquoi :* l'anti-framing du 6.7 n'est solide que si fabriquer 3
accusateurs distincts est cher. *Piste :* lier au D6 (identités chères) **et** pondérer
les accusations par la réputation de l'accusateur (un inconnu pèse peu), **et** exiger
que les accusateurs soient eux-mêmes des voisins plausibles de l'accusé (on n'accuse
crédiblement que ce qu'on a pu observer). *Vérif :* en simu, K attaquants ne peuvent
PAS faire bannir un nœud honnête, même en se coordonnant.

**D8 — Aucune réhabilitation, aucune expiration.** 🟠 `[ch. 9]`
*Constat :* une fois muet (par fautes ou quorum), c'est définitif ; les compteurs de
fautes/accusations ne s'effacent jamais. *Pourquoi :* injuste (un faux positif est une
condamnation à vie) et ça fuit en mémoire (cf. D17). *Piste :* fenêtre glissante (les
fautes anciennes s'estompent), et procédure d'appel/quarantaine plutôt que ban sec.
*Vérif :* un nœud injustement muté redevient audible après une période de bon
comportement ; les compteurs décroissent.

### Catégorie 4 — La confiance topologique

**D9 — La position n'est pas vérifiée → attaque par éclipse.** 🔴 `[ch. 9]`
*Constat :* un nœud peut **mentir sur sa position** pour se déclarer voisin le plus
proche de tout le monde, s'insérer dans chaque voisinage, t'isoler (eclipse) ou tout
surveiller. Le choix des voisins fait confiance au rendez-vous + à des positions
auto-déclarées. *Pourquoi :* qui contrôle tes voisins contrôle ta réalité (il peut te
cacher des joueurs, t'en inventer, te couper). *Piste :* diversité forcée du voisinage
(ne pas prendre tous ses voisins de la même source/sous-réseau ; mélanger « plus
proches » et « aléatoires vérifiés », façon table de routage Kademlia), et corroborer
la position d'un pair par ce que d'autres en rapportent. *Vérif :* un attaquant qui ment
sur sa position ne peut pas occuper plus d'une fraction bornée d'un voisinage.

**D10 — Le rendez-vous reste un point unique (panne, centralisation, vie privée, DoS).** 🔴 `[ch. 9]`
*Constat :* s'il meurt → plus de découverte ni de détection de vivacité ; il voit
**toutes** les IP + positions (vie privée) ; il choisit tes voisins (cf. D9) ; il n'a
**aucun rate-limit** (on peut l'inonder de HELLO valides). *Pourquoi :* c'est l'astérisque
géant sous « pas de serveur ». *Piste :* fédération (plusieurs rendez-vous qui
s'échangent des pairs), puis découverte par **DHT / gossip** (les pairs s'entre-présentent),
le rendez-vous ne servant qu'à l'amorçage. + rate-limit + PoW à l'entrée du rendez-vous.
*Vérif :* tuer le rendez-vous en cours de partie ne casse pas une session déjà établie ;
deux rendez-vous se partagent les joueurs.

### Catégorie 5 — L'autorité des objets

**D11 — La migration de l'orbe est le point mou (ta question « comment corriger ? »).** 🟠 `[ch. 11]`
*Constat :* quand le maître se tait (> `MASTER_TIMEOUT`), un revendiqueur **inconnu** est
accepté **sans** preuve de contact (sinon on bloquerait une vraie reprise). Un attaquant
patient attend ce délai puis prend l'orbe. *Pourquoi :* c'est la seule porte laissée
ouverte sur l'orbe. *Comment corriger, concrètement :*
  1. **Élection confirmée par quorum** : la reprise n'est valide que si un quorum de
     voisins (a) confirme que l'ancien maître est bien silencieux ET (b) est d'accord
     sur le gagnant déterministe (le plus petit id parmi les *présents confirmés*). Un
     attaquant seul ne fait pas quorum → il ne peut pas voler pendant la panne.
  2. **Le gagnant doit prouver sa présence récente** (un état signé récent dans la zone),
     pas forcément le contact, mais au moins « j'existe et je suis là, maintenant ».
  3. **Période de grâce** : pendant la fenêtre de migration, on n'accepte AUCUN saut de
     version > +1, et on log/quarantaine tout candidat multiple (signe de course/triche).
*Vérif :* une attaque « j'attends le timeout puis je prends » échoue tant qu'un quorum de
voisins honnêtes est présent.

**D12 — Tout est codé pour UN objet (l'orbe).** 🟠 `[ch. 11]`
*Constat :* le vrai jeu aura des milliers d'objets partagés (portes, scores, projectiles).
L'autorité par objet + migration + preuve de contact doit se **généraliser** et passer
à l'échelle (des milliers d'autorités à bas débit). *Piste :* un registre générique
d'objets autoritaires (id d'objet, maître, version, règle de contact paramétrable),
réutilisant la machinerie de l'orbe. *Vérif :* 1000 objets partagés simulés tiennent,
chacun avec son maître, sans exploser le débit.

**D13 — Pas d'horloge commune → conflits mal arbitrés.** 🟡 `[ch. 11]`
*Constat :* deux joueurs qui touchent un objet « en même temps » : qui a raison ? On
départage par (version, id), pas par le temps réel. Les `now` locaux diffèrent. *Pourquoi :*
pour des règles de jeu équitables (qui a marqué en premier ?), il faut un ordre temporel.
*Piste :* horodatage signé dans les paquets + synchro d'horloge légère entre pairs
(estimation d'offset, façon NTP simplifié) ; **jamais** de consensus lourd sur le chemin
temps réel. *Vérif :* une course à l'objet est tranchée de façon cohérente par tous.

### Catégorie 6 — Identité & vie privée

**D14 — L'identité n'est PAS persistante.** ✅ `[10.1 FAIT, 20 juin]`
*Constat (était) :* `NetLink::new` minait une identité neuve à chaque lancement → pas de « compte »,
réputation qui ne s'accumule pas, amis qui ne te reconnaissent pas. *Résolu (10.1) :* la clé est minée
UNE fois puis sauvée dans `~/.web3game/<profil>.key` (perms 600, comme `~/.ssh/id_ed25519`), rechargée
au lancement (`crypto::load_or_create_in` + `from_secret`/`secret` ; `NetLink::new_persistent` côté jeu,
`new` éphémère intact côté simu). Profil = le mode → `a.key` ≠ `b.key` (deux fenêtres distinctes ET
stables). *Prouvé (test) :* 2e lancement RECHARGE la même identité, profils distincts = identités
distinctes. *Reste (10.2) :* chiffrer le fichier par mot de passe (aujourd'hui en clair, passphrase plus tard).

**D15 — Tout est en clair (aucune confidentialité).** 🟠 `[ch. 10]`
*Constat :* positions, et bientôt la **voix**, circulent en clair ; le rendez-vous et tout
nœud sur le chemin voient où tu es et avec qui. La signature prouve l'authenticité, pas
le secret. *Pourquoi :* pour de vrais utilisateurs, savoir « qui est où » en clair est un
trou de vie privée (harcèlement, traçage). *Piste :* chiffrement de transport par paire
via un échange de clés **X25519** (Diffie-Hellman sur courbe, cousin d'Ed25519) → un
secret partagé par paire, le contenu chiffré, la signature conservée pour l'intégrité.
*Vérif :* un observateur tiers ne peut plus lire positions/voix entre deux pairs.

### Catégorie 7 — Robustesse & longévité

**D16 — Fuites mémoire à long terme.** 🟠 `[ch. 12]`
*Constat :* `last_seq`, `strikes`, `accusations`, `last_state`, `holes` grossissent avec
chaque pair jamais croisé ; on n'évince jamais les partis (on a borné les *seaux* au
6.5, pas ceux-là). Sur une longue session avec du va-et-vient → la mémoire enfle.
*Piste :* TTL / éviction des pairs absents depuis longtemps (lié à D8 pour la réputation).
*Vérif :* sur une simu longue avec fort renouvellement, la mémoire se stabilise.

**D17 — NAT symétrique = repli forcé sur relais (et donc sur D3/D4).** 🟠 `[ch. 8/12]`
*Constat :* le hole punching échoue sur NAT symétrique (CGNAT mobile). Une fraction
réelle des joueurs ne pourra pas se connecter en direct → relais obligatoire → toute la
promesse P2P dépend de l'économie du parent (D4). *Piste :* relais TURN **décentralisés**
(chaque bon nœud peut en héberger un), priorité IPv6 (plus de NAT du tout). *Vérif :* le
cas symétrique de `tools/test-nat.sh` réussit via un relais.

**D18 — Le seuil de speed-hack (30 m/s) est grossier.** 🟡 `[ch. 11/12]`
*Constat :* un tricheur subtil reste juste sous 30 m/s (plus rapide qu'un honnête, mais
non détecté) ; pas de borne d'accélération ni de téléport-sur-longue-absence. *Piste :*
bornes plus fines (vitesse + accélération + cohérence avec la vitesse annoncée `vx,vy,vz`),
calibrées sur la vraie vitesse du jeu. *Vérif :* un « speed-hack discret » à 29 m/s est
repéré comme statistiquement anormal.

### Catégorie 8 — Méta-doutes (sur la démarche)

**D19 — On n'a jamais mesuré le coût RÉEL par nœud (CPU, RAM, bande passante).** ✅ `[7.4 + 7.4b FAITS]`
*Constat (était) :* la simu disait « ça tient » sans chiffrer Ko/s ↑↓, % CPU, Mo RAM par nœud.
*Résolu (7.4) :* `sim` mesure désormais, **par nœud** sur la fenêtre de test, la bande
passante réelle (compteurs d'octets dans la prise) et le temps CPU réel du thread
(`/proc/thread-self/stat`) ; la RAM est donnée **globale** (crête `VmHWM` du process), car
un seul tas est partagé entre threads → on REFUSE d'inventer une RAM par nœud factice.
*Correctif de fidélité (7.4b) :* le 1er chiffre de 7.4 (**↑ ~89 Ko/s**) était mesuré sur le
**mauvais chemin d'envoi** — le bot émettait naïvement à plein débit à tous ses voisins, alors
que le **vrai client** ([netcode/send.rs]) répartit un budget fini par AoI water-filling. Le
bot ([bot.rs]) appelle DÉSORMAIS les mêmes fonctions ([aoi.rs] : `relevance_weight` +
`allocate_rates`). Re-mesuré à SATURATION (50 nœuds co-localisés, plafond 32, PC tour ASUS) :
**↑ ~34 Ko/s/nœud (max ~38), ↓ ~31 Ko/s, CPU ~0,7 %/cœur, RAM crête 38 Mo.** L'écart 89→34
(≈ budget AoI 240 ÷ 640 envois naïfs = 0,38) confirme que le 89 était l'artefact du bot naïf.
*Extrapolation 55k (honnête) :* bornés par le voisinage (~32), PAS par le total (6.6) → ne
bougent PAS à 55k ; l'échelle se fait en AJOUTANT des machines. Un nœud demande **~0,27 Mbit/s ↑**
(≈0,4 avec en-têtes IP/UDP non comptés) — très tenable sur une connexion domestique. *Réserves :*
(1) sur `localhost`, le CPU ne compte PAS le coût réseau réel (NIC, RTT) → plancher honnête ;
(2) compteur = charge utile UDP, le fil réel ajoute ~28 o/paquet ; (3) **CE « constant à 55k »
suppose qu'il SUFFIT de voir ~32 voisins — faux en foule dense : voir D22.**

**D22 — La foule DENSE n'est pas résolue : au-delà de 32 voisins, on est AVEUGLE.** 🔴 `[ch. 8 — la foule dense]`
*Constat (révélé au 7.4b) :* le « coût constant à 55k » est acheté par un **plafond dur** au
rendez-vous (`keep_nearest(…, 32)`, [rendezvous.rs]) : il ne présente que les 32 plus proches,
« au-delà, on n'existe pas pour vous » ([aoi.rs]). Comme les bots de `sim` sont co-localisés
(rayon 3 m), `sim 50` est une **foule de 50 au même point** → mesuré : chacun n'en voit que ~32,
**aveugle aux ~17 autres**. À un rassemblement de 500, on serait aveugle à ~468. *Pourquoi
c'est grave :* dans un monde social type VRChat, VOIR la foule est le cœur du jeu. Et le joli
water-filling d'[aoi.rs] **ne peut pas aider** : il répartit le budget entre les 32 connus, mais
n'apprend JAMAIS l'existence du 33e (découverte plafonnée en amont). *Pourquoi c'est une
question d'ARCHITECTURE, pas de réglage :* (a) le rendez-vous est central (à supprimer pour du
P2P pur) ET plafonne la vision ; (b) la vraie réponse = AoI **par vision** (périmètre par joueur,
foule lointaine en LOD/imposteur) + découverte décentralisée (gossip/DHT) + relais/parent pour
la densité. *Piste :* un **chapitre densité dédié** (pas un patch) : scénario de foule dense
dans `sim`, découverte sans plafond dur, dégradation gracieuse perceptuelle. *Vérif :* à 200+
nœuds co-localisés, un joueur perçoit une foule cohérente (proches nets, lointains dégradés)
sans exploser son débit, et SANS plafond dur arbitraire.

**D20 — Attaques combinées / adaptatives jamais testées.** 🟠 `[ch. 9]`
*Constat :* nos attaques sont jouées isolément. Un vrai adversaire combine (Sybil +
éclipse + framing), s'adapte, joue honnête puis trahit. *Piste :* un mode d'attaque
« scénario » qui enchaîne et coordonne plusieurs vecteurs. *Vérif :* l'essaim tient face
à un scénario combiné, pas juste face à une attaque pure.

**D21 — La sécurité du rendez-vous lui-même.** 🟠 `[ch. 9]`
*Constat :* le rendez-vous accepte tout HELLO valide sans rate-limit ; sa table `clients`
grossit sans borne ; il n'a pas la machinerie anti-DoS des clients. *Piste :* lui donner
rate-limit + éviction + PoW à l'entrée (déjà le `has_pow`, mais pas le débit). *Vérif :*
inonder le rendez-vous ne le met pas à genoux.

**D25 — Le banc d'essai PLAFONNE (~1500 nœuds) ; « 55 000 » n'est pas mesuré DIRECTEMENT.** 🟡 `[dette de harnais]`
*Constat (nommé le 20 juin, au 8.3d) :* `sim`/`crowd` lance **un OS-thread par bot**. La mesure dit ~**1 %
d'un cœur par nœud** ; sur cette machine (**12 cœurs**, `nproc`), au-delà de ~1200-1500 nœuds on sur-souscrit
les cœurs → les threads s'affament → la SIMU étouffe (débit et couverture chutent) — **artefact du banc, pas du
protocole** (confirmé par l'arithmétique : 2000 × 1 % = ~20 cœurs demandés sur 12). *Pourquoi ça compte :* la
promesse « 55k sans serveur » repose donc sur **l'argument d'architecture** (budget d'émission capé → réception
bornée, indépendante de N — prouvé jusqu'à 1000) **+ une extrapolation**, PAS sur une mesure directe à 55k. C'est
honnête, mais c'est un *trou de preuve*, pas un trou de défense. *Piste :* un **simulateur léger** à ordonnancement
coopératif (N bots par thread, une seule boucle d'événements, pas de `thread::sleep` par bot) → 5000-50000 nœuds
tiendraient sur un PC. *Vérif :* à 5000+ nœuds simulés léger, le débit ↓ reste plat et la perception suit N.
*(Jugé NON urgent : le protocole est déjà démontré ; on bâtira ce simulateur si on DOUTE du résultat ou avant un
vrai déploiement de masse.)*

### Catégorie 9 — Doutes nés du chapitre 8 (la rançon honnête du gossip)

**D23 — Le gossip est un amplificateur de DDoS et un vecteur de pollution de table.** ✅ `[8.1b FAIT]`
*Constat (introduit au 8.1) :* une « carte de visite » porte `(id, adresse)` choisis par
l'émetteur, et on l'apprenait **sans vérifier la preuve de travail** ni **corroborer l'adresse**.
Trois sous-attaques isolées en relisant le code : (1) **pollution de table** par ids-poubelle
gratuits ; (2) **réflexion de PUNCH** — une carte `(id, addr=victime)` faisait que chaque nœud
perçait la victime 4×/s **pour toujours** ([net_punch] ne renonçait jamais) ; (3) **redirection
d'un pair connu** — `learn_peer` écrasait l'adresse d'un pair connu depuis n'importe quelle carte.
*Résolu (8.1b) :* défense en profondeur — **(a)** `has_pow` exigé sur chaque carte (ferme #1) ;
**(b)** le gossip n'écrase jamais l'adresse d'un pair connu (ferme #3) ; **(c)** abandon du perçage
spéculatif après ~10 s non corroborés (`PUNCH_GIVEUP`, ferme la DURÉE de #2) ; **(d)** rate-limit
d'apprentissage par source (ferme le DÉBIT d'injection de #1/#2). Logique centralisée dans `NetLink`
(`learn_from_gossip`) + fonction pure `punch_abandoned` partagée bot/jeu (anti-D2). *Prouvé :* un
VRAI attaquant `attack gossip-flood` déverse des cartes pointant vers une cible → **0 perçage
réfléchi** reçu, tables non polluées ; découverte honnête inchangée (`crowd 60` couverture 100 %),
essaim TENU avec l'attaquant actif. **47 tests, 0 warning.** *Résidus assumés :* multi-sources
contourne partiellement (d) ; NAT symétrique abandonné par (c) → relais (D17) ; pas de plafond
GLOBAL de perçage (jugé inutile pour l'instant). *Amorce D9 (corroboration multi-informateurs : 8.8).*

**D24 — Le client 3D plafonne la foule VISIBLE à 64 (`MAX_AVATARS`).** 🟠 `[ch. 8.2/8.3]`
*Constat (révélé au 8.1) :* [receive.rs](src/net/netcode/receive.rs) refuse de créer plus de
`MAX_AVATARS = 64` avatars (anti-DoS, chap. 6.5). Donc même quand le gossip fait CONNAÎTRE 200
joueurs, le vrai client n'en **affiche** que 64 — ma métrique de couverture (mesurée sur le bot,
qui n'a pas ce plafond) **surévalue** ce qu'un joueur perçoit réellement. *Pourquoi ça compte :*
« voir la foule » est le cœur de D22 ; tant que le client coupe à 64, D22 n'est pas tenu *dans le
jeu réel*, seulement dans l'abstraction du bot. *Piste :* remplacer le plafond plat par un
**budget de rendu à deux tiers** (8.2) — focus en maillage détaillé, conscience en LOD/imposteurs
bon marché — de sorte qu'on AFFICHE des centaines de silhouettes lointaines sans fondre le GPU,
tout en gardant la borne anti-DoS sur les avatars *détaillés*. *Vérif :* dans le vrai jeu 3D à
foule dense, on VOIT bien plus que 64 (proches nets + lointains en imposteurs), sans chute de FPS.

> *Mesure à corriger (dette, pas un doute durable) :* la couverture de perception compte
> aujourd'hui les pairs **connus** (dans `link.peers`), pas ceux dont on reçoit **vraiment** un
> état récent. À resserrer au 8.2 (compter « entendus récemment ») pour ne pas se mentir.

**D26 — L'AGRÉGATEUR (hôte de cellule / parent) peut MENTIR sur sa région.** 🟡 `[Phase B — DÉFRICHÉ 20 juin]`
*Constat (nommé le 20 juin, ouvert par 8.3) :* depuis 8.3, un **hôte de cellule** produit le RÉSUMÉ de la foule de
sa zone (combien, où) ; en Phase B, un **parent** agrégera et dégradera la foule pour ses protégés faibles. Or rien
ne l'empêche de **CACHER des gens** (éclipse douce) ou d'en **INVENTER** (fantômes). La signature prouve l'AUTHENTICITÉ
d'un état individuel, pas l'HONNÊTETÉ d'un *résumé* (qui agrège des tiers). *Atténué :* un résumé est CONSULTATIF, pas
autoritaire (deux hôtes en désaccord = deux flux, aucune corruption d'état).
*DÉFRICHÉ en H2 (20 juin, sous-agents + lecture du code) — DEUX couches :*
- **Couche 1 — AUTHENTIFIER le résumé (mécanique, gros gain).** Aujourd'hui le résumé est le SEUL paquet
  TOTALEMENT anonyme (`ingest_summary` [link.rs:479] ne vérifie que `ts` + `MAX_CELLS` — ni clé, ni sceau, ni
  « est-ce l'hôte ? »). Fix = copier le modèle des états : l'hôte **embarque sa clé + SIGNE** ; la fraîcheur
  devient un **`seq` par hôte** (l'anti-rejeu des états, qu'on a DÉJÀ) et NON l'horloge murale `ts` — attaquable,
  et D13 = pas d'horloge commune → ⚠ le patron `MAX_ORB_VERSION_JUMP` de l'orbe NE se transplante PAS (il borne un
  compteur monotone, pas une horloge). À l'ingestion : vérifier le sceau ET `émetteur == cell_host(cellule)`. Tue
  la forge anonyme, le `count=0` qui efface, et l'épinglage `ts=MAX` (seul le vrai hôte produit un `seq` plus haut).
  *Manque aussi : aucun rate-limit par source sur le résumé (le gossip en a un — à aligner).*
- **Couche 2 — CORROBORER (le vrai mur, couplé à D4).** Un mensonge SIGNÉ reste un mensonge : l'hôte légitime peut
  gonfler/cacher. Défense = ne croire un résumé que **recoupé** par ce qu'on perçoit en direct et/ou par K
  observateurs indépendants (diversité IP, comme 9.4b) ; « trou noir » détecté → je change de parent (D5). C'est
  « mesurer le réel plutôt que croire une déclaration » — le MÊME principe que D4 (n°7).
*Audit de surface (sous-agents, 20 juin) :* parmi les paquets non signés (WELCOME, PUNCH, GOSSIP, CELL_SUMMARY),
**seul le résumé est OUVERT** ; WELCOME (rendez-vous non fiable par conception, 6.1/D10), PUNCH et GOSSIP (déjà bornés
8.1b/D23, PoW exigée sur les cartes) sont non-signés *à dessein* mais maîtrisés. *Vérif :* un hôte qui cache/invente
>X % de sa cellule est détecté et contourné en N s. **Couche 1 = quasi mécanique (à coder en TÊTE de Phase B) ;
couche 2 = le design à prouver.**

### Catégorie 10 — Périmètre (le doute qui peut tuer le projet)

> *Catégorie créée le 22 juin : elle MANQUAIT. 26 doutes techniques, 0 de périmètre — or le seul qui peut tuer le
> projet n'est aucun des 26. Toute la rigueur était pointée VERS L'INTÉRIEUR du netcode ; aucune vers « est-ce la
> bonne chose à construire, pour qui, et quand un humain y touche ».*

**D27 — « La forteresse vide ».** 🔴 `[test 2-humains, prochaine action]`
*Constat :* on a bâti une forteresse réseau profonde et rare (crypto, gossip, AOI, anti-Sybil, anti-inflation
corroborée) dans laquelle **deux humains réels ne se sont jamais retrouvés via le vrai Internet**. *La preuve du
doute :* la mesure (b) « fraîcheur ressentie » — la SEULE grandeur qui décide si un espace social est *vivant* (si
ta position a 4 s de retard, l'espace est mort) — est la SEULE jamais chiffrée. Et ce n'est pas un oubli : le banc
bus headless en est **structurellement incapable** ((b) est une grandeur humaine-perceptuelle sur un lien réel avec
perte/jitter). *Conséquence méta :* la cadence anti-collapse (compile → test → **preuve headless**) a un angle mort
CÂBLÉ — elle ne peut prouver QUE ce que le simulateur sait prouver, et continuer à travailler le protocole peut
devenir une façon très sophistiquée d'éviter le test le plus effrayant : mettre la chose entre les mains d'une
personne. *Piste (= la seule honnête) :* deux humains, deux vrais NAT, le vrai Internet, critère de fraîcheur
pré-enregistré AVANT le test, conditions hostiles incluses (lien mobile, NAT symétrique). *Vérif :* (b) chiffrée en
direct, taux de connexion mesuré par type de NAT. **Ferme aussi, par ricochet, ce que D17/R1 cachaient.**

---

## D. Le programme (les chapitres à venir)

> Chaque chapitre liste : **But**, **sous-étapes** (à cocher comme au chap. 6),
> **doutes fermés**, **méthode de vérification**. On garde la discipline du chap. 6 :
> petites étapes, chacune compilée + testée + prouvée (souvent en headless/simu), puis
> commitée et écrite ici.

### Chapitre 7 — Confrontation au réel (latence, perte, NAT) 🔴 *priorité 1*
**But :** arrêter de mentir comme localhost. Mesurer la vérité.
- [x] 7.1 — Harnais `tc netem` sur `lo` : script `tools/sim-netem.sh` (3 profils
  `bon|moyen|mauvais`) qui applique latence/jitter/perte/ré-ordonnancement, lance `sim`,
  et **retire toujours** le netem à la sortie (`trap`, comme `sim-cool.sh`). *(fait)*
  **Piège tranché :** sur `lo`, le délai compte DOUBLE (aller + retour sur la même
  interface) → les profils sont exprimés en PING cible et le script applique `delay =
  ping ÷ 2`. **Prouvé réel** (profil `moyen`, ~120 ms + 2 % perte) : netem posé → simu
  → `lo` revenu à `noqueue` ; l'essaim a TENU (20/20 montés, voisins moy 21/plafond 32,
  ~5680 paquets honnêtes/s, orbe 0/20 volée, orb-creep mis en SOURDINE).
  > À creuser au 7.2 : le `teleport` n'a laissé **aucune** trace (0 paquet de triche
  > rejeté, 0 faute) — rejoint trop tard, perte netem, ou angle mort réel ?
- [x] 7.2 — Simu sous les 3 profils (`sim 50 5 30`), mesurée. *(fait)*
  **Résultats** (état honnête accepté par seconde) : `bon` (30 ms) ~22 668/s · `moyen`
  (120 ms, 2 %) ~14 553/s (−36 %) · `mauvais` (250 ms, 5 %, reorder) ~6 832/s (−70 %).
  **Sécurité INTACTE partout** : orbe 0/50 volée, téléport/orb-creep/sybil neutralisés
  même à 250 ms + ré-ordonnancement (le `teleport` agit bien avec une fenêtre ≥ 30 s ;
  à 10 s il n'avait pas le temps — artefact de durée, pas un trou). **Bug révélé :** le
  débit honnête s'effondre quand le réseau se dégrade. La perte (5 %) ne peut pas
  expliquer −70 % → cause = l'**anti-rejeu strict** `accept_seq` (`seq ≤ last → rejet`,
  link.rs) qui jette les paquets honnêtes **ré-ordonnés** par jitter/reorder. C'est D1
  matérialisé → cible directe du 7.3.
  > Limite de preuve : mécanisme (code) + symptôme (débit) prouvés ; le rapport ne
  > COMPTE pas encore les rejets `accept_seq` séparément. Le fix 7.3 (débit qui remonte)
  > sera la preuve causale — ou on instrumente un compteur (7.4).
  > **⚠ CORRECTION (mesurée au 7.3) : ce diagnostic était FAUX.** Le fix anti-rejeu ne
  > récupère que **+15 %** du débit, pas le −70 %. La vraie cause est le `limit 1000` par
  > défaut de `tc netem` (plafond ≈ limit/délai ≈ 8 000/s à 125 ms = la mesure), pas le
  > protocole. Détail et preuve à venir : voir 7.3 et 7.3b.
- [x] 7.3 — Anti-rejeu à FENÊTRE GLISSANTE (style IPsec/DTLS/WireGuard), fenêtre = 64
  (masque `u64`). `NetLink::accept_seq` (link.rs) : on retient le plus grand `seq` + un
  masque des 64 derniers ; on accepte tout `seq` neuf dans la fenêtre (← tolère le
  ré-ordonnancement), on refuse le rejeu (déjà vu) et le trop-vieux (hors fenêtre).
  +1 test (`accept_seq_tolere_le_reordonnancement`) ; l'ancien test anti-rejeu passe
  toujours. 36 tests, 0 warning, release vert. *(fait)*
  **Résultat honnête — mon pari du 7.2 était FAUX.** Re-mesuré : `mauvais` ~6 832 →
  ~7 876/s (**+15 % seulement**, pas ×2–×3) ; `bon` ~23 026/s (aucune régression). Donc
  **l'anti-rejeu n'était PAS la cause dominante du −70 %.** Le fix reste correct et
  nécessaire (les vrais réseaux ré-ordonnent), mais il ne résout pas le débit.
  **Vraie cause trouvée (arithmétique, à confirmer en 7.3b) :** `tc netem` a `limit 1000`
  PAR DÉFAUT → plafond de débit ≈ `limit/délai` = 1000/0,125 s ≈ **8 000/s** à 125 ms =
  pile la mesure (7 876). Recoupé : `moyen` (60 ms) plafond ≈ 16 667 (mesuré 14 553, sous
  le plafond) ; `bon` (15 ms) plafond ≈ 66 667 (pas bridé). Le goulot était la **file du
  harnais**, pas le protocole.
- [x] 7.3b — `limit 100000` dans la règle netem de `tools/sim-netem.sh` (file non
  bloquante) + re-mesure des 3 profils. **PROUVÉ :** sous `mauvais`, le débit honnête
  remonte de ~7 876 (limit 1000) à **~21 287/s** ; `bon` ~23 289/s → **−9 % seulement**
  (≈ la perte de 5 % × double traversée `lo`, donc essentiellement optimal). Orbe 0/50,
  attaques neutralisées. *(fait)*
  **Conclusion du faux débat 7.2/7.3 :** le −70 % était à **100 %** l'artefact `limit 1000`
  du harnais, PAS le protocole. **Le protocole tient sous réseau réel** (250 ms + jitter +
  5 % perte + ré-ordonnancement). Le fix anti-rejeu (7.3) reste correct et utile (les vrais
  réseaux ré-ordonnent), il n'était juste pas le goulot. Aucun défaut réseau résiduel à ce
  stade — le cœur du chapitre 7 (« arrêter de mentir comme localhost ») est atteint.
- [x] 7.4 ✓ — Instrumenter `sim` : Ko/s ↑↓ + CPU réels **par nœud** (compteurs prise +
  `/proc/thread-self/stat`), RAM crête **globale** (pas de RAM/nœud factice). (nouveau `src/net/probe.rs`)
- [x] 7.4b ✓ — **Fidélité + densité.** Le bot passe par le VRAI chemin AoI (water-filling,
  mêmes fonctions qu'[aoi.rs]) → coût re-mesuré **↑34/↓31 Ko/s/nœud** (et non 89, artefact du
  bot naïf). Le rapport AVOUE désormais la **réserve de densité (D22)** : en foule, aveugle
  au-delà de 32. **Ferme D19, ouvre D22 (→ chapitre densité dédié).**
- [x] 7.5 ✓ — NAT MULTI-joueurs : `tools/test-nat.sh` prend un N (défaut 3), crée N maisons
  (`p1..pN` derrière `nat1..natN`) + résumé du mesh (trous ouverts / N−1). Logique multi-pair
  prouvée sur localhost (mesh 3 joueurs = 6/6). **Bug d'instrumentation corrigé** ([natdemo.rs] :
  le trou s'ouvrait en SILENCE si les données arrivaient avant le punch → sous-comptage).
  **PREUVE NAT RÉELLE FAITE** (sudo, namespaces + NAT, ~16 s) : `--cone` → 6/6 MESH COMPLET ;
  symétrique → 0/6 (relais ch.5). + 2 bugs du script corrigés (hang `wait`/rendez-vous ; `set -e`
  vs code 124 de `timeout`).
**Ferme :** D1, D19 (et révèle des correctifs réseau réels + le doute densité D22).
**Vérif :** rapport de simu sous netem montrant que l'essaim tient avec de *vrais* défauts réseau.

### Chapitre 8 — La foule dense & l'inclusivité (fermer D22) 🔴 *priorité 2 — LE gros morceau d'archi*
**But :** que dans une foule de 200, 500, 5000, chacun **perçoive la foule entière**
(proches nets, lointains dégradés) **sans plafond dur à 32** et **sans exploser son
débit** — et que le 0-connexion comme le 2 Gb/s aient chacun LA meilleure expérience.

> **Pourquoi ce chapitre existe (constat mesuré au 7.4b, voir D22).** Tout notre passage
> à l'échelle (6.6) achète son « coût constant à 55k » par un **plafond dur** :
> [rendezvous.rs](src/net/rendezvous.rs) ne présente que les `MAX_NEIGHBORS = 32` plus
> proches, et [receive.rs](src/net/netcode/receive.rs) **écrase** `link.peers` avec ce
> roster (les clients ne s'échangent RIEN entre eux). Donc le **33e voisin n'existe pas
> et ne peut jamais exister** — le water-filling d'[aoi.rs](src/net/aoi.rs) répartit le
> débit entre 32 connus mais n'apprend jamais le 33e. À une foule de 200, chacun est
> **aveugle à 168**. Dans un monde social type VRChat, VOIR la foule EST le jeu.

> **L'idée directrice — séparer deux choses que le plafond à 32 confond :**
> 1. le **FOCUS** = à qui je tiens un lien netcode plein débit (prédiction/20 Hz). Ça
>    DOIT rester borné (~16-32). C'est, en gros, le système actuel.
> 2. la **CONSCIENCE** = qui je perçois / sais exister (des centaines), en LOD/basse
>    fidélité. Ça ne doit **PAS** être plafonné. C'est ton « **AoI par vision** » : pas
>    besoin d'un lien 20 Hz avec un type à 80 m, juste de savoir qu'il est là, pour pas cher.

> **⚠ L'INVARIANT À TENIR (le piège à ne JAMAIS se cacher).** Le coût de réception doit
> rester **O(K_focus + C_cellules)**, *indépendant de N* (la taille de la foule). Sinon on
> a juste rebaptisé le problème. La PREUVE de réussite n'est donc pas « la couverture
> monte » seule — c'est **couverture → ~100 % ET débit ↓ qui reste PLAT quand la foule
> grandit**. Augmenter `MAX_NEIGHBORS` est interdit : ça rouvre l'O(N²) (trou n°3), le
> WELCOME débordé (trou n°2) et surtout D3 (le faible noyé). On déplacerait le mur, pas
> on ne le casserait.

**— Phase A : VOIR la foule sans plafond dur (le cœur de D22) —**

- [x] 8.0 ✓ — **Scénario de foule + métriques de perception dans `sim` (mesurer le mur AVANT
  de le casser).** Nouveau mode `cargo run -- crowd <N> [secs]` (N bots co-localisés sur un
  cercle de 3 m → tous réellement à portée) et DEUX mesures neuves au rapport, à côté du
  probe 7.4 (Ko/s ↓, CPU) :
  • **Couverture de perception** = sur les pairs réellement à portée (actifs − 1), combien
    ce nœud perçoit-il ? Aujourd'hui : plafonné → `32/(N−1)`.
  • **Tiers de fidélité** = *focus* (lien plein) vs *conscience* (basse fidélité). Aujourd'hui :
    conscience = 0 (le tier n'existe pas encore → 8.2).
  **PROUVÉ (le mur, en ROUGE comme prévu) :** `crowd 60` → couverture **54 %** (aveugle à 27) ;
  `crowd 200` → couverture **16 %**, FOCUS 32 + CONSCIENCE 0, **aveugle à 167**. Le chiffre
  prédit (16 %) = le mesuré. **Débit de référence À BATTRE** (le coût qui devra rester PLAT
  quand la couverture montera) : `crowd 200` → **↓ 24,8 Ko/s moy (41 max), ↑ 26,8 Ko/s** ;
  l'essaim TIENT (orbe 0/200). *Rien n'est « résolu » ici — le mur est juste rendu chiffrable
  et reproductible, pour qu'on sache à la fin si on l'a vraiment cassé.* 36 tests, 0 warning.

> **⚙ CONCEPTION DÉTAILLÉE 8.1 (écrite AVANT de coder, 19 juin) — avec une DÉCOUVERTE faite
> en lisant le code.**
>
> **La bonne nouvelle d'abord (le water-filling nous a déjà à moitié sauvés).** En relisant
> [aoi.rs](src/net/aoi.rs) + [bot.rs](src/net/bot.rs), le budget d'émission est FIXE
> (`SEND_BUDGET_HZ = 240`) et réparti entre les pairs connus. Conséquence ARITHMÉTIQUE : si
> chacun connaît N−1 pairs, il envoie ~240/(N−1) Hz à chacun ; un receveur reçoit donc de
> (N−1) émetteurs × 240/(N−1) = **~240 Hz au total, QUEL QUE SOIT N**. Autrement dit, **le coût
> de réception est DÉJÀ borné** par la générosité (fixe) des émetteurs — l'invariant « débit
> plat » est à moitié déjà tenu, côté débit. Ce qui n'est PAS tenu : (1) la DÉCOUVERTE (plafond
> 32 → le 33e jamais appris) et (2) la FRAÎCHEUR par pair, qui s'effondre uniformément en 1/N
> (à 5000, une maj toutes les ~20 s pour tout le monde → trop vieux même pour du LOD).
>
> **Ce que ça change pour le plan (et l'ordre se confirme) :**
> - **8.1 (gossip) seul** doit faire MONTER la couverture 16 %→~100 % à `crowd 200`, avec un
>   débit qui monte au plafond (~43 Ko/s) puis reste PLAT à 500. Preuve que le plafond 32 était
>   arbitraire. MAIS tout le monde sera également « flou » (~1 Hz).
> - **8.2** rend les PROCHES nets (focus 20 Hz ; la conscience ne touche que les miettes du budget).
> - **8.3** fait tenir la fraîcheur des LOINTAINS à 5000 : une région = UN flux agrégé, pas des milliers.
>
> **Mécanique concrète de 8.1 :**
> - Nouveau paquet `KIND_GOSSIP` (carte de visite), nouveau `src/net/gossip.rs` :
>   `[type|ver|count| (id 32, ip 4, port 2, x 4, z 4)×count ]`, `count` borné (≤16 → paquet < 800 o).
> - [receive.rs](src/net/netcode/receive.rs) + [bot.rs](src/net/bot.rs) : le WELCOME n'ÉCRASE
>   plus `link.peers`, il l'AMORCE (merge). On RETIRE le `retain(present)` du bot (ligne ~194).
> - Chaque nœud émet périodiquement (~1 Hz) un GOSSIP vers quelques pairs, avec un sous-ensemble
>   DIVERS (tirage tournant, pas toujours les mêmes → anti-éclipse) des pairs qu'il connaît.
> - À la réception : on fusionne les cartes INCONNUES (hole=false → on les perce ensuite via la
>   machinerie PUNCH existante), table bornée en MÉMOIRE (`MAX_KNOWN`, éviction des plus vieux →
>   amorce D16). Aucune confiance à une source unique (corroboration durcie en 8.8).
> - **Preuve :** `crowd 200` couverture 16 % → ~100 % ; puis `crowd 500` montre le débit ↓ PLAT
>   (ne croît pas avec N) → le plafond est cassé sans rouvrir l'O(N²) ni exploser le débit.

- [x] 8.1 ✓ — **Découverte décentralisée par gossip (le 33e devient APPRENABLE).** Le
  rendez-vous cesse d'être l'énumérateur autoritaire et redevient un simple **amorçage** :
  le WELCOME n'**écrase** plus `link.peers`, il l'**amorce**. Ensuite, chaque pair annonce
  à bas débit, à ses voisins, quelques AUTRES pairs qu'il connaît (id + dernière position
  connue) — des « cartes de visite ». La table de pairs s'enrichit ainsi **sans** plafond à
  32 et **sans** serveur central qui énumère. Table bornée en MÉMOIRE (lié D16) mais pas en
  VISION. **Anti-éclipse dès le départ (amorce D9) :** diversité forcée des informateurs, on
  corrobore une position par plusieurs sources (un menteur seul ne peut pas te cacher/inventer
  la foule). *Ferme une partie de D10 (rendez-vous démoté à l'amorçage) ; amorce D9.*
  **Preuve :** à `crowd 200`, couverture 16 % → ~100 % (chacun finit par apprendre les 200),
  sans que le rendez-vous ne les énumère. *Risque à surveiller : le gossip lui-même coûte du
  débit — c'est 8.2 + 8.3 qui le bornent ; à 200 c'est tenable, à 5000 il FAUDRA l'agrégation.*
  **FAIT (19 juin) — le mur tombe, l'invariant TIENT.** Nouveau `src/net/gossip.rs`
  (paquet `KIND_GOSSIP` = « cartes de visite » : id + adresse + dernière position, ≤16/paquet,
  sérialisation à la main + 6 tests). Logique d'apprentissage CENTRALISÉE dans `NetLink`
  (`learn_peer` + `note_pos` + `peer_pos`, borne mémoire `MAX_KNOWN = 4096`) → **partagée par le
  bot ET le jeu** (on évite de rouvrir D2). Le WELCOME **amorce** désormais `link.peers` (merge),
  il ne l'**écrase** plus ([receive.rs] + [bot.rs], `retain(present)` supprimé). Chaque nœud diffuse
  ~2 Hz un lot de cartes (sous-ensemble DIVERS par curseur tournant → amorce anti-éclipse) à
  `GOSSIP_FANOUT = 4` voisins ; à la réception on apprend les inconnus (puis on les perce).
  **MESURÉ** (`crowd`, PC tour ASUS, ventilos au max) : couverture **16 % → 98 %** à `crowd 200`
  (50 s ; moy 194/199 voisins), **67 %** à `crowd 500` (40 s, convergence non finie : démarrage
  échelonné). **L'INVARIANT EST PROUVÉ** : le débit ↓ ne grandit PAS de 200 à 500 (↓35 → ↓27 Ko/s,
  ↑42 → ↑39 Ko/s, **PLAT/en baisse**), CPU ~0,7 % inchangé, orbe 0 volée, essaim TENU. 42 tests,
  0 warning. *Pourquoi le débit reste plat sans rien faire de plus : le budget d'émission est FIXE
  (240 Hz) et réparti → quand chacun connaît N−1 pairs, il envoie 240/(N−1) Hz à chacun, donc un
  receveur reçoit (N−1)×240/(N−1) = ~240 Hz, indépendant de N (la DÉCOUVERTE était le seul mur).*
  **Résidu pour 8.2/8.3 :** la fraîcheur PAR pair s'effondre en 1/N (uniforme) → 8.2 rend les
  proches nets (focus 20 Hz) ; 8.3 fait tenir les lointains à 5000 (agrégation par cellule).

> **⚙ CONCEPTION DÉTAILLÉE 8.1b (écrite AVANT de coder, 19 juin) — la rançon honnête du gossip.**
>
> **Le diagnostic, en relisant le code (D23 n'est pas UN bug mais TROIS sous-attaques).** Une carte
> de visite affirme `(id, adresse)` choisis par l'émetteur, SANS preuve. En lisant [punch.rs](src/net/punch.rs),
> [send.rs](src/net/netcode/send.rs), [bot.rs](src/net/bot.rs) et [receive.rs](src/net/netcode/receive.rs),
> j'ai isolé exactement ce qu'un menteur peut en faire :
> 1. **Pollution de table (ids-poubelle).** `learn_peer` n'exige PAS `has_pow` sur l'id d'une carte.
>    Un attaquant forge des milliers d'ids gratuits → il remplit `link.peers` jusqu'à `MAX_KNOWN = 4096`,
>    et comme il n'y a pas d'éviction (D16), les vrais pairs ne rentrent plus. Coût attaquant : ~0.
> 2. **Réflexion / amplification de PUNCH (le vrai danger).** Découverte clé : le STATE et le gossip
>    ne partent QU'aux trous DÉJÀ ouverts (un trou ne s'ouvre que quand on a *entendu* le pair). La SEULE
>    chose qu'on envoie à une adresse jamais corroborée, c'est le **PUNCH** — et [net_punch](src/net/punch.rs)
>    le répète vers une adresse jamais confirmée **indéfiniment** (4×/s, sans jamais abandonner). Donc une
>    carte `(id quelconque, adresse = VICTIME)` fait que CHAQUE nœud qui la reçoit perce la victime 4×/s,
>    pour toujours. Diffusée à la foule → flot réfléchi soutenu vers la victime, source (l'attaquant) masquée.
> 3. **Redirection d'un pair connu.** `learn_peer` rafraîchit l'adresse d'un pair DÉJÀ connu depuis
>    n'importe quelle carte. Une carte `(id-d-un-vrai-pair, adresse = VICTIME)` détourne donc notre trafic
>    vers la victime ET nous fait perdre le vrai pair.
>
> **Les défenses (en profondeur — chacune dit son rôle ET sa limite, règle anti-enfermement).**
> - **(a) PoW exigée sur chaque carte apprise.** On rejette tout id sans `has_pow(POW_BITS)`. Une fausse
>   identité coûte alors ~2¹⁶ (comme une vraie). *Ferme #1. Limite : n'arrête PAS la réflexion via des ids
>   RÉELS récoltés au rendez-vous/dans le gossip (#2/#3).*
> - **(b) Le gossip n'ÉCRASE jamais l'adresse d'un pair déjà connu.** On sépare « apprendre via gossip »
>   (ouï-dire → n'AJOUTE que des inconnus) de « rafraîchir via WELCOME/paquet signé » (corroboré). *Ferme #3.
>   Limite : un vrai changement d'adresse (NAT rebinding) ne sera vu que via le prochain paquet signé du pair,
>   pas via gossip — c'est voulu.*
> - **(c) Perçage spéculatif borné ET abandonné.** [net_punch] (et le bot) cessent de percer un trou jamais
>   corroboré après un délai généreux (~`PUNCH_GIVEUP` essais ≈ 10 s) au lieu de marteler à vie. *Ferme la
>   DURÉE de #2 : une carte empoisonnée n'arrose la victime que quelques secondes, plus l'éternité. Limite :
>   un pair derrière NAT symétrique (qui ne répond jamais au perçage direct) sera aussi abandonné → il lui
>   faudra un relais (D17), pas un perçage éternel. C'est le bon compromis.*
> - **(d) Rate-limit de l'apprentissage PAR SOURCE.** Un même expéditeur de gossip (son adresse) ne peut
>   nous faire apprendre qu'un nombre borné de NOUVEAUX pairs/s (seau à jetons par source, comme le 5.5).
>   *Ferme le DÉBIT d'injection (#1 et #2 à la source). Limite : un attaquant à plusieurs sources contourne
>   partiellement — mais chaque source reste bornée, et émettre du gossip crédible suppose un PoW payé.*
> - *NON retenu pour 8.1b (anti-gold-plating) : un plafond GLOBAL de perçage/s. (c)+(d) bornent déjà débit
>   ET durée ; si la preuve montre un résidu, on l'ajoutera — pas avant.*
>
> **Où vit le code (anti-D2).** Toute la confiance des cartes va dans `NetLink` (`learn_from_gossip` +
> le seau par source), comme `learn_peer` au 8.1 → **partagée par le bot ET le jeu**, testée une fois. Les
> appelants ne font que « recharger les seaux » + « tenter d'apprendre ». L'abandon de perçage touche
> [punch.rs] (jeu) et la boucle de perçage du [bot.rs] (miroir).
>
> **Preuve (un VRAI attaquant, fidèle à la philosophie du projet).** `cargo run -- attack gossip-flood` :
> s'inscrit, récolte les vraies victimes, ouvre une 2ᵉ prise « cible de réflexion », puis déverse aux
> victimes des cartes empoisonnées `(id-poubelle, addr=cible)` et `(id-réel, addr=cible)` ; il COMPTE les
> perçages réfléchis arrivant sur la cible. Attendu : ~0 (les poubelles rejetées par PoW, les ids réels non
> écrasés). + tests unitaires sur `learn_from_gossip` (PoW rejeté, adresse non écrasée, seau par source qui
> plafonne) et sur l'abandon de perçage. + `gossip-flood` ajouté aux variantes de `sim` : l'essaim TIENT,
> couverture honnête inchangée.

- [x] 8.1b ✓ — **Durcir le gossip (ferme D23) — AVANT d'empiler 8.2.** Le 8.1 a échangé le plafond
  de 32 contre une porte d'entrée DoS : on apprenait des cartes `(id, adresse)` sans preuve de travail
  ni corroboration → réflexion/amplification possible vers une victime, et pollution de table. On a
  refermé AVANT de construire dessus (règle d'or : pas de béton sur du sable). Quatre défenses en
  profondeur : (a) `has_pow` exigé sur l'id d'une carte apprise (poubelles jetées) ; (b) le gossip
  n'écrase JAMAIS l'adresse d'un pair déjà connu (anti-redirection) ; (c) abandon du perçage spéculatif
  après ~10 s non corroborés (`PUNCH_GIVEUP`, [punch.rs]) — avant on arrosait à VIE ; (d) rate-limit de
  l'apprentissage PAR SOURCE (seau à jetons dans `NetLink`). Logique centralisée dans `NetLink`
  (`learn_from_gossip`) + une fonction pure partagée `punch_abandoned` → bot ET jeu, pas de re-D2.
  **FAIT (19 juin) — la porte DoS est fermée, la découverte honnête intacte.** Nouvel attaquant RÉEL
  `cargo run -- attack gossip-flood` ([attack.rs]) : il ouvre une 2ᵉ prise « cible » et déverse aux
  victimes des cartes pointant toutes vers elle. **MESURÉ : 0 perçage réfléchi reçu par la cible**
  (poubelles jetées par PoW, ids réels non redirigés), table de la victime NON polluée (reste à ses
  vrais pairs). La découverte honnête est inchangée : `crowd 60` → couverture **100 % (min 90 %)**,
  orbe 0/60. `sim 40 6 20` (avec `gossip-flood` ajouté aux variantes) → essaim TENU, orbe 0/40,
  couverture 100 %. **47 tests, 0 warning** (+5 : `learn_from_gossip` PoW/no-overwrite/rate-limit,
  abandon de perçage, aller-retour PUNCH). *Limites assumées (registre) : un attaquant à plusieurs
  sources contourne partiellement le rate-limit (chaque source reste bornée) ; un pair NAT symétrique
  est aussi abandonné par (c) → relais nécessaire (D17). Plafond GLOBAL de perçage NON ajouté
  (anti-gold-plating) : (c)+(d) suffisent, on l'ajoutera si une mesure le réclame.* *Ferme D23 ; amorce D9.*

- [x] 8.1b-preuve ✓ — **Preuve RÉELLE de l'abandon de perçage (doute #1 FERMÉ).**
  *Honnêteté :* le « 0 perçage réfléchi » du 8.1b couvrait les cartes-poubelle (jetées par PoW) et les
  ids déjà connus (non redirigés). Le cas le plus DUR — un attaquant qui **mine de vraies identités PoW
  neuves** et les pointe vers une victime — n'était prouvé que par TESTS UNITAIRES (`punch_abandoned` +
  seau par source), pas bout-en-bout. Trou de *démonstration*, pas de *défense*. **FAIT (19 juin) :**
  `attack gossip-flood` mine maintenant `N_POW = 4` identités PoW neuves pointant vers la cible et compte
  les perçages réfléchis **par fenêtres de 2 s sur ~20 s**, EN inondant sans arrêt. **MESURÉ (3 bots) :**
  rafale CONSTANTE de **96 perçages/2 s de t=0 à t=10 s, puis 0 de t=10 à t=20 s** (total 480, queue 0).
  L'abandon mord **pile à ~10 s** (`PUNCH_GIVEUP = 40 × 0,25 s`), **même en continuant d'inonder** —
  re-déverser une carte ne réarme pas le perçage (un id connu n'est pas réappris). Avant 8.1b : 96/2 s
  **SANS FIN**. *La réflexion par ids PoW minés est donc BORNÉE dans le temps, prouvé en réel — pas
  seulement en test unitaire. Doute #1 fermé.* *(Résidu inchangé : un attaquant peut relancer une
  rafale en minant de NOUVELLES identités, mais chacune coûte un PoW et ne dure que ~10 s → plus une
  amplification, juste un coût attaquant linéaire ; la corroboration multi-informateurs du 8.8 le réduira encore.)*

> **⚙ CONCEPTION DÉTAILLÉE 8.2 (écrite AVANT de coder, 19 juin) — avec une DÉCOUVERTE en relisant [aoi.rs].**
>
> **Le vrai problème, vu dans le code (pas celui que je croyais).** Le water-filling d'[aoi.rs] répartit
> le budget par `relevance_weight(distance)`. En géométrie VARIÉE (vrai monde), ça donne déjà un dégradé :
> proches ≈ 20 Hz, lointains peu. MAIS en **foule DENSE** (le cas `crowd` : tout le monde à ~même
> distance, sous `COMFORT_DIST = 6 m`), **tous les poids sont ~égaux** (~1,0) → le budget 240 Hz se
> répartit UNIFORMÉMENT → ~240/(N−1) Hz pour chacun (≈1,2 Hz à 200). **Tout le monde est également flou.**
> C'est ça que 8.1 a laissé ouvert et que 8.2 doit casser : faire émerger des PROCHES nets même quand la
> pertinence-distance ne discrimine plus.
>
> **L'idée : deux tiers EXPLICITES (pas juste un dégradé).**
> - **FOCUS** = les `K_FOCUS` pairs les PLUS pertinents (≈16). Lien plein : jusqu'à `SEND_HZ = 20`,
>   prédiction/interpolation, avatar 3D détaillé. Borné → c'est la borne d'échelle (on NE touche PAS à
>   `MAX_NEIGHBORS` côté émission ; le focus est un sous-ensemble servi en priorité).
> - **CONSCIENCE** = tout le reste de la table connue. Petit budget réservé, débit plafonné bas
>   (`CONSCIENCE_HZ` ≈ 1–2 Hz), pas de prédiction fine, rendu LOD/imposteur bon marché.
>
> **Mécanique d'allocation (extension d'[aoi.rs], pas une réécriture).** On trie les pairs par
> `relevance_weight` ; les `K_FOCUS` premiers reçoivent un water-filling avec la GROSSE part du budget
> (≈ `K_FOCUS × SEND_HZ`), le reste (conscience) un water-filling avec la part résiduelle et un `r_max`
> bas. *Différence clé avec aujourd'hui : on RÉSERVE de quoi rendre les proches nets AVANT de saupoudrer
> le reste, au lieu de tout étaler.* En foule dense où les poids sont égaux, le « tri » choisit K pairs
> (arbitraire mais STABLE) → en vrai jeu la géométrie variée les choisit naturellement.
>
> **⚠ L'invariant tient toujours (à re-prouver, pas à affirmer).** Réception d'un nœud = somme de ce que
> les autres lui envoient. Il est dans le FOCUS de ses ~`K_FOCUS` voisins proches (≈ `K_FOCUS × 20` Hz)
> + dans la CONSCIENCE de tous les autres (chacun lui donne une miette ; la somme des miettes ≈ le budget
> conscience d'UN émetteur, **indépendant de N**). Donc réception ≈ `K_FOCUS × 20 + budget_conscience` =
> **PLAT quand N grandit**. La preuve = rejouer `crowd 200` puis `500` et montrer ↓ plat ET focus≈K plein.
>
> **+ Honnêteté de mesure (dette du 8.1).** La couverture compte aujourd'hui les pairs CONNUS (`link.peers`).
> On la corrige pour compter les **ENTENDUS récemment** (un instantané reçu dans les N dernières s). Le
> rapport `crowd` distinguera alors FOCUS (entendus à plein débit) / CONSCIENCE (entendus en basse fidélité).
>
> **+ Amorce D24 (rendu à deux tiers).** Côté [receive.rs] : les pairs FOCUS → avatar détaillé (la borne
> `MAX_AVATARS` ne s'applique plus qu'à EUX, anti-DoS conservé) ; les CONSCIENCE → imposteur LOD bon marché
> (silhouette/point), plafond bien plus haut. *Je code la logique de tiers (vérifiable au bot : débit, qui
> est focus/conscience), mais le RENDU 3D (FPS, silhouettes) ne se vérifie que dans le vrai jeu → c'est TOI
> qui le lances ; je te donnerai `tools/foule-3d.sh <N>` pour ouvrir une foule d'un coup.*
>
> **Sous-étapes prévues (chacune compilée + testée + commitée) :** **8.2a** allocation à deux tiers dans
> [aoi.rs] (+ tests : focus servi plein, conscience en miettes, budget respecté) appelée par [send.rs] ET
> [bot.rs] ; **8.2b** métrique « entendus récemment » + tiers focus/conscience au rapport `crowd` ; **8.2c**
> rendu à deux tiers ([receive.rs]) + `tools/foule-3d.sh` (amorce D24, vérif 3D par l'utilisateur).
> **Preuve globale :** `crowd 200`/`500` → couverture (entendue) ~haute, focus≈`K_FOCUS` à plein débit,
> conscience = le gros de la foule en basse fidélité, **débit ↓ PLAT** quand N grandit.

- [ ] 8.2 — **AoI à DEUX TIERS : focus (≤K, plein débit) + conscience (basse fidélité).**
  Séparer dans le code « à qui je tiens un lien netcode complet » (borné ~16-32, prédiction/
  réconciliation/20 Hz, ≈ aujourd'hui) de « qui je perçois » (tout le reste de la table :
  position échantillonnée ~1 Hz, pas de prédiction fine, rendu LOD/imposteur). Le
  water-filling d'[aoi.rs](src/net/aoi.rs) s'étend : il sert d'ABORD le focus, puis saupoudre
  un PETIT budget résiduel sur la conscience. *C'est ton « AoI par vision » concrétisée.*
  **+ Honnêteté de mesure (dette du 8.1) :** corriger la couverture pour compter les pairs
  **vraiment entendus récemment**, pas seulement *connus* dans `link.peers`.
  **+ Amorce D24 :** relier `MAX_AVATARS` au tier de rendu — focus en avatars détaillés (borne
  anti-DoS conservée), conscience en LOD/imposteurs bon marché → on AFFICHE des centaines de
  silhouettes sans le plafond plat de 64. **Preuve :** couverture (entendue) ~100 % MAIS le débit ↓
  reste **borné** (focus ≈ 32 pleins, la conscience ne coûte que des miettes) → la vraie preuve de
  D22 : couverture↑ **sans** explosion du débit ; et dans le vrai jeu 3D on voit > 64 silhouettes.
  - **8.2a ✓ FAIT (19 juin)** — allocation à deux tiers `allocate_two_tier` dans [aoi.rs] (+ 3 tests :
    focus émerge en foule dense, petit groupe tout au plein débit, le plus pertinent passe au focus),
    branchée sur le jeu ([send.rs]) ET le bot ([bot.rs]). **Choix corrigé en codant : `K_FOCUS = 8`,
    PAS 16** — à 240 Hz de budget, 16 focus (×20 = 320) videraient tout et tueraient la conscience ;
    8 (×20 = 160) laissent **80 Hz garantis** à la conscience. Sanity `crowd 60` : essaim TENU, orbe
    0/60, débit borné (↑45 Ko/s). 50 tests, 0 warning. *La preuve PARLANTE (focus net / conscience LOD
    distingués) vient avec 8.2b — la métrique actuelle compte les CONNUS, pas la fidélité d'écoute.*
  - **8.2b ✓ FAIT (19 juin) — et la métrique a DÉMASQUÉ un défaut de 8.2a (le but d'une bonne mesure).**
    Le rapport `crowd` compte désormais les pairs **ENTENDUS** sur la fenêtre (compteur par pair dans
    [bot.rs], remis à zéro au début de la fenêtre) et les classe en **FOCUS** (≥ ~5 Hz, plein débit) vs
    **CONSCIENCE** (entendu mais moins). ⚠️ **CE QU'ELLE RÉVÈLE (mesuré) :** `crowd 80` → FOCUS 8,6 +
    CONSCIENCE 68 (couv. 98 %) — beau ; MAIS `crowd 160` → **FOCUS 0,2** + CONSCIENCE 147 (couv. 92 %),
    et ça **ne remonte pas** avec une fenêtre plus longue (40 s) → **pas un artefact de convergence, un
    vrai défaut.** *Cause (en relisant le code) :* le focus CHURNE — `relevance_weight` dépend des
    positions VIVANTES (les bots bougent) → le « top-8 » se recalcule à chaque tick ; en foule dense des
    dizaines de pairs sont à quasi-égalité au bord du top-8 → l'ensemble focus change sans cesse → aucun
    lien 20 Hz SOUTENU. *Pire :* un pair pas-encore-entendu a distance 0 → poids MAX (le coup de pouce
    « découvre vite ») → le focus cible les INCONNUS, qui sortent une fois entendus → rotation perpétuelle.
    **Conséquence : le budget plein débit est ÉTALÉ → retour au « tout le monde flou » que 8.2 devait
    casser.** À 80 ça marchait par chance (moins de churn). *La métrique a fait son travail : 8.2 n'est
    PAS fini.* 50 tests, 0 warning.
  - **8.2a-bis ✓ FAIT (19 juin) — focus COLLANT : le churn est mort, l'invariant TIENT.** Ensemble
    focus PERSISTANT dans `NetLink` (`focus: Vec<PeerId>` + `refresh_focus`/`is_focus`), maintenu par
    HYSTÉRÉSIS : (1) on retire les partis, (2) on remplit les places libres par les plus pertinents, (3)
    on ne REMPLACE un membre que si un autre est `FOCUS_SWAP_MARGIN = 1,5`× plus pertinent (un échange/tick).
    DÉCOUPLAGE découverte/focus : la pertinence vient de la position CONNUE (`peer_pos`) ; un pair sans
    position connue a pertinence 0 → il n'accapare plus de slot (il se fait entendre par la conscience, pas
    en volant le plein débit). `allocate_rates` → `allocate_tiers(weights, is_focus, …)` : le focus est
    DONNÉ, plus recalculé au tri. Logique centralisée dans `NetLink` → partagée [send.rs] (passé en `ResMut`)
    ET [bot.rs] (anti-D2). **PROUVÉ (fenêtre identique 30 s) :** `crowd 160` FOCUS **0,2 → 9,4** (le churn
    était bien la cause). Pair d'invariant 80↔160 : **FOCUS borné 8,8 → 10,5** (ne grossit pas avec N),
    **CONSCIENCE 68 → 134** (= LOD de toute la foule, scale avec N), **débit ↓ 43,8 → 40,4 Ko/s = PLAT**
    quand N double, orbe 0 volée. +1 test (`focus_est_collant_pas_de_churn` : stable sous bruit, accepte un
    pair nettement plus proche). 51 tests, 0 warning. *L'invariant de D22 est enfin tenu POUR DE VRAI :
    couverture en deux tiers (proches nets + foule en LOD) à débit reçu CONSTANT. Reste 8.2c (rendu 3D).*
    *Résidu honnête : couverture « entendue » à 30 s = 91 % à 160 (convergence non finie dans la fenêtre,
    comme au 8.1) ; le tri des candidats focus est O(N log N)/tick → à revoir pour 5000 (index spatial, 8.3).*
  - **8.2c ✓ CODÉ (19 juin) — rendu 3D à deux tiers (ferme l'amorce D24) — ⏳ VÉRIF VISUELLE = utilisateur.**
    [receive.rs] : un avatar porte un tier (`detailed` dans [state.rs]). FOCUS (`link.is_focus`) → avatar
    DÉTAILLÉ (corps articulé + tête + pseudo), borné par `MAX_FOCUS_DETAIL = 64` (la borne anti-DoS ne pèse
    plus que sur le rendu COÛTEUX). CONSCIENCE → IMPOSTEUR LOD (une silhouette capsule, sans tête ni pseudo),
    borné bien plus haut (`MAX_AWARE = 512`). Bascule de tier à la volée (despawn+respawn) quand un pair
    entre/sort du focus — rare car le focus est collant (8.2a-bis). [nameplates.rs] n'étiquette QUE le focus
    (sinon 500 labels illisibles). Nouveau `tools/foule-3d.sh <N>` : ouvre le rendez-vous + N fenêtres
    clientes d'un coup (récupère l'env nix une fois, lance le binaire direct → rapide ; Ctrl-C ferme tout).
    **51 tests, 0 warning, build vert.** ⚠️ *JE NE PEUX PAS LE VOIR (pas de GPU ici) — c'est TOI qui valides :*
    lance `./tools/foule-3d.sh 80`, pilote une fenêtre, et confirme : (1) on voit BIEN PLUS que 64 silhouettes,
    (2) ~8 détaillées (avec pseudo) près de toi + la foule en imposteurs, (3) pas de chute de FPS. *Doutes
    que je te signale d'avance : (a) un « pop » visuel possible quand un pair bascule de tier (despawn/respawn) ;
    (b) 512 imposteurs = 512 dessins → si ça rame sur ta carte, on baissera `MAX_AWARE` ou on fera de l'instancing ;
    (c) le binaire est lancé hors nix-shell via `LD_LIBRARY_PATH` capturé — si une fenêtre refuse de s'ouvrir,
    dis-le, on ajustera.* **→ ✓ CONFIRMÉ À L'ÉCRAN (19 juin, capture utilisateur, 80 fenêtres) : LOD parfait,
    > 64 visibles, pas de lag de rendu. D24 FERMÉ.** *(Les 80 fenêtres se sont ouvertes sans souci — niri +
    netcode encaissent une vraie foule.) Spawn éparpillé dans la salle ajouté pour distinguer les tiers à l'œil.*

> **⚙ CONCEPTION DÉTAILLÉE 8.3 (écrite AVANT de coder, 19 juin) — pourquoi la conscience NE scale PAS seule.**
>
> **Le mur qui reste (mesurable).** 8.2 a borné le DÉBIT reçu (plat 80↔160). Mais la conscience distribue
> un budget FIXE entre TOUS les pairs lointains : à N=5000, chaque émetteur donne à chaque conscient
> `80 Hz / ~5000 ≈ 0,016 Hz` = **une mise à jour par MINUTE**. Le débit reste plat (bien), mais la
> FRAÎCHEUR par pair lointain s'effondre en 1/N → la « foule lointaine » devient une purée figée, inutile.
> C'est le résidu noté depuis 8.1. **8.3 le règle en remplaçant N flux individuels lointains par
> QUELQUES flux de RÉSUMÉ.**
>
> **L'idée : des CELLULES, chacune avec un HÔTE agrégateur.**
> - Le monde est découpé en **cellules** (grille ; `cell = (floor(x/T), floor(z/T))`).
> - Chaque cellule a un **hôte élu** (réutilise l'élection déterministe + migration de l'orbe : plus petit
>   id parmi les pairs connus DANS la cellule ; relie **D12** « tout est codé pour un objet » → on généralise).
> - L'hôte produit **UN résumé basse fréquence** de sa cellule : nombre d'occupants + quelques positions
>   représentatives (échantillon) — pas les 500 individus. Il le diffuse à qui regarde cette cellule.
> - Un observateur lointain s'abonne aux **cellules** (un flux résumé chacune) au lieu des N individus.
>   Réception = `focus (plein) + voisinage proche (conscience) + C_cellules (résumés)` = **O(K + C),
>   indépendant de N** — l'invariant tenu jusqu'à 5000, ET avec une fraîcheur correcte des lointains.
>
> **Un point clé qui SIMPLIFIE (et qui répond à R2/D11) :** un résumé est **consultatif**, pas autoritaire
> comme l'orbe. Si deux hôtes résument la même cellule (split-brain de migration en grande foule), on a
> juste **deux flux redondants** — un peu de gaspillage, AUCUNE corruption. Donc **8.3 N'A PAS besoin que la
> migration soit durcie d'abord** (D11/ch.11.2) : il TOLÈRE le multi-hôte par nature. (À l'inverse de l'orbe,
> où le split-brain corrompt l'autorité.) On construit donc 8.3 sur l'élection simple existante, sans béton sur du sable.
>
> **Bonus :** l'index spatial des cellules sert AUSSI à choisir le focus en O(K) au lieu du tri O(N log N)
> de `refresh_focus` (la dette du 8.2a-bis) — on ne trie plus que les pairs des cellules proches.
>
> **Sous-étapes prévues :** **8.3a** grille de cellules + `cell_of(pos)` + index « qui est dans quelle
> cellule » (depuis `peer_pos`) — pur, testé. **8.3b** élection/migration d'hôte de cellule (réutilise la
> machinerie orbe, généralisée) — testé en headless. **8.3c** paquet `KIND_CELL_SUMMARY` (occupants +
> représentants) + émission par l'hôte + ingestion (affichage des lointains via résumé). **8.3d** preuve :
> `crowd 500 → 1000 → 2000` en headless → fraîcheur des lointains correcte ET débit ↓ PLAT, + brancher le
> focus sur l'index (vire le O(N log N)).
> **Doutes d'avance :** (a) coût/justice de l'hôte (il bosse pour les autres → relie D4, l'économie du parent,
> Phase B) ; (b) un hôte malveillant ment sur sa cellule (cache/invente des gens → relie D5/D9 ; corroboration 8.8) ;
> (c) granularité de la cellule (trop grosse = résumé grossier ; trop fine = trop de cellules) — à calibrer par la mesure.

- [ ] 8.3 — **Cellules spatiales + hôte de cellule agrégateur (ce qui fait tenir l'invariant
  à 500/5000).** Partitionner le monde en cellules ; chaque cellule a un **hôte élu** (réutilise
  la machinerie d'autorité+migration de l'orbe, D11/D12) qui : connaît grossièrement qui est
  dans la cellule (rendez-vous décentralisé local), et produit UN **résumé basse fréquence**
  de la cellule (positions échantillonnées / densité + quelques représentants) pour les
  observateurs lointains. Ainsi un nœud lointain reçoit **1 flux agrégé par cellule** au lieu
  de N flux individuels → réception en **O(K_focus + C_cellules)**, l'invariant est tenu. *Relie
  D11/D12 (autorité généralisée) et prépare la Phase B (le parent agrège pour le faible).*
  **Preuve :** à `crowd 500` puis `5000`, couverture ~100 % et débit ↓ **plat** (ne croît pas
  avec N) ; on rejoue 8.0 et on montre la courbe couverture(N) ≈ 100 % à débit constant.

> ### ⚙ REDESIGN 8.3★ — PERCEPTION AUTO-CERTIFIANTE (le chef de cellule RETIRÉ) — écrit le 20 juin 2026 (PAPIER, zéro code)
> *Décidé avec l'utilisateur après la passe de mesure du banc bus (cascade-vs-N). On NE patche PAS le contrôle
> d'hôte de D26 : on retire le besoin de chef. À CHALLENGER ensemble avant de coder ; la mesure tranchera.*
>
> **▶ ÉTAT (21 juin) — le papier est devenu de la MESURE :** **C-diag FAIT** (`DENSITY_MAX` — retirer le chef
> RESTAURE la densité : N=1000 → 89 %, taxe 0 %, débit plat ; à 5000 bridé par la découverte = mur n°2).
> **C-sécu-1a + 1b FAITS** (`CORROB` densité molle corroborée /24 + plancher d'union signée → récup **87 % @1000**,
> cible ≥80 % franchie) ; **C-sécu-2 EN COURS** (échantillons auto-signés, étape 1/5 faite, bloc wire à venir).
> État à jour + plan = l'⏱️ ANCRE en tête de §0. Détail sécurité : bloc **« ÉTAPE C-sécu »** plus bas. Journal mesures : `PLAN_AUTONOME.md`.
>
> **CE QUE LA MESURE A ÉTABLI (banc bus, 20 juin).** Deux murs DISTINCTS de la perception à l'échelle :
> - **Mur n°1 (DOMINANT) — la taxe `émetteur≠hôte` de D26 couche 1.** `ingest_summary` n'accepte un résumé que
>   si l'émetteur == `cell_host(cell)` = **le plus petit id connu DANS la cellule** (une *minimalité*). Or à grand
>   N chaque nœud connaît un sous-ensemble DIFFÉRENT → vues de l'hôte DIVERGENTES → rejet de résumés pourtant
>   légitimes. Mesuré (fenêtres ~45-55 s) : taxe **10 % (N=500) → 24 % (1000) → 61 % (2000) → 68 % (5000)** ;
>   perception/N s'effondre **91 % → 64 % → 10 %**. *C'est le suspect initial de l'utilisateur, CONFIRMÉ.*
> - **Mur n°2 (secondaire, orthogonal) — stagnation de découverte au bootstrap**, seulement à ~5000 (~49 pairs
>   pendant ~40 s puis cascade), robuste au jitter d'horloges → propriété réelle du protocole, pas un artefact de
>   banc. **NON traité par ce redesign** (il vise la perception, pas la découverte).
>
> **LA FAUTE D'ÉLÉGANCE D'ORIGINE.** On a emprunté à l'ORBE son modèle d'**autorité par chef unique élu** (plus
> petit id, déterministe). Pour l'orbe c'est *nécessaire* (un maître doit trancher la physique, sinon l'état se
> corrompt). Pour la PERCEPTION il n'y a **rien à trancher** : voir une foule est un **constat**, pas un acte
> d'autorité. On a donc importé un problème de **consensus sur un chef sous vue partielle** — intrinsèquement
> view-dépendant — là où il n'en fallait pas. La taxe 10→68 % EST cette view-dépendance. Aucune rustine (fenêtre
> d'hôtes, K plus petits ids) ne guérit la cause ; il faut **retirer le besoin de chef**.
>
> **LE PRINCIPE ÉLÉGANT — preuve, pas permission (le keystone du projet, un cran plus haut).**
> - **6.1** : l'identité ne dépend pas d'un annuaire de confiance, elle **se prouve** (id = clé).
> - **9.4b** : on ne croit pas une accusation sur parole, on exige de la **corroboration** (diversité d'IP).
> - **8.3★** : un résumé de cellule n'est pas « la parole d'un chef », c'est un **paquet de PREUVES** —
>   un échantillon de **positions SIGNÉES** `(id, pos, seq)` (auto-certifiantes, comme un état joueur). On ne
>   vérifie plus *QUI* envoie (l'émetteur n'est qu'un **porteur d'octets** : « Own ≠ Relais » du README), on
>   vérifie **les signatures dedans**. *C'est aussi le principe n°7 (mesurer le réel > croire une déclaration).*
>
> **LE MODÈLE (à challenger).**
> - Un résumé = `cell` + **K échantillons signés** `(id, x, z, seq, sceau)` (K borné, ~8) + un **indice de
>   densité** `count` (MOU, non autoritaire). **N'importe quel nœud** peut agréger les états signés qu'il a reçus
>   et en relayer un sous-ensemble. Plus d'élection, plus de `cell_host` à l'ingestion.
> - À la réception : pour chaque échantillon, **vérifier le sceau** (auto-certifiant) + **fraîcheur par id**
>   (`accept_seq`, qu'on a déjà) + **TTL** (D16, qu'on a déjà). On **UNIONNE** les échantillons vérifiés reçus de
>   **plusieurs relayeurs indépendants** (diversité /24, comme 9.4b).
> - **Perception** = `|union des gens signés vérifiés, récents, dans la région|` (l'ensemble des « visages » sûrs)
>   **+** la densité molle corroborée (la « marée » en nombre, pour le LOD lointain).
>
> **POURQUOI ÇA DISSOUT LES DEUX MURS DE PERCEPTION (pas patche : dissout).**
> - **Plus de chef → plus de vue divergente → la taxe `émetteur≠hôte` DISPARAÎT** (mur n°1 supprimé).
> - **Inventer des fantômes = IMPOSSIBLE** : chaque personne du résumé porte SA signature (on ne montre que du réel).
> - **Gonfler le compte = IMPOSSIBLE** si la perception se compte sur les échantillons VÉRIFIÉS (pas sur le `count`).
> - **Cacher des gens (éclipse douce) = BORNÉ** : l'**union** d'évidence signée est *monotone* — un trou-noir peut
>   OMETTRE, jamais retrancher à l'union des autres sources. → **la couche 2 (corroboration) est résolue PAR
>   CONSTRUCTION**, dans le même geste. *(D26 couche 1 + couche 2 fusionnées ; le concept « hôte de cellule
>   autoritaire » est RETIRÉ de la roadmap — il restait une scorie de l'orbe.)*
>
> **ANALOGIE PHYSIQUE (pourquoi c'est « vrai »).** Tu crois qu'une foule lointaine existe parce que (a) tu
> reconnais **quelques visages** assez nets pour être sûr, et (b) **plusieurs points de vue** concordent — pas
> parce qu'un porte-parole officiel te l'a dit.
>
> **L'INVARIANT & LE COÛT (preuve papier — à FERMER avant de coder, c'est le make-or-break).**
> - **Bande passante** : K échantillons signés × ~(32+4+4+8+64) ≈ **~112 o/échantillon** → K=8 ≈ **0,9 Ko/résumé**,
>   à basse fréquence, par cellule SUIVIE. Reste **O(cellules), indépendant de N** (l'invariant tient). À chiffrer
>   contre le débit-cible (~40-46 Ko/s mesuré).
> - **CPU (LE mur à fermer)** : vérifs Ed25519 = `C_cellules × K × fréquence`. Ed25519 verify ≈ 20-50 µs. Il faut
>   borner C, K, fréquence pour rester à quelques %/cœur. *Si ça ne ferme pas → l'idée tombe, et on l'aura su gratis.*
> - **Exactitude** : connaître **toute** la foule individuellement reste **O(N)** (incompressible — l'élégance
>   n'efface pas la physique). On l'assume : LOD = **densité molle** (combien) + **échantillon signé TOURNANT**
>   (quelques visages, renouvelés dans le temps et diversifiés entre relayeurs → l'union grossit). On NE prétend PAS
>   percevoir 5000 individus à coût O(cellules).
>
> **MURS POSSIBLES (honnêteté — à examiner sur le papier).** (1) coût CPU des vérifs (ci-dessus) ; (2)
> **échantillonnage représentatif** : si tout le monde échantillonne les mêmes K, l'union ne grossit pas → il faut
> un tirage TOURNANT/diversifié (par relayeur) ; (3) le `count` mou peut être sur-déclaré → ne JAMAIS le compter
> comme perception, seulement comme indice de rendu ; (4) fraîcheur/TTL pour ne pas unioner des partis (on a
> `accept_seq` + D16) ; (5) le format `KIND_CELL_SUMMARY` grossit → touche le wire (prouvable headless, mais à
> re-vérifier en 3D par l'utilisateur) ; (6) n'adresse pas le mur n°2 (découverte).
>
> **PLAN EN PETITS PAS + CRITÈRE DE SUCCÈS PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT).**
> 1. **Papier** : fermer le calcul de coût CPU/bande passante (ce bloc). Si CPU ne ferme pas → STOP, on garde l'idée mais on ne code pas.
> 2. **Prototype headless** (banc bus) : nouveau résumé à échantillons signés + ingestion par union vérifiée, le
>    `cell_host` RETIRÉ de l'ingestion. Additif/derrière un drapeau pour rester un harnais de régression.
> 3. **Mesure (le résultat décide)** : à N=1000/2000/5000, **la taxe `émetteur≠hôte` doit DISPARAÎTRE** et
>    **perception/N doit remonter nettement** (cible : ne s'effondre plus comme 91→10 %), **à débit ↓ PLAT** et
>    **CPU borné** (mesuré, pas argumenté), **sans rouvrir** la forge de fantômes (test red-team : échantillon non
>    signé / rejoué rejeté). Si la perception ne remonte pas OU le CPU explose → l'idée est réfutée, on le note.
> 4. **Anti-rejeu de l'attaque** : un `attack` qui forge un faux résumé (fantômes/gonflage) doit imprimer « ÉCHOUÉ ».

> ### ⚙ REDESIGN 8.3★ — ÉTAPE C-sécu : DENSITÉ MOLLE CORROBORÉE (PAPIER, écrit le 20 juin 2026)
> *Conçu + auto-challengé pendant que les benchs C-diag tournaient (l'utilisateur s'en remet à moi sur ce choix
> très pointu — cf. mémoire). À CHALLENGER encore à la relecture ; la mesure tranchera. Zéro code ici.*
>
> **CE QUE C-diag A ÉTABLI (banc bus, 20 juin) — le pré-requis de ce bloc.** Sous `DENSITY_MAX=1` (count/cellule =
> MAX vu, non-thrashant ; hôte relâché), la DENSITÉ se restaure : N=1000 → perception moy **895 / max 1000** (89 %
> de N), **taxe émetteur≠hôte = 0 %**, débit ↓ plat (~47 Ko/s) ; N=2000 → moy ~1035 (52 %, montait encore, plafonné
> par la découverte = mur n°2). *Donc le mur n°1 (taxe) est bien la cause, et la densité est RÉCUPÉRABLE une fois la
> taxe retirée.* MAIS `DENSITY_MAX` est un **INSTRUMENT non sécurisé** : le MAX est trivialement gonflable (un menteur
> déclare `count` énorme → densité empoisonnée). C-sécu = rendre cette densité SÛRE sans rouvrir le mur n°1.
>
> **LA TENSION À DÉNOUER.** Les deux extrêmes échouent symétriquement : **MAX** robuste à l'omission (un trou-noir
> qui sous-compte ne baisse pas le max) mais cassé par l'**inflation** ; **MIN** robuste à l'inflation mais cassé par
> l'**omission** (un menteur dit 0 → suppression). Pire : entre HONNÊTES, le vrai count ≈ le plus informé → « crois le
> plus haut » ; la sécurité dit « ne crois pas le plus haut ». Il faut un agrégat robuste qui réconcilie les deux.
>
> **LE PRINCIPE (cohérent 6.1 / 9.4b / 8.3★ : preuve + corroboration diversifiée, jamais une parole).** La densité
> finale n'est pas un scalaire qu'on croit — c'est DEUX choses superposées :
> - **(1) PLANCHER dur vérifiable = l'union signée (étape B réutilisée comme borne basse).** `|union des individus
>   SIGNÉS distincts, frais, dans la cellule|`. Infalsifiable (chaque élément porte sa signature), **monotone** : un
>   trou-noir peut OMETTRE, jamais RETRANCHER à ce que J'ai vérifié. Coût **O(cellules)** (échantillon ≤ K/cellule),
>   pas O(N). Faiblesse assumée : sous-compte (~139 à 2000 mesuré) → c'est un plancher de CONFIANCE, pas le chiffre.
> - **(2) DENSITÉ MOLLE corroborée (le vrai chiffre, sécurisé par diversité /24).** Chaque nœud peut publier
>   `(cell, count, signer_id, seq, sig)` — **SIGNÉ** (anti-forge anonyme) mais **SANS élection d'hôte** (c'est ce qui
>   supprime le mur n°1 : plus de `émetteur==cell_host` → plus de vue divergente → plus de taxe). Le receveur garde
>   **un count par /24 distinct** (le meilleur de ce /24, modèle de cap de 9.4b), et **densité estimée = le Q-ième plus
>   grand count parmi les /24 distincts** (un quantile haut borné par quorum, **Q≈3** comme `ACCUSE_QUORUM`).
> - **Densité retenue = `max(plancher vérifié, densité molle corroborée)`** ; **consultative** (LOD/rendu), jamais autoritaire.
>
> **POURQUOI ÇA DISSOUT LES DEUX ATTAQUES (par construction).**
> - **Inflation** : le `count` géant d'un menteur = **1 seul /24** → 1ᵉ plus haut, mais le **Q-ième est inchangé**.
>   Pour le bouger il faut **Q réseaux /24 distincts** → coût = diversité d'IP (ressource rare, 9.4b), pas du CPU gratuit.
> - **Omission / trou-noir** : déclarer 0 ajoute une valeur basse → ne baisse PAS le Q-ième plus haut, ne touche PAS au
>   plancher vérifié. **La couche 2 (corroboration) est résolue dans le même geste.**
> - **Suppression par flood de bas counts** : ajouter des valeurs basses ne déplace jamais un quantile HAUT, et on ne
>   peut pas « dé-signer » les claims hauts des honnêtes → robuste.
> - **Plus d'élection → la taxe émetteur≠hôte ne peut pas réapparaître** (mur n°1 dissous définitivement). Le count
>   signé réintroduit un *signataire* mais **PAS une autorité** (chacun signe le sien, aucune élection) → ce n'est pas
>   le retour du chef de cellule.
>
> **MES DOUTES (à garder ouverts — c'est mon rôle de les porter).**
> 1. **⚠ Le banc bus NE peut PAS prouver la sécurité /24** (loopback = même /24, ports gratuits pour un attaquant — la
>    limite EXACTE de 9.4b). → headless prouve la **récupération honnête** + la *logique* (tests unitaires) ; l'**anti-
>    inflation /24 se prouve sous le harnais NAT namespaces (vraies IP)**, en réutilisant l'infra 9.4b. À ne JAMAIS confondre.
> 2. **Temps de chauffe à découverte sparse** : avoir Q /24 distincts/cellule suppose assez de sources ; pendant le
>    bootstrap (mur n°2) la densité retombe sur le **plancher vérifié** (conservateur) → dégrade vers la PRUDENCE
>    (sous-compte), jamais vers la sur-confiance. Acceptable, à noter.
> 3. **Récupération < MAX** : le Q-ième plus haut est un cran sous le MAX → on perdra un peu des 89 % (prix de la sécurité).
> 4. **Choix de Q** : trop bas (1-2) = inflation trop facile ; trop haut = sous-compte permanent + temps de chauffe long.
>    Q=3 proposé, **à confirmer par la mesure**. Limite fondamentale (comme tout P2P) : un botnet à IP réellement diverses
>    contourne — et un attaquant qui POSSÈDE toute une cellule (tous Sybils) la définit (densité consultative → toléré).
>
> **PLAN EN PETITS PAS + CRITÈRE DE SUCCÈS PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT).**
> 1. **Papier** : ce bloc (fait). Si un trou apparaît à la relecture → STOP, on ne code pas.
> 2. **C-sécu-1 (headless, banc bus)** : count signé `(cell,count,signer,seq,sig)` SANS élection + agrégation
>    « Q-ième plus haut par /24 » + plancher = union vérifiée. *Critère : densité corroborée ≥ **~80 % de ce que
>    `DENSITY_MAX` atteignait** (on ne perd qu'un cran), débit ↓ PLAT, CPU borné.* Tests unitaires : Q-ième plus haut
>    correct, plancher monotone, anti-rejeu par signer (seq).
> 3. **C-sécu-2 (harnais NAT, vraies IP — réutilise 9.4b)** : red-team **inflation** (attaquant multi-Sybil sur 1 /24
>    gonfle des counts → inflation mesurée ≈ 0 au-dessus de l'honnête → `attack` imprime « inflation ÉCHOUÉE ») ;
>    red-team **omission** (trou-noir déclare 0 → densité perçue par les honnêtes INCHANGÉE).
> 4. Si la récupération s'effondre OU le CPU explose OU l'inflation passe → l'idée est réfutée, on le note (résultat négatif = résultat).

**— Phase B : l'inclusivité, maintenant que la foule est visible (ferme D3, D4, D5) —**

- [ ] 8.4 — **Budget de réception annoncé (water-filling BILATÉRAL).** Chaque joueur publie
  son débit descendant soutenable + son rayon d'intérêt ; les émetteurs en tiennent compte
  (le water-filling, jusqu'ici unilatéral côté émetteur, devient bilatéral). Ferme D3.
- [ ] 8.5 — **Dégradation gracieuse côté receveur** : au-delà du budget, on baisse la
  fréquence des lointains AVANT les proches (paliers focus / proche / foule) — la conscience
  (8.2) se raréfie avant le focus.
- [ ] 8.6 — **Parent agrégateur pour très faibles** : le parent (ou l'hôte de cellule de 8.3)
  reçoit le voisinage et n'envoie au protégé qu'un résumé basse fréquence. Le 0-connexion joue
  *via* son parent. *Réutilise 8.3 ; ferme la moitié « réception bornée » de D3.*
- [ ] 8.7 — **Économie du parent (anti free-riding)** : réciprocité façon BitTorrent
  (choking / optimistic unchoke pondéré par la réputation). Ferme D4.
- [ ] 8.8 — **Anti-censure du parent / hôte de cellule** : multi-parents + détection du « trou
  noir » ; un hôte qui CACHE une partie de la foule est repéré (corroboration gossip de 8.1) et
  contourné. Ferme D5 ; relie D9.

**Ferme :** D22, D23, D24, D3, D4, D5 (+ amorce D9 et D10, relie D11/D12, amorce D16/D17).
**Vérif globale :** à `crowd 200+` (puis 500, 5000), un joueur perçoit ~100 % de la foule
(focus net + lointains dégradés), son débit ↓ reste **borné et plat** quand N grandit, AUCUN
plafond dur à 32, et un hôte/parent malveillant ne peut ni cacher la foule ni couler le faible
(sous netem throttlé à 5 Ko/s, le faible reste fonctionnel ; un nœud égoïste est servi en dégradé).
Un essaim d'attaquants `gossip-flood` est absorbé (D23). **Et dans le VRAI jeu 3D** (lancé par
l'utilisateur, plusieurs fenêtres), on voit bien plus que 64 silhouettes sans chute de FPS (D24).

### Chapitre 9 — Durcissement de la confiance (Sybil, éclipse, rendez-vous) 🔴 *priorité 3*
**But :** rendre la triche *coordonnée* coûteuse et l'isolement impossible.
> **État (19 juin) :** ✅ 9.1a (PoW réglable) · ✅ 9.2 (quorum pondéré par crédibilité) · ✅ 9.3 (réhabilitation) ·
> ✅ 9.4a (positions corroborées) · ✅ 9.4b (diversité IP) · ✅ 9.5a (rendez-vous borné). **Le cœur dur est tenu.**
> Les parties avancées **9.1(b) adaptative, 9.2c, 9.5-fédération, vouching → reportées en ANNEXE H (optionnel)**.
- [ ] 9.1 — **Refonte anti-Sybil** : difficulté PoW bien plus haute + adaptative ; étude
  d'un second facteur (vouching social). Ferme D6.

> ### 🧭 CARREFOUR D'ARCHITECTURE 9.1 — comment rendre une identité « chère » (écrit AVANT de coder, 19 juin)
> *Réversible. On note les trois pistes pour pouvoir y revenir ; rien n'est figé. État actuel du code :
> `crypto::POW_BITS = 16`, une **constante de compilation GLOBALE et figée** — le « jouet » de D6. Une
> identité coûte ~2¹⁶ essais ≈ instantané → 3 identities Sybil suffisent à frame un honnête via le quorum
> d'accusation (6.7, `ACCUSE_QUORUM = 3`). C'est CE trou que 9.1+9.2 referment.*
>
> **La tension de fond (à garder en tête) :** anti-Sybil et **inclusivité tirent en sens opposés**. Plus on
> rend l'identité chère (CPU), plus on punit le joueur FAIBLE (un téléphone qui doit miner 2²⁴ avant de
> jouer) — ce qui contredit la vision « du 0-connexion à la fibre, chacun joue » (D3/D4, la Phase B du ch.8
> qu'on vient de mettre en pause). Une PoW dure mal pensée *referme* l'inclusivité par la fenêtre. Le bon
> design doit donc être **cher pour l'attaquant en masse, mais léger pour l'honnête isolé**.
>
> **Piste (a) — PoW FIXE mais bien plus haute.** On monte simplement `POW_BITS` (ex. 20–24) et on le rend
> réglable (plus une constante de compilation). *Pour :* trivial, aucun mécanisme nouveau, aucun accord
> entre nœuds à trouver. *Contre :* taxe à PLAT — même prix pour l'attaquant et pour le téléphone du
> débutant ; ne s'adapte pas à une attaque (toujours trop cher en temps calme, ou trop peu sous assaut).
> → **rôle naturel : un SOCLE (prix plancher minimal), pas la réponse entière.**
>
> **Piste (b) — PoW LOCALEMENT ADAPTATIVE.** Chaque nœud, en bon « Shield », relève le prix d'admission
> qu'il EXIGE des autres selon la pression LOCALE qu'il observe (cadence de nouvelles identités, taux
> d'accusations…). *Pour :* pas de serveur ni de consensus global (un nœud calme exige peu, un nœud sous
> assaut exige plus) ; la PoW est *monotone* — miner 24 bits satisfait quiconque demande ≤ 24, donc un
> honnête mine juste au plus haut niveau exigé là où il veut aller, et ne paie cher QUE pendant une
> attaque. *Contre :* la difficulté devient relative au nœud ; un nœud (ou un faux rendez-vous) qui exige
> une PoW absurde peut exclure les nouveaux → c'est un vecteur d'ÉCLIPSE, à traiter avec 9.4/9.5.
> → **rôle naturel : la DÉFENSE STRUCTURELLE (le prix monte tout seul sous attaque, redescend au calme).**
>
> **Piste (c) — VOUCHING SOCIAL (second facteur).** Un pair déjà connu se porte GARANT d'un nouveau → le
> coût devient SOCIAL (une relation), pas CPU. *Pour :* ami des faibles (un téléphone avec un ami entre
> sans miner) ; et le graphe de garants résiste aux Sybils par sa STRUCTURE (un essaim de faux comptes
> peine à se faire parrainer par des honnêtes — façon SybilGuard/SybilLimit). *Contre :* amorçage (le tout
> premier joueur n'a pas de parrain → il faut une voie « sans parrain » qui retombe sur (a)/(b)) ; c'est un
> sous-système plus gros. → **rôle naturel : un SECOND FACTEUR ajouté plus tard, qui PRÉSERVE l'inclusivité
> (relie la Phase B du ch.8).**
>
> **💡 L'insight : ce ne sont PAS trois rivales, ce sont trois COUCHES.** Le design solide est probablement
> *(a) socle plancher + (b) adaptatif structurel maintenant (ch.9) + (c) vouching en second facteur plus
> tard (à la reprise de la Phase B du ch.8)*. On ne « choisit » donc pas une voie contre les autres : on
> choisit **dans quel ORDRE** on pose les couches. *Décision concrète (combien de bits, quel signal de
> pression pour (b)) prise APRÈS l'étape de preuve ci-dessous — la mesure tranchera, pas ce document.*
>
> **Premier pas (avant tout codage de correctif) — ✓ FAIT (19 juin) :** PROUVER le trou — attaque rouge
> `attack sybil-frame` (3 identités Sybil minées en ~2,1 s qui accusent un innocent → témoin honnête le met
> en sourdine), comme `gossip-flood` a prouvé D23. **Résultat : « FRAMING RÉUSSI ».** La menace est nette ;
> l'attaque sert maintenant de harnais de régression (elle imprimera « framing ÉCHOUÉ » une fois (a)/(b) posées).
- [ ] 9.2 — **Quorum d'accusation pondéré** : par réputation de l'accusateur + plausibilité
  de voisinage ; K attaquants ne peuvent pas framer un honnête. Ferme D7, D20.

> ### ⚙ CONCEPTION 9.2 — le « TÉMOIN CRÉDIBLE » (écrit avant de coder, 19 juin ; FAIT en 1re couche)
> *L'attaque `sybil-frame` l'a prouvé : compter des TÊTES distinctes est naïf — 3 identités bon marché
> suffisent. La réponse : ne pas COMPTER les accusateurs mais SOMMER leur POIDS de crédibilité, et ne bannir
> qu'au-delà d'un seuil (`ACCUSE_WEIGHT_QUORUM`). Un accusateur ne pèse que s'il est un TÉMOIN PLAUSIBLE.*
>
> **Poids d'un accusateur (1re couche, codée) = STANDING × co-localisation :**
> - **Standing (le verrou anti-Sybil-conjuré) :** l'accusateur m'a-t-il déjà envoyé un VRAI état signé que
>   j'ai accepté ? (= a-t-il une entrée dans `replay` ?) Un Sybil fraîchement miné qui ne fait que CRACHER des
>   accusations n'a JAMAIS participé au monde chez moi → **poids 0**. Pour peser, il faut avoir été un acteur
>   observé (coûteux à entretenir, et visible). *C'est ce qui ferme `sybil-frame` : les Sybils n'ont pas de standing.*
> - **Co-localisation (plausibilité de voisinage) :** si je connais les positions de l'accusateur ET de
>   l'accusé et qu'elles sont à portée (`WITNESS_RADIUS`), poids plein (1.0) — il a pu VOIR la triche ; sinon
>   poids plancher réduit (`WITNESS_FLOOR`) — établi mais témoin lointain, il compte un peu, pas plein.
> - **Seuil :** `ACCUSE_WEIGHT_QUORUM = ACCUSE_QUORUM` (3.0) → il faut ~3 témoins crédibles co-localisés,
>   OU beaucoup plus d'établis-lointains, et **AUCUN** Sybil conjuré ne contribue. *Dégradation gracieuse.*
>
> **Preuve :** re-jouer `attack sybil-frame` → doit basculer de « FRAMING RÉUSSI » à « framing ÉCHOUÉ ».
> **Doutes honnêtes (résidus, couches suivantes) :** (a) un attaquant PATIENT qui fait VIVRE ses Sybils comme
> de vrais participants (ils envoient des états, gagnent du standing) puis les co-localise avec la victime peut
> encore peser → durci par **9.2c** (standing par DURÉE/quantité, pas binaire) et surtout par **9.4**
> (corroboration des positions : les positions de gossip sont non vérifiées, D9). (b) un témoin honnête dont je
> n'ai jamais reçu d'état (lointain) ne compte pas chez moi → acceptable : la réputation se propage entre
> voisins réellement connectés. La 1re couche ferme le framing BON MARCHÉ ; le patient/coordonné = 9.2c+9.4.
- [ ] 9.3 — **Réhabilitation** : fenêtre glissante des fautes + appel/quarantaine. Ferme D8.
- [ ] 9.4 — **Anti-éclipse** : diversité forcée du voisinage (proches + aléatoires
  vérifiés, façon Kademlia) + corroboration des positions. Ferme D9.

> ### ⚙ CONCEPTION 9.4 — corroboration des positions + diversité du voisinage (écrit avant de coder, 19 juin)
> **9.4a — corroboration des positions ✓ FAIT.** *Le levier (relevé en lisant le code) :* `peer_pos` mêlait
> des positions SIGNÉES (depuis l'état d'un pair) et des positions de GOSSIP (revendiquées par un TIERS,
> falsifiables). Or la crédibilité 9.2 lisait `peer_pos` → un attaquant pouvait **gossiper qu'un témoin est
> "collé" sur la victime** pour fabriquer une fausse co-localisation et regagner du poids (le résidu patient
> de 9.2). *Le fix :* champ séparé `confirmed_pos`, écrit UNIQUEMENT par `note_pos` (donc depuis un état SIGNÉ
> du pair lui-même) ; le gossip n'y touche jamais. `accusation_weight` lit `confirmed_pos`, pas `peer_pos`
> (qui reste l'indice ouvert de découverte/AoI). *Preuve :* nouveau test `gossip_ne_peut_pas_falsifier_la_co_
> localisation_pour_framer` (témoins établis mais réellement loin ; gossip qui ment « à (0,0) » → pollue bien
> `peer_pos` mais PAS `confirmed_pos` → poids plancher → innocent intact). 55 tests, 0 warning ; non-régression
> `sim 40 3 15` → 21 sourdines (vrais tricheurs neutralisés), orbe 0/40, essaim tenu.
>
> **9.4b — diversité de RÉSEAU (anti-éclipse) ✓ FAIT — ⚠ CHALLENGE de la feuille assumé.** *La menace :* un
> attaquant qui co-localise pour de vrai des Sybils établis près d'une victime (le résidu COÛTEUX de 9.4a) réunit
> un quorum de « témoins » et la frame quand même. *Le challenge (important) :* la piste initiale « diversité d'id
> façon Kademlia (XOR) » est le MAUVAIS outil DANS NOTRE MODÈLE — une identité étant une clé PoW ~aléatoire, les
> Sybils se répartissent dans les buckets EXACTEMENT comme les honnêtes → la diversité d'id ne distingue pas
> l'attaquant. *Le bon levier (celui de Bitcoin/Ethereum) = diversité d'IP :* un attaquant mine des ids gratis
> mais n'a qu'une POIGNÉE d'adresses IP. *Le fix codé :* `record_accusation` CAPE la contribution par
> SOUS-RÉSEAU /24 (`subnet_key`) à ≤ 1 témoin effectif → le quorum (3.0) exige des témoins de ≥3 RÉSEAUX
> distincts ; mille Sybils derrière une IP = 1 voix. *Détail loopback :* en simu localhost (tous 127.0.0.1) on
> distingue par PORT (vrais process séparés) → la réputation légitime n'est pas cassée ; le /24 ne s'applique
> qu'aux vraies IP. *Preuve :* test `sybils_d_un_meme_sous_reseau_ne_font_pas_quorum` (5 Sybils co-localisés mono-IP
> → pas de sourdine ; les mêmes sur 5 /24 → sourdine). 56 tests, 0 warning ; non-régression `sim 40 3 15` → 40
> sourdines, orbe 0/40, couverture 100 %. *Résidus : la simu localhost ne peut pas EXERCER le /24 (→ harnais NAT
> namespaces, vraies IP) ; un attaquant à IP réellement diverses (botnet) contourne — limite fondamentale, comme
> tout système P2P. Diversité réseau aussi applicable à la table de pairs / au focus (anti-éclipse général) = suite.*
- [ ] 9.5 — **Rendez-vous résilient** : rate-limit + éviction + fédération (2+ rendez-vous
  qui s'échangent des pairs) ; amorce d'une découverte par gossip. Ferme D10, D21.
**Ferme :** D6, D7, D8, D9, D10, D20, D21. **Vérif :** scénario d'attaque combinée en simu
(Sybil + éclipse + framing) → l'essaim tient.

### Chapitre 10 — Identité persistante & vie privée 🔴 *priorité 4*
**But :** un vrai « compte » décentralisé, et de la confidentialité.
- [x] **10.1 — Identité persistante (20 juin) — FERME D14.** Clé minée UNE fois puis sauvée dans
  `~/.web3game/<profil>.key` (perms 600, comme une clé SSH), rechargée au lancement. `NetLink::new`
  (éphémère) reste INTACT pour la simu/les bots ; nouveau `NetLink::new_persistent(color, weak, profil)`
  pour le vrai jeu, profil = le mode (`a.key` ≠ `b.key` → deux fenêtres restent distinctes ET stables
  entre sessions). `crypto::load_or_create_in` (pure, testable) + `from_secret`/`secret`. *Prouvé :*
  2e lancement RECHARGE la même identité (pas une neuve), profils distincts = identités distinctes.
  68 tests, 0 warning. *(Chiffrement du fichier par mot de passe = plus tard, avec 10.2 ; pour l'instant
  clé en clair sur disque, comme `~/.ssh/id_ed25519` sans passphrase — décision G.4.)*
- [ ] 10.2 — **Chiffrement de transport** : échange X25519 par paire → contenu chiffré +
  signé. Ferme D15.
**Ferme :** D14, D15. **Vérif :** relancer garde l'identité & la réputation ; un tiers ne
peut plus lire positions/voix.

### Chapitre 11 — Autorité généralisée & ordre 🟠
**But :** sortir de l'orbe-unique, durcir la migration, arbitrer le temps.
- [ ] 11.1 — **Registre d'objets autoritaires** générique (réutilise la machinerie orbe).
  Ferme D12.
- [ ] 11.2 — **Migration par quorum** (élection confirmée + présence prouvée + grâce).
  Ferme D11.
- [ ] 11.3 — **Horodatage signé + synchro d'horloge légère** pour arbitrer les courses.
  Ferme D13.
- [ ] 11.4 — **Anti speed-hack fin** (vitesse + accélération + cohérence). Ferme D18.
**Ferme :** D11, D12, D13, D18. **Vérif :** 1000 objets + courses arbitrées de façon
cohérente par tous les nœuds.

### Chapitre 12 — Robustesse, longévité, unification 🟠
**But :** que ça tienne des heures, et que le code ne dérive pas.
- [ ] 12.1 — **Éviction/TTL** des pairs absents (mémoire stable). Ferme D16.
- [ ] 12.2 — **Unifier bot/jeu** : un cœur de session partagé. Ferme D2.
- [ ] 12.3 — **Relais TURN décentralisés + IPv6** (NAT symétrique). Ferme D17.
- [ ] 12.4 — **Découper le résumé v2 > MTU** (foresight 22 juin, ci-dessous). Préalable à l'échelle V2.
**Ferme :** D2, D16, D17. **Vérif :** simu longue (mémoire stable) ; cas NAT symétrique OK.

> ### ⚙ FORESIGHT « gros paquets à l'échelle » — MESURÉ le 22 juin 2026 (à la demande de l'utilisateur)
> *« obtenir 5K connectés et voir si de trop gros paquets ne deviennent pas horribles ». Réponse mesurée
> (test `foresight_taille_paquets_vs_mtu` dans `cell.rs`, 91 tests, 0 warning), pas argumentée.*
>
> **Bonne nouvelle — AUCUN paquet ne grandit avec N.** Tout est plafonné par des constantes
> (`MAX_NEIGHBORS=32`, `MAX_CARDS=16`, `MAX_CELL_SAMPLES=16`) → **5K ou 50K ne gonfle PAS les paquets**,
> la taille est bornée par design. L'intuition « à 5K ça explose » est levée sur ce point précis.
>
> **Taille MAX par type (mesurée / calculée des encodeurs), vs MTU UDP sûr ~1200 o :**
>
> | Paquet | Taille max | Croît avec N ? | vs MTU |
> |--------|-----------|----------------|--------|
> | STATE / RELAY / RELAY_FWD / PUNCH / HELLO | 118 / 182 / 216 / 34 / 42 o | non (fixe) | ✅ |
> | WELCOME (cap 32 voisins) | **1221 o** | non (capé) | ✅ proche du bord |
> | GOSSIP (cap 16 cartes) | ~740 o | non (capé) | ✅ |
> | CELL_SUMMARY v1 (cap 16 samples) | **759 o** | non (capé) | ✅ |
> | **CELL_SUMMARY_V2** (cap 16 preuves ×182) | **3672 o** | non (capé) | ❌ **~×2,6 le MTU** |
>
> *⤷ MAJ 12.3-G : `RELAY_FWD` n'est plus « fixe 216 o » — son payload est désormais VARIABLE (avatar
> 216 o, orbe 170 o). Toujours bien sous le MTU (le plus gros payload relayé prévu = l'état 182 o). Le
> seul paquet > MTU reste le résumé v2 ci-dessous. Voir le PAPIER-WIRE 12.3-G plus bas.*
>
> **LE SEUL vrai risque : le résumé v2 (~3,6 Ko) DÉPASSE le MTU**, indépendamment de N → IP le fragmente
> en ~3 morceaux ; en UDP, **un fragment perdu = tout le paquet jeté** → à l'échelle, avec de la perte,
> c'est exactement le « horrible » pressenti. **Nuance :** v2 n'est émis QUE sous le drapeau
> `SIGNED_SAMPLES` (défaut = v1 intact, V2 pas sur le fil) → le **chemin par défaut n'a AUCUN paquet >
> MTU** aujourd'hui (le plus gros = WELCOME à 1221 o). Mais v2 est le format que C-sécu-2 a fermé et
> qu'on voudra utiliser → **dette latente, à régler AVANT de scaler (→ tâche 12.4).**
>
> **Ce que je tranche pour 12.4 :** découper le **trailer de preuves en paquets séparés** (chaque preuve
> = 182 o, DÉJÀ auto-certifiante → voyage seule, unionnée à la réception). Le code le permet déjà (cf.
> test « relais peut retirer des preuves » : trailer hors corps signé, sous-ensemble tournant). Alternative
> plus simple si besoin : plafonner à ~3 preuves/paquet (759+1+3×182 = 1306 o, sous MTU). *Pas urgent
> (hors chemin défaut), mais tracé pour ne pas le découvrir le jour de l'échelle.*

> ### ⚙ PAPIER 12.3 — LE RELAIS (D17) — écrit le 22 juin 2026 (PAPIER, zéro code)
> *Méthode : comme C-sécu, on pose le mur SUR LE PAPIER avant de coder. Le drapeau gate tout,
> le chemin par défaut reste byte-pour-byte intact, 0 warning. On écrit le critère AVANT de coder.*
>
> **CE QUE LA MESURE PROUVE VRAIMENT (et ce qu'elle NE prouve pas).** Test du 22 juin : A (Windows,
> partage mobile, NAT symétrique/CGNAT) + B (NixOS maison) se trouvent via le rendez-vous public ✅,
> mais le perçage direct échoue des deux côtés ❌. **ATTENTION — le test a un confound de topologie**
> qu'on ne blanchit pas : B était derrière la **MÊME box que le serveur** → vu en hairpin comme
> `192.168.1.254` (adresse privée). Donc deux échecs de **nature différente** ont été mélangés :
>
> | Pair | Échec observé | Nature réelle |
> |------|---------------|---------------|
> | **A** (mobile/CGNAT) | port ne correspond pas | **VRAI mur** — NAT symétrique, fréquent sur mobile |
> | **B** (même box que serveur) | adresse privée `192.168.1.254` | **ARTEFACT** de topo, PAS un mur de NAT |
>
> Conséquence honnête : on ne peut PAS encore écrire « la majorité ne pourra pas se connecter » — la
> moitié de l'échec est un artefact. Ce qui est prouvé : **le NAT symétrique (A) exige un relais.** Le
> reste est NON-MESURÉ. → Le relais est donc cadré comme **REPLI** (pour la minorité non-perçable),
> **pas comme chemin nominal**. Cela rend la centralisation v1 (le rendez-vous relaie) acceptable :
> elle est hors du chemin par défaut, pas sur chaque connexion.
>
> **⚠ ÉTAPE 0 PRÉ-ENREGISTRÉE (avant tout code) — UNE mesure propre, ~0 code.** Remettre **B sur un
> autre réseau que la box du serveur** (2e partage mobile, ou un ami ailleurs) et relancer le test.
> Question tranchée par cette mesure : *un NAT non-symétrique perce-t-il, lui ?* 
> - Si **oui** → le relais reste un repli pour la minorité symétrique (cadrage ci-dessus confirmé).
> - Si **non** (même les NAT classiques échouent) → le relais redevient chemin quasi-nominal, et la
>   centralisation v1 devient un vrai problème à re-discuter AVANT de coder le repli.
> *C'est la mesure qui n'a jamais tourné proprement faute de 2e réseau. Elle dimensionne le chantier.*
>
> **QUI RELAIE ? (la décision d'archi).**
> - **v1 pragmatique — le RENDEZ-VOUS relaie**, derrière un flag/mode, JAMAIS le défaut. Les deux pairs
>   l'atteignent déjà (seul point public commun) → chemin le plus court vers « deux humains se voient ».
>   ⚠ Casse la propriété « rendez-vous tuable » + en fait un goulot → **acceptable SEULEMENT en repli**,
>   à décentraliser ensuite. *On part de là.*
> - **v2 décentralisé — un PAIR bien connecté relaie** (TURN-like P2P). Aligné avec la vision sans-serveur,
>   mais bute sur **D4 (incitation au relais)** déjà ouvert. Plus tard.
>
> **LE HOOK PRÉCIS (le primitif existe déjà, on le RÉUTILISE).** Le repli se branche sur
> `punch_abandoned(tries)` ([`src/net/punch.rs`](src/net/punch.rs)) : après `PUNCH_GIVEUP` (40 essais
> ~10 s) le perçage est ABANDONNÉ et plus rien ne se passe → **c'est LÀ qu'on route via relais.** Et le
> primitif « porter un état signé d'autrui sans pouvoir le forger, à débit borné » est **déjà codé et
> durci** : `mark_as_relay`/`KIND_RELAY` (le sceau tient → le relayeur ne peut que recopier les octets)
> + `RELAY_RATE`(30)/`RELAY_CAP`(60)/`MAX_RELAY_FANOUT`(12) (anti-amplification). Le chantier = réutiliser
> ce primitif pour le cas « perçage échoué entre A et B », pas seulement « upload faible ».
>
> **FORMAT (esquisse, à figer à l'étape papier-wire).** Quand A abandonne le perçage vers B : A demande
> au point public commun (v1 = rendez-vous) « relaie-moi vers B ». Le rendez-vous, en mode relais,
> applique le **MÊME forwarding borné que `KIND_RELAY`** (vérifie le sceau, recopie, plafonné). L'état de
> jeu relayé reste **signé bout-en-bout** → le relais ne peut que porter, jamais falsifier (sécu OK par
> construction, rien de neuf à prouver côté confiance).
>
> **COÛTS À CHIFFRER (mesure, pas argument).**
> - **Latence → fraîcheur (b).** Un saut de plus mord directement sur la fraîcheur. Mesurer : âge de
>   perception **direct** (quand ça perce) vs **relayé**, en ms. *C'est la 1re fois qu'on pourra chiffrer
>   (b) en conditions réelles — le perçage échoué l'a toujours empêché.*
> - **Bande passante du relais.** Il porte du trafic de jeu (positions ~20 Hz), plus juste des adresses.
>   Trivial à 2 ; à **borner** à l'échelle (un relais ≠ 55K) — les bornes `KIND_RELAY` s'appliquent.
> - **Centralisation.** Le rendez-vous-relais redevient un serveur. Acceptable en v1 **car repli**, à
>   décentraliser en v2.
>
> **⚠ RECADRAGE (22 juin, avec l'utilisateur) — la fraîcheur (b) N'EST PAS prioritaire.** Le « vivant »
> ne se mesure pas, il se FABRIQUE : l'interpolation/extrapolation côté client (`smooth_damp`, déjà là)
> rend un mouvement prédit+lissé souvent PLUS vivant qu'un 0 ms brut mais saccadé. Donc on ne vise pas le
> 0 ms et le RTT n'est PAS une barrière. On le **logue pour info** (borne de sécu), point. La make-or-break
> du relais redevient **purement BINAIRE**.
>
> **CRITÈRE PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT de coder).** Deux pairs qui NE peuvent PAS se percer
> (perçage abandonné des **deux** côtés, vérifié dans les logs) **se voient néanmoins BOUGER** via le
> relais. (RTT relayé loggé pour info, jamais un gate.) C'est
> le test qui n'a jamais pu tourner faute de connexion : le relais le débloque. *Échec admis si : pas de
> mouvement relayé, ou fraîcheur si dégradée qu'elle casse le game-feel (seuil à fixer dans le papier-wire).*
>
> **PETITS PAS (cadence intouchable).**
> 1. **Étape 0** (ci-dessus) : 1 mesure propre B-ailleurs (~0 code) → dimensionne le repli.
> 2. **Papier-wire** : format exact de la demande de relais + seuil de fraîcheur acceptable.
> 3. **Repli minimal codé derrière un flag** (v1 = rendez-vous relaie une paire qui a abandonné), défaut
>    byte-pour-byte intact, 0 warning, compile → test → preuve headless → commit → push.
> 4. **Preuve réelle** A (mobile) ↔ B : ils se voient bouger + fraîcheur (b) chiffrée en ms.
> *Ferme D17 en v1 (repli centralisé) ; v2 décentralisé (D4) reste un chantier ultérieur.*

> ### ⚙ PAPIER-WIRE 12.3 — le format exact + le seuil de fraîcheur — écrit le 22 juin 2026 (PAPIER, zéro code)
> *Étape 1 du PAPIER 12.3. On fige le format AVANT de coder. Lu dans le vrai code : `wire.rs` (les
> `KIND_*`, dernier = 10), `message.rs` (état scellé 182 o = `SIGNED_STATE_SIZE`, `mark_as_relay` ne
> change QUE le 1er octet), `rendezvous.rs` (présentateur pur : HELLO→WELCOME, ne relaie RIEN
> aujourd'hui), `receive.rs` (`KIND_RELAY` existant = relais BROADCAST « upload faible », fanout 12).*
>
> **CONSTAT QUI DÉCIDE LE FORMAT.** Le `KIND_RELAY` existant ne convient PAS au cas D17 : il est
> *broadcast* (le parent recopie l'état à TOUS ses voisins) et *sans destinataire* (le paquet ne porte
> que l'id de l'ÉMETTEUR). Le cas NAT est *unicast* : A veut joindre **B précisément**, via le seul
> point public commun (le rendez-vous, v1). Il faut donc **porter le destinataire** dans le paquet et
> apprendre au rendez-vous à **router** (il ne sait aujourd'hui que présenter). → nouveau KIND dédié.
>
> **LE FORMAT (figé) — `KIND_RELAY_FWD = 11` (prochain libre après 10) :** une simple ENVELOPPE de
> routage autour de l'état déjà scellé. On ne re-signe rien (on réutilise le sceau bout-en-bout).
> ```
> [0]                     KIND_RELAY_FWD (11)
> [1]                     PROTO_VERSION
> [2..34]                 dest_id : clé publique du DESTINATAIRE (B). ROUTAGE seul, NON signé.
> [34..216]               l'état KIND_STATE SCELLÉ de l'émetteur (A), VERBATIM (182 o = SIGNED_STATE_SIZE)
> ```
> Taille = 2 + 32 + 182 = **216 o**. Le rendez-vous (en mode relais) : lit `dest_id`, retrouve l'adresse
> de B dans sa table de clients récents (reverse-lookup id→addr), **vérifie le sceau interne** (`sig_ok`
> — refuse de relayer du bruit = anti-amplification), puis renvoie les **182 o internes tels quels** (ils
> sont DÉJÀ en forme `KIND_STATE`) à l'adresse de B. B reçoit un `KIND_STATE` normal, vérifie le sceau,
> voit A bouger. **Symétrique :** B fait pareil (dest=A) → relais bidirectionnel, un `KIND_RELAY_FWD` par
> sens. **Fanout = 1** (unicast) → strictement plus sûr que le `KIND_RELAY` broadcast (ratio ≤ 1, jamais
> amplificateur). Bornes réutilisées : rate-limit par source (déjà là) + budget relais façon
> `RELAY_RATE`/`CAP` ; dest doit être un **client récent** (vu < 5 s). *Sécu : le rendez-vous-relais ne
> peut que PORTER des octets signés, jamais forger — exactement la propriété de `mark_as_relay`.*
>
> **DEUX DRAPEAUX, PAR DÉFAUT À ZÉRO (le défaut reste byte-pour-byte intact).**
> - Côté rendez-vous : `RENDEZVOUS_RELAY=1`. Éteint → présentateur pur, « tuable », ne route rien (état
>   actuel inchangé). Allumé → comprend `KIND_RELAY_FWD`.
> - Côté client : `RELAY_FALLBACK=1`. Éteint → `punch_abandoned` reste un **no-op** (comportement
>   actuel exact). Allumé → quand le perçage est abandonné vers un pair, on emballe notre état scellé en
>   `KIND_RELAY_FWD(dest=ce pair)` et on l'envoie à l'adresse du rendez-vous. *Le hook est déjà là :
>   `punch_abandoned(tries)` dans `punch.rs`.*
>
> **LA FRAÎCHEUR (b) — RÉTROGRADÉE (22 juin, décision utilisateur), loggée pour info, JAMAIS un gate.**
> Le « vivant » se FABRIQUE (interpolation/extrapolation côté client, déjà en place) — souvent plus
> vivant qu'un 0 ms saccadé. On ne vise donc pas une cible de ms. On mesure le **RTT relayé** (horloge
> LOCALE de A : ping relayé horodaté, B le réfléchit, A calcule) **uniquement pour le journal** — borne
> de sécu, pas critère. Le seul juge de « est-ce acceptable », c'est le RESSENTI à l'écran. Make-or-break
> du relais = **binaire** (ils se voient bouger, oui/non).
>
> **MES DOUTES (transparence, pas devoirs — à flairer) :**
> - `dest_id` non signé : si on le falsifie, l'état (toujours scellé, infalsifiable) de A part juste vers
>   le MAUVAIS pair, qui vérifie le sceau et affiche A. Pas de forge d'état possible ; fanout 1 ;
>   rate-limité ; dest doit être un client récent. Pire abus = se servir du rendez-vous comme réflecteur
>   1:1 entre deux clients déjà inscrits → borné (ratio ≤ 1, pas un amplificateur). **Jugé acceptable.**
> - Centralisation : le rendez-vous-relais reporte du trafic de JEU → c'est un serveur à nouveau. Assumé
>   en v1 **parce que REPLI** (hors chemin par défaut). À décentraliser en v2 (bute sur D4).
> - Le confound hairpin (B même box que le serveur) reste NON mesuré → **étape 0 d'abord** (mesure propre
>   B-ailleurs) pour savoir si le repli sert une minorité ou (presque) tout le monde.
>
> **✅✅ D17 BIDIRECTIONNEL PROUVÉ EN RÉEL (22 juin soir) — la forteresse n'est plus vide (D27).**
> Deux humains, deux vrais réseaux (A = Windows/mobile NAT symétrique ; B = NixOS/maison LAN), qui NE
> peuvent PAS se percer, **se voient bouger via le relais qu'on a écrit**. Preuve = le log du rendez-vous
> (juge neutre) montre **les DEUX sens** : `🔀 00000b4b → 000010d6` (B→A) ET `🔀 000010d6 → 00000b4b` (A→B).
> *C'est la 1re fois que le code fait entrer deux humains réels dans le même espace via Internet.*
> - ✅ **Pas 1 — routage rendez-vous** (`RENDEZVOUS_RELAY`, défaut OFF) : décode `KIND_RELAY_FWD`, vérifie
>   le sceau interne, route vers le destinataire, à débit borné. + log diagnostic « 🔀 RELAIS établi X→Y ».
> - ✅ **Pas 2 — émission client** (`RELAY_FALLBACK`, défaut OFF) : au perçage abandonné, `net_send` relaie
>   via le rendez-vous. + le client annonce « Repli relais : ACTIF » au démarrage.
> - ✅ **BUG trouvé EN RÉEL (introuvable en headless) :** `ingest_state` ouvrait le trou direct même sur un
>   état RELAYÉ → A se croyait connecté en direct, n'abandonnait jamais, ne relayait jamais en retour
>   (asymétrie : A voyait B, B ne voyait pas A). Fix : n'ouvrir le trou que si `from != rendezvous`. *Aucun
>   test sur `lo` ne pouvait le voir (le perçage y réussit) — la valeur d'un humain dehors, prouvée.*
> - **Ce que ça NE prouve PAS :** v1 = rendez-vous-relais CENTRALISÉ (repli, pas décentralisé — v2/D4 reste) ;
>   testé à 2 pairs (pas l'échelle) ; la fraîcheur (b) RTT non chiffrée (rétrogradée, optionnelle) ; B était
>   sur le LAN du serveur (pointé en LAN direct → pas de hairpin, mais pas un 3e réseau distinct non plus).
> - **Reste (plus tard) :** v2 relais décentralisé (D4) ; relais à l'échelle (bucket partagé par source) ;
>   étape 0 « B sur un 3e réseau » si on veut chiffrer le taux de réussite par type de NAT.
>
> **▶️ COMMANDES POUR LA PREUVE RÉELLE (les 2 drapeaux) :**
> - Serveur : relancer le rendez-vous avec `RENDEZVOUS_RELAY=1` (sur le service systemd : ajouter la variable
>   d'env, ou lancer à la main `env RENDEZVOUS_RELAY=1 .../jeu rendezvous`). Le timer d'autoupdate pulle déjà ce code.
> - Joueur A : `env RENDEZVOUS_ADDR=<ip>:<port> RELAY_FALLBACK=1 ...jeu a`
> - Joueur B : `env RENDEZVOUS_ADDR=<ip>:<port> RELAY_FALLBACK=1 ...jeu b`
> - Critère : A et B se voient BOUGER alors qu'aucun ne perce l'autre (vérifié par « ABANDON du perçage » dans les logs).

> ### ⚙ PAPIER-WIRE 12.3-G — LE RELAIS GÉNÉRALISÉ (l'orbe & le reste traversent) — écrit le 22 juin 2026 (PAPIER, zéro code)
> *Suite directe du 12.3 PROUVÉ. Méthode identique : on fige le format AVANT de coder ; le drapeau gate
> tout ; le défaut reste byte-pour-byte intact ; 0 warning ; critère écrit AVANT. Lu dans le vrai code :
> `message.rs` (`RELAY_FWD_SIZE` = 2+32+182 = 216 o FIXE ; `encode/decode_relay_fwd`), `rendezvous.rs`
> (`relay_decision` fait `sig_ok(&inner)` sur un état 182 o), `orb.rs` (`orb_send` n'émet qu'aux trous
> `h.open` ; `decode_orb_verified`/`orb_sig_ok` vérifient le sceau de l'orbe à la RÉCEPTION),
> `netcode/send.rs` (`net_send` relaie déjà l'avatar pour les pairs `wants_relay`).*
>
> **LE SYMPTÔME (vérifié, pas supposé).** L'orbe apparaît en DOUBLE entre deux pairs relayés. Cause exacte
> ([`orb.rs`](src/net/orb.rs) `orb_send`, l.389-393) : l'orbe n'est diffusée qu'aux pairs au trou **OUVERT**
> (`h.open`). En relais NAT, **aucun trou n'est ouvert** → l'état d'orbe ne traverse jamais → chacun reste
> maître de la sienne. **Le relais porte l'avatar (182 o) mais PAS l'orbe.** `net_send` sait relayer ;
> `orb_send` non. C'est la seule lacune : le relais 12.3 est fait pour UN type de paquet (l'état joueur).
>
> **LA DÉCISION D'ARCHI (ce que je tranche).** On NE crée PAS un `KIND_RELAY_ORB_FWD` par type (2 kinds
> aujourd'hui, 3 demain… ça ne scale pas, et la vision 55K dit « le relais porte les OBJETS PARTAGÉS du
> monde, pas que les avatars »). On **généralise `KIND_RELAY_FWD`** en enveloppe à **payload de longueur
> variable** : elle porte n'importe quel paquet déjà scellé (état joueur, orbe, plus tard gossip…). Une
> seule enveloppe, propre et non-jetable.
>
> **LE FORMAT (figé) — `KIND_RELAY_FWD` reste 11, mais payload VARIABLE :**
> ```
> [0]                     KIND_RELAY_FWD (11)              — inchangé
> [1]                     PROTO_VERSION                    — inchangé
> [2..34]                 dest_id : clé publique du DESTINATAIRE. ROUTAGE seul, NON signé. — inchangé
> [34..]                  PAYLOAD : un paquet déjà scellé, VERBATIM, de longueur LIBRE      — CHANGE
> ```
> **⭐ Propriété clé qui protège la base PROUVÉE : pour l'avatar, les octets sont IDENTIQUES.** L'ancien
> format fixe = ce nouveau format avec payload = 182 o. Donc l'enveloppe d'avatar sur le fil **ne change
> pas d'un octet**. Seul le DÉCODEUR change : il ne vérifie plus `len == 216` mais `len >= 34`, et rend le
> payload comme tranche/`Vec` de longueur libre. **L'enveloppe d'orbe** = 2+32+`SIGNED_ORB_SIZE`(136) =
> **170 o** → bien sous le MTU (~1200 o), aucune fragmentation (≠ résumé v2 à 3,6 Ko, cf. foresight 12.4).
>
> **CE QUE LE RENDEZ-VOUS DOIT LÂCHER — et pourquoi c'est SÛR.** Aujourd'hui `relay_decision` fait
> `sig_ok(&inner)` : ça vérifie le sceau d'un état joueur de 182 o. Un payload variable peut être une orbe
> (autre structure, autre sceau) → le rendez-vous **ne peut plus vérifier le sceau génériquement** sans
> connaître le type. On **retire la vérif de sceau au rendez-vous** ; **le destinataire vérifie** (il le
> faisait déjà : un avatar relayé arrive en `KIND_STATE` normal et passe par le sceau à l'ingestion ; une
> orbe passe par `decode_orb_verified`/`orb_sig_ok` — VÉRIFIÉ dans le code, [orb.rs:149-164](src/net/orb.rs#L149)).
> **L'anti-amplification NE repose PAS sur ce `sig_ok`** — il repose sur 3 barrières qui RESTENT intactes
> (VÉRIFIÉ dans `rendezvous.rs`) : (1) **fanout = 1** (1 entrant → 1 sortant, jamais amplificateur) ;
> (2) **rate-limit par source** (`relay_credit`/`RELAY_CAP`, [rendezvous.rs:140](src/net/rendezvous.rs#L140)) ;
> (3) **dest doit être un client inscrit** (`relay_decision` rend `None` sinon → impossible de réfléchir vers
> une victime hors-jeu). La barrière qui PORTE la sécurité (dest-inscrit + fanout 1) ne bouge pas.
>
> **MES DOUTES (transparence, pas devoirs) :**
> - *Résidu assumé du retrait de `sig_ok` :* le rendez-vous relaiera désormais des octets non vérifiés (le
>   dest les jette si le sceau échoue). Pire abus = un client inscrit fait gaspiller à un AUTRE client
>   inscrit une vérif de signature (bon marché) à SON propre débit (1:1, rate-limité, attribuable). Borné,
>   pas un amplificateur. **Jugé acceptable** (on perdait juste un filtre d'hygiène, pas une garantie).
> - *Une alternative écartée :* unifier tous les payloads en « [corps signé][sig] avec pubkey à offset
>   connu » pour que le rendez-vous vérifie générique. Plus invasif (orbe et état ont des layouts
>   différents) pour re-gagner un filtre dont le dest se charge déjà. Pas le bon compromis.
>
> **CRITÈRE PRÉ-ENREGISTRÉ (Règle 2, écrit AVANT de coder).** Deux pairs au perçage abandonné des deux
> côtés (donc relayés, comme la preuve 12.3) voient **UNE SEULE orbe**, avec un **maître cohérent** (le
> même `owner` des deux côtés), et le transfert d'autorité fonctionne en relais. Juge = le ressenti +
> idéalement une trace (l'`owner` vu par A == celui vu par B). *Échec admis si l'orbe reste double, ou si
> le maître diverge entre A et B.*
>
> **PETITS PAS (cadence intouchable — base 12.3 PROUVÉE = intouchable, Règle 1) :**
> 1. **Papier-wire** = CE bloc. Zéro code.
> 2. **Pas A — généraliser l'enveloppe (prouvable HEADLESS, zéro changement observable).**
>    `encode/decode_relay_fwd` → payload variable ; `relay_decision` n'assume plus 182 o et **ne fait plus
>    `sig_ok`**. Tests unitaires : (a) **l'enveloppe d'avatar reste byte-identique** (garde-fou de
>    non-régression — c'est LA preuve que la base ne bouge pas) ; (b) un payload de longueur libre fait
>    l'aller-retour ; (c) malformé rejeté. compile → test → 0 warning → commit → push. *Aucun émetteur ne
>    produit encore de payload non-182 → comportement observable inchangé. Le relais avatar prouvé reste
>    byte-pour-byte sur le fil ; seul le `sig_ok` du rendez-vous disparaît (le dest vérifiait déjà).*
> 3. **Pas B — l'orbe emprunte le relais.** `orb_send` : pour les pairs `wants_relay` (comme `net_send`,
>    [send.rs:240-254](src/net/netcode/send.rs#L240)), emballer l'orbe scellée en `KIND_RELAY_FWD(dest)`
>    vers le rendez-vous au lieu de ne rien envoyer. Gaté par `RELAY_FALLBACK` (défaut OFF → `orb_send`
>    inchangé). compile → test → 0 warning → commit → push.
> 4. **Preuve réelle (Pas B)** : A (mobile) ↔ B relayés → **une seule orbe, maître cohérent** (critère
>    ci-dessus), lu sur le ressenti + trace. Re-vérifier au passage que **le relais avatar n'a PAS
>    régressé** (le log serveur montre toujours les 🔀 des deux sens) — no-premature-victory.
>
> **CE QUE ÇA NE FERA PAS (à dire d'avance) :** ne décentralise pas le relais (toujours v1 centralisé,
> D4 reste) ; ne résout PAS l'orbe « repère mort » (bug île séparé, cf. REPRISE §4) ; testé à 2 pairs, pas
> à l'échelle ; le gossip/autres objets ne passeront que quand on câblera LEUR émission (le format les
> portera, mais Pas B ne branche QUE l'orbe).
>
> **✅ 12.3-G PROUVÉ EN RÉEL (22-23 juin 2026, nuit) — l'orbe traverse le NAT.** Test A (Windows/mobile) ↔
> B (NixOS/maison), relayés des deux côtés : **une seule orbe, maître cohérent, les deux deviennent maître
> à tour de rôle.** Juge neutre = log serveur : 🔀 des DEUX sens (relais avatar NON régressé). Poussé `main`
> `56c237e` (Pas A `2878bea`, Pas B `56c237e`). 103 tests, 0 warning, défaut byte-pour-byte intact.
> - **Ce que ça NE prouve PAS / observé en réel :** (a) **convergence lente** ~10 s au connect (seuil
>   d'abandon de perçage) + phase asymétrique transitoire → connu (même fenêtre que le relais avatar),
>   s'auto-répare, améliorable plus tard (papier d'abord). (b) **L'orbe GÈLE si la fenêtre est occultée**
>   (niri/Wayland coupe les frame-callbacks → la sim, couplée au rendu, s'arrête → le pair voit l'orbe
>   revenir en arrière en boucle). **Dette renvoyée au SIDECAR** (le cœur en process séparé tourne hors
>   boucle de rendu → disparaît par construction). (c) v1 centralisé (D4 reste) ; testé à 2 pairs.
> - **Météorites désynchronisées** (observé Île) = seed RNG LOCAL (`subsec_nanos`, [meteorites.rs](src/meteorites.rs)) →
>   chacun son champ. Fix cheap = **seed PARTAGÉ** (objets procéduraux = déterministe local, pas de stream).

> ### 🎮 BASCULE UNREAL — réordonnée AVANT voix/chiffrement (décidé 22-23 juin 2026)
> *La preuve corrige le plan (pas de cases à cocher). Bevy bloque la créativité ; le cœur réseau est
> solide. On passe à Unreal pour la PRÉSENTATION ; **le cœur Rust reste INTOUCHABLE (Règle 1)** via le
> pont SIDECAR (process séparé, socket locale). Voix + chiffrement = chantiers du cœur, APRÈS (additifs).*
>
> **FAIT (22-23 juin) :** UE 5.8 installé et fonctionnel sur NixOS+niri (flake NixOSUnreal, binaire Epic
> précompilé, Vulkan prouvé sur RTX 4070, projet **C++** qui compile — fix CPATH). **Détail complet +
> recette de relance + objectifs : `REPRISE-PRIVEE.md` §6.** *(Réordonne H3 : NAT/D17 ✅ → Unreal MAINTENANT
> → voix → chiffrement plus tard.)*
>
> **AVANCEMENT (24 juin) :** projet Unreal `Spike01` = **dépôt GitHub PRIVÉ séparé**
> `shazamifius/spike01-unreal` (pas livré publiquement tant que pas solide — règle 4 ; ce n'est pas un
> secret, juste pas exposé). Le cœur réseau reste sur `web3game` (public). Fait côté client : **perso FP
> jouable + déplacement ZQSD** (Enhanced Input 100 % C++, prouvé), fix souris inversée, **regard clavier
> OKLM** (O haut/L bas/K droite/M gauche, axe additif). En cours : **menu Contrôles** (sensibilités,
> inversion, remap touches, manettes). Carnet technique « où on trouve quoi » = `COMPREHENSION_UNREAL.md` (gitignoré).

### Chapitre 13 — Voix spatiale
**But :** chat vocal P2P, priorité au volume (loudness priority), spatialisé. Profite du
chiffrement (10.2) et de l'inclusivité (ch. 8 — la voix s'adapte au lien).

### Chapitre 14 (plus tard, pas maintenant) — Portabilité moteurs
**But :** extraire un `net-core` portable (ABI C) pour Unreal/Unity. **Décidé : reporté.**

---

## E. Comment tester POUR DE VRAI avec une seule machine (ta question D1)

Tu as dit « on n'a qu'une seule machine, je ne sais pas comment faire ». **Bonne
nouvelle : une seule machine suffit.** Linux sait simuler une mauvaise connexion :

- **`tc netem`** (sur l'interface loopback `lo`) ajoute **latence, jitter, perte,
  ré-ordonnancement** à TOUT le trafic localhost. On lance notre `sim` derrière, et nos
  centaines de nœuds se parlent soudain « comme sur Internet ». C'est exactement fait
  pour ça. (Au chap. 7 je t'écris `tools/sim-netem.sh` qui l'applique puis le retire
  proprement, comme `sim-cool.sh` le fait pour les ventilos.)
- **`tc tbf`** limite le débit (pour simuler les « quelques Ko/s »).
- **`ip netns`** (namespaces réseau) crée de « fausses machines » isolées derrière de
  « faux NAT » sur le même PC — c'est déjà ce que fait `tools/test-nat.sh`.

Donc : **on n'a pas besoin de 2 machines** pour confronter au réel. Une seule + netem =
un labo réseau complet. C'est ça qui transforme nos tests « localhost » en vraie preuve.

---

## F. Ordre conseillé & priorités

1. **Chapitre 7 d'abord** (réalisme/netem) — sans ça, tout le reste est de la confiance,
   pas de la preuve. Et il va probablement révéler des bugs réseau réels à corriger.
2. **Chapitre 8 ensuite** (la foule dense + inclusivité, **ferme D22**) — LE gros morceau
   d'archi et le cœur de ta vision « tout le monde peut jouer / voir la foule ». Dépend des
   mesures réalistes (donc après 7). On commence par **8.0** (mesurer le mur : couverture de
   perception) avant de coder la solution (gossip + AoI à deux tiers + cellules agrégées).
3. **Chapitre 9** (confiance dure) — referme les attaques *coordonnées*, les vraies.
4. **Chapitre 10** (identité persistante + chiffrement) — indispensable pour de vrais
   utilisateurs.
5. **Chapitres 11–12** (autorité généralisée, robustesse) — élargir et durcir.
6. **Chapitre 13** (voix), puis **14** (moteurs, plus tard).

> Note : 7 → 8 → 9 → 10 est le chemin « solide ». Mais si un jour tu veux du *visible*
> vite (pour le moral), 8.3 (le 0-connexion qui joue via un parent) est très
> spectaculaire. À toi de doser preuve vs effet.
>
> **🔀 ORDRE RÉVISÉ (décidé le 19 juin, cf. pivot §0) : 7 → 8 (partiel : jusqu'à 8.2c) →
> 9 (entier) → REPRISE de 8 (8.3 cellules + Phase B inclusivité) → 10.** Pourquoi on
> intercale le 9 au milieu du 8 : le reste du chapitre 8 (hôte de cellule, parent agrégateur)
> bâtit une couche d'**agrégateurs de confiance** ; on durcit donc la confiance (ch.9 : Sybil,
> éclipse, quorum pondéré) AVANT de s'appuyer dessus. Décision RÉVERSIBLE et tracée — si la
> reprise du 8 révèle qu'on s'est trompé, on rouvre.

---

## G. Décisions PRISES (juin 2026)

Ces choix ont été tranchés avec l'utilisateur. Ils orientent le plan :

1. **Vie privée → ON CHIFFRE TOUT.** Direction ferme : positions + voix chiffrées.
   Décidé par l'utilisateur (« en vrai il faudrait tout chiffrer »). Réalisé au
   **chapitre 10.2** (X25519 par paire) — pas avant, la base passe d'abord.
2. **Anti-Sybil → preuve de travail RÉGLABLE.** On garde la PoW, on rend sa difficulté
   ajustable, et on ne l'augmente QUE si les tests montrent que les attaquants gagnent.
   Le *vouching* social reste en réserve (chap. 9.1).
3. **0-connexion → on suit l'ordre normal.** Pas de priorité forcée : chapitre 7
   (mesurer) puis chapitre 8 (inclusivité, déjà priorité 2) ; le 0-connexion (8.3) vient
   dans le 8, après ses briques de base.
4. **Identité persistante → OUI, clé sauvée dans un fichier** (comme une clé SSH).
   Simple d'abord ; protection par mot de passe plus tard, avec le chiffrement du
   chapitre 10. Réalisé au **chapitre 10.1**.

---

## H. Annexe — features avancées OPTIONNELLES (reportées, décidé le 19 juin)

> *Ces durcissements sont réels et intéressants, mais **trop complexes pour le gain actuel** : le cœur dur
> (Sybil, éclipse, framing, réputation) est déjà tenu et prouvé sans eux. On les **sort du chemin critique**
> pour rester concentré sur ce qui valide la vision (échelle + inclusivité + vie privée). On y reviendra
> seulement si une MESURE ou un vrai déploiement les rend nécessaires — surtout au moment du portage vers un
> vrai moteur (Unreal/Unity) à grande échelle. Rien n'est perdu : le raisonnement est tracé, prêt à reprendre.*

- **9.1(b) — PoW localement ADAPTATIVE.** Chaque nœud relève la difficulté qu'il EXIGE selon la pression locale
  (cadence de nouvelles identités / d'accusations). Vraie défense *dynamique* de masse, mais soulève la question
  « comment les nœuds s'accordent » (pas de consensus). *Le socle réglable (9.1a) suffit pour l'instant.* Détail :
  🧭 CARREFOUR 9.1 (§D, ch.9), piste (b).
- **9.2c — Standing par DURÉE.** Graduer la crédibilité d'un témoin par son ancienneté/quantité de participation
  (pas binaire). *Reporté car 9.4b (cap par sous-réseau /24) ferme déjà l'essentiel du résidu patient* : des
  Sybils même établis et co-localisés partagent les IP de l'attaquant → comptés comme UNE voix. Gain marginal.
- **9.5 — FÉDÉRATION de rendez-vous.** Plusieurs rendez-vous indépendants qui s'échangent des pairs (résilience
  ultime de l'amorçage, ferme la fin de D10/D21). *L'amorçage actuel tient (rendez-vous borné 9.5a + découverte
  par gossip 8.1) ; la fédération est un gros morceau d'archi distribuée, à faire quand le besoin réel se posera.*
- **Vouching social (2ᵉ facteur anti-Sybil).** Parrainage social (coût relationnel, pas CPU → ami des faibles).
  *Relié à l'inclusivité (Phase B, ch.8) — à étudier là-bas si besoin.* Détail : 🧭 CARREFOUR 9.1, piste (c).

*(Les chapitres 13 « voix » et 14 « portabilité moteurs » restent eux aussi « plus tard », comme déjà noté en §D.)*

---

*Ce document est vivant : on coche les sous-étapes et on l'enrichit au fil de l'eau,
exactement comme on l'a fait pour le chapitre 6 dans le README.*
