# 🧭 ÉTAT — le point de reprise (À LIRE EN PREMIER chaque session)

> L'ancre anti-dérive : le pourquoi, les 5 règles, le plan d'attaque, l'état courant + la prochaine action, et le registre de dettes.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

## 0. ▶️ POINT DE REPRISE (lis ça en premier, surtout si nouvelle session)

> ### 🎯 LE POURQUOI + RÈGLES + PLAN D'ATTAQUE (posé le 20 juin 2026 — lire EN PREMIER)
> **⤴ MAJ 25 juin 2026 — la VISION a grandi :** le projet n'est plus « un espace social VRChat-like » mais une
> **PLATEFORME de jeux P2P façon gamejolt** (multi-moteur, mondes téléchargés, hub 3D — détail dans
> [VISION.md](VISION.md)). Le **« 55K » ci-dessous reste le TEST DE STRESS du CŒUR** (prouver que le P2P sans serveur
> passe l'échelle), pas le produit lui-même. Le produit immédiat = un **1er petit jeu** jouable à ~10 potes (l'« île aux
> étoiles ») pour récolter bugs/critiques. *Toute mention « espace social / VRChat » plus bas = le 1er monde, pas tout le projet.*
>
> **Le but ultime (cœur) :** un **événement P2P sans serveur réunissant ~55 000 personnes** sur Unreal Engine, pour fêter
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

> ### ⏱️ ÉTAT COURANT + PROCHAINE ACTION (L'ANCRE anti-dérive ; maj 25 juin 2026)
> *Lire CE bloc + le 🎯 ci-dessus suffit pour reprendre au bon niveau. Tout ce qui suit dans §0
> est un JOURNAL d'archive — ne le relire qu'au besoin, via `grep`. Anti-collapse : on s'ancre ici,
> on ne se rejoue pas tout l'historique à chaque session.*
>
> **🧹 DÉ-BEVY (Étape A) FAIT — 25 juin 2026.** Le **client Bevy (la fenêtre de jeu) est RETIRÉ** et le
> **cœur ne dépend plus d'aucun moteur 3D** : `bevy::Vec3` → un `Vec3` maison (`src/math.rs`) ; derives
> `Resource`/`Component` retirés ; systèmes ECS de l'orbe/punch supprimés (la **logique pure ORBE+OWN** —
> `supersedes`/`apply_incoming`/wire signé — est GARDÉE, propre et réutilisable, exercée par le `Bot` + tests) ;
> `netcode/` (interpolation/prédiction) supprimé (c'est Unreal qui interpole, cf. `CONTRAT_SIDECAR.md` §4).
> Supprimés aussi : `world/player/scenes/meteorites` + champs morts `world_hue`/`weak`. **Cœur byte-pour-byte
> intact** : compile **0 warning** (Bevy retiré → build ~1 s vs interminable), **108 tests verts** (106 +4 `math`
> −2 `HoleState` dont le seul consommateur, `net_punch`, disparaît), `sim` = orbe intègre, `relay-test` = relais
> deux sens, `sidecar` démarre. **Reste (Étape B, optionnelle) :** extraire le cœur en crate lib `web3core` (workspace) —
> plumbing cosmétique, le sidecar marche déjà ; à faire SI ça sert, sinon prioriser le petit jeu.
>
> **🟢 JALON 1 FAIT (25 juin) :** vrais avatars dans Unreal (au lieu des capsules debug), prouvé en `-game` (un avatar
> nommé bouge sur la map via le réseau). Repère partagé ~gratuit (deux UE = même niveau). *(dépôt UE `spike01-unreal`.)*
>
> **▶️ COURT TERME — la liste « MVP : est-ce vivant ? » (PAS une checklist — cf. la boussole vivante dans [VISION.md](VISION.md)).**
> On fait chaque pas BIEN, dans l'ordre que la preuve dicte ; on ne coche jamais « ça passe à peu près ». Pas à pas :
> 1. **Jalon 2 — l'île + les avatars dessus** (utilisateur = niveau/visuel/assets ; moi = câblage). Un vrai sol, une origine claire.
> 2. **Fix dette anti-triche « sur-punition »** (téléport/claim légitime → `NoContact` → mute) — AVANT de refaire circuler des objets.
> 3. **Palier 4 — les objets partagés dans le contrat sidecar** (réveiller l'orbe proprement) : le chaînon « avatars » → « jeu avec des choses ».
> 4. **Étoiles déterministes (graine partagée) + ramassage = événement d'autorité** (réutilise ORBE+OWN) + cristaux.
> 5. **Chat texte** de proximité.
> 6. **TEST avec de vrais potes** (+ vocal Discord pour dé-risquer la prémisse sociale) → ferme **D27** « est-ce vivant ».
> *On ne décide races/stats/**persistance (D28)** qu'APRÈS ce test. La plateforme reste une étoile lointaine.*
>
> **OÙ ON EN EST (cœur, avant le virage jeu) :** chapitres **0→7 faits**, **ch.9 (confiance dure) tenu**, **ch.8 « foule dense » Phase A
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

