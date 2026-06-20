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

> ### ⏱️ ÉTAT COURANT + PROCHAINE ACTION (≤30 lignes — L'ANCRE anti-dérive ; maj 20 juin 2026)
> *Lire CE bloc + le 🎯 ci-dessus suffit pour reprendre au bon niveau. Tout ce qui suit dans §0
> est un JOURNAL d'archive — ne le relire qu'au besoin, via `grep`. Anti-collapse : on s'ancre ici,
> on ne se rejoue pas tout l'historique à chaque session.*
>
> **OÙ ON EN EST :** chapitres **0→7 faits**, **ch.9 (confiance dure) tenu**, **ch.8 « foule dense »
> Phase A BOUCLÉE** (FOCUS ~32 / CONSCIENCE LOD / gossip / résumés de cellule frais), **10.1 identité
> PERSISTANTE FAIT** (D14 fermé). **68 tests, 0 warning.** Audit : **9 fermés · 7 bornés · 10 ouverts**
> (sur 26).
>
> **3 MESURES « TROP GENTILLES » à ne pas se mentir (détail au 🧾 registre) :** (a) « couverture » =
> pairs CONNUS, pas ENTENDUS (optimiste) ; (b) la FRAÎCHEUR (âge de perception d'un lointain) jamais
> mesurée en direct → « 1/N tué » prouvé seulement *indirectement* ; (c) banc plafonné ~1500 nœuds →
> « perception ∝ N » mesuré à 1000-2000, *argumenté* au-delà → **ne JAMAIS dire « 55K prouvé »**.
>
> **⚠ DOUTE VIVANT repéré le 20 juin (à traiter en H2) :** les résumés de cellule (8.3) sont **NON
> authentifiés** — `ingest_summary` ne vérifie que `ts` + borne mémoire (aucune signature, aucune
> PoW, aucun contrôle d'émetteur). Donc **D26 est plus large que « l'hôte ment »** : n'importe qui
> forge un résumé pour n'importe quelle cellule, et un `ts = u64::MAX` ÉPINGLE le mensonge à vie (la
> vraie info ne peut plus l'écraser — c'est le « verrou v65535 » de l'orbe, mais SANS le garde de 5.3).
>
> **PROCHAINE ACTION = H2 (défricher SUR LE PAPIER, règle 5)** les 2 murs probables avant de coder
> Phase B : **D4** (incitation au relais = problème de MÉCANISME, le vrai mur intellectuel → EN
> PREMIER) puis **D26** (résumé menteur, cf. doute ci-dessus). *Ne PAS construire d'outillage avant
> d'avoir pensé le mécanisme.*
>
> ──────────── JOURNAL DÉTAILLÉ ci-dessous (archive — relire au besoin via `grep`) ────────────

**Où on en est :** le **chapitre 6 (refonte BÉTON) est TERMINÉ** — les 10 trous de
l'audit fermés/bornés (0.0→6.8), **35 tests, 0 warning**, et **le jeu 3D réel
fonctionne** (avatars + pseudos `0000…` + badge OWN BALLE visibles à l'écran). Tout
est poussé sur GitHub (`shazamifius/web3game`, branche `main`).

**Les 4 décisions de direction sont prises** (détail section G) : ① **on chiffre
tout** (ch. 10) ; ② **PoW anti-Sybil réglable** (on durcit si les tests l'exigent) ;
③ **ordre normal** 7→8→9→10 (pas de priorité forcée au 0-connexion) ; ④ **identité
persistante = clé sauvée dans un fichier** (ch. 10).

**On est dans le CHAPITRE 7 (confrontation au réel). 7.1, 7.2 et 7.3 sont FAITS.**
**7.1 ✓** — `tools/sim-netem.sh` (3 profils) applique `tc netem` sur `lo`, retire toujours
le netem (`trap`). Piège : sur `lo` le délai compte double → profils en ping, `delay/2`.
**7.2 ✓** — mesuré (`sim 50 5 30`). Sécurité INTACTE partout (orbe 0/50, attaques
neutralisées même à 250 ms + reorder). Débit honnête : `bon` ~22,7k/s, `mauvais` ~6,8k/s
(−70 %). Diagnostic initial (anti-rejeu strict) → **s'est révélé FAUX au 7.3.**
**7.3 ✓** — anti-rejeu à fenêtre glissante (64, masque `u64`) dans `accept_seq` ; tolère
le ré-ordo, refuse rejeu + trop-vieux ; 36 tests, 0 warning. Re-mesuré : `mauvais` remonte
de ~6,8k à ~7,9k/s (**+15 % seulement**) → l'anti-rejeu n'était PAS le goulot. Vraie cause
identifiée : le `limit 1000` par défaut de netem plafonne à ~limit/délai ≈ 8 000/s à
125 ms (= pile la mesure). Le fix reste correct (vrais réseaux ré-ordonnent).

**7.3b ✓** — `limit 100000` dans le harnais : sous `mauvais`, débit honnête ~7,9k → **~21,3k/s**
(`bon` ~23,3k) → **−9 % seulement**. PROUVÉ : le −70 % était l'artefact `limit 1000`, PAS le
protocole. **Le protocole tient sous réseau réel.** Le cœur du chapitre 7 est atteint.

**7.4 ✓** — `sim` chiffre le coût RÉEL par nœud (nouveau `src/net/probe.rs`) : bande passante
(compteurs d'octets dans la prise) + CPU du thread (`/proc/thread-self/stat`), réels ; RAM
crête **globale** du process (pas de RAM/nœud factice — un seul tas partagé).

**7.4b ✓ — fidélité + densité (révision de feuille de route faite avec la vision du code) :**
le 1er chiffre de 7.4 (↑89 Ko/s) était mesuré sur le **mauvais chemin** — le bot émettait
naïvement à tous, pas via l'AoI water-filling du vrai client. Corrigé : le bot appelle
maintenant les mêmes fonctions qu'[aoi.rs]. Re-mesuré : **↑34/↓31 Ko/s/nœud, CPU ~0,7 %/cœur,
38 Mo** (~0,27 Mbit/s ↑/joueur → très tenable). SURTOUT, le rapport AVOUE désormais le vrai
mur : **D22 — en foule dense, on est aveugle au-delà de 32 voisins** (plafond dur du
rendez-vous ; le water-filling ne peut rien car il n'apprend jamais le 33e). **Ferme D19,
ouvre D22.** 36 tests, 0 warning.

**7.5 ✓ (PREUVE NAT RÉELLE FAITE)** — `tools/test-nat.sh` généralisé au MULTI-joueurs (N
maisons `p1..pN` derrière `nat1..natN`, résumé du mesh). A révélé + corrigé un **bug
d'instrumentation** dans [natdemo.rs] (trou ouvert en silence si données reçues avant le
punch). Puis **preuve réelle sous `sudo` (namespaces + vrais NAT), en ~16 s chacun** :
`sudo ./tools/test-nat.sh 3 --cone` → **6/6 → MESH COMPLET** (full-cone : punch direct
deux-à-deux) ; `sudo ./tools/test-nat.sh 3` (symétrique) → **0/6** (punch échoue → c'est le
rôle du relais ch.5). En cours de route, **deux bugs du script** corrigés (exposés par le run
réel) : `wait` nu attendait le rendez-vous sans fin (test « durait 25 min ») → on n'attend que
les joueurs ; et `set -e` + code 124 de `timeout` coupait avant le résumé → absorbé par
`|| true`. Le hole punching multi-joueurs est donc prouvé pour de vrai, pas juste sur localhost.

**PLAN DU CHAPITRE 8 (densité, D22) ÉCRIT — prochaine action = CODER 8.0.** Le chapitre 7
(« confrontation au réel ») est bouclé : le lien tient sous mauvais réseau (7.1→7.3b), le
coût/nœud est chiffré honnêtement (7.4/7.4b), le NAT multi-joueurs marche (7.5). Le gros
morceau d'archi — **D22 : en foule dense, aveugle au-delà de 32** — a maintenant son chapitre
dédié, écrit AVANT de coder (règle d'or). Voir **§D, Chapitre 8 — La foule dense & l'inclusivité**.
Le diagnostic est net : le plafond est au rendez-vous ([rendezvous.rs](src/net/rendezvous.rs) :
`keep_nearest(…, 32)`) et le client écrase `link.peers` avec ce roster ([receive.rs](src/net/netcode/receive.rs))
→ le 33e n'est jamais appris. La réponse (architecture, pas réglage) : **séparer FOCUS (lien plein,
borné ~32) de CONSCIENCE (perception LOD, non plafonnée)**, découverte par **gossip** (le rendez-vous
démoté à l'amorçage), **cellules + hôte agrégateur** pour tenir l'invariant *réception = O(K + cellules),
indépendant de N*. **8.0 ✓ FAIT (le mur est chiffré) :** mode `cargo run -- crowd <N>` + métrique de
**couverture de perception** + tiers focus/conscience. Mesuré : `crowd 200` → couverture **16 %**
(FOCUS 32 + CONSCIENCE 0), **aveugle à 167** ; débit de référence à BATTRE **↓ 24,8 Ko/s** (doit
rester PLAT quand la couverture montera). NB : l'ancien « Chapitre 8 — Inclusivité » a été FUSIONNÉ
dans ce chapitre densité (même problème vu des deux bouts : « je ne vois pas la foule » ↔ « je ne
peux pas tout recevoir de la foule ») — D3/D4/D5 y deviennent la Phase B.
**8.1 ✓ FAIT (19 juin) — LE MUR DE D22 TOMBE.** Découverte par GOSSIP : nouveau `src/net/gossip.rs`
(cartes de visite `KIND_GOSSIP`), le WELCOME **amorce** `link.peers` au lieu de l'écraser, et chaque
nœud s'échange à bas débit un sous-ensemble divers de pairs connus. Logique d'apprentissage mise dans
`NetLink` (`learn_peer`/`note_pos`, borne `MAX_KNOWN`) → partagée bot+jeu (pas de re-D2). **Mesuré :
couverture 16 % → 98 % à `crowd 200`, et l'INVARIANT est prouvé** — le débit ↓ ne grandit PAS de 200
à 500 nœuds (↓35→↓27 Ko/s, ↑ plat ~40 Ko/s, CPU ~0,7 %, orbe 0 volée). 42 tests, 0 warning. *Découverte
clé en lisant le code : le coût de réception était DÉJÀ borné par le budget d'émission fixe ; le seul
vrai mur était la DÉCOUVERTE (le plafond 32). Le gossip l'enlève.*

**Jeu 3D VALIDÉ à 2 joueurs (capture utilisateur, 19 juin) :** deux fenêtres, chacune voit l'avatar
de l'autre (pseudos `0000…`, badge OWN BALLE, membres/ombres/néon OK) — le gossip n'a rien cassé en
3D. *Ne prouve PAS la foule dense en 3D* (2 ≠ 200 ; le plafond `MAX_AVATARS = 64` n'est pas stressé → D24).

**8.1b ✓ FAIT (19 juin) — la porte DoS du gossip est FERMÉE (D23 fermé).** Quatre défenses en
profondeur : (a) PoW exigée sur chaque carte apprise, (b) le gossip n'écrase jamais l'adresse d'un
pair connu (anti-redirection), (c) abandon du perçage spéculatif après ~10 s (avant : à vie → flot
réfléchi infini), (d) rate-limit d'apprentissage par source. Logique centralisée dans `NetLink`
(`learn_from_gossip`) + fonction pure partagée `punch_abandoned` (bot ET jeu). **PROUVÉ par un VRAI
attaquant** `attack gossip-flood` : **0 perçage réfléchi** reçu par la cible, tables non polluées ;
découverte honnête intacte (`crowd 60` → couverture 100 %), essaim TENU avec l'attaquant actif
(`sim 40 6 20`, orbe 0/40). **47 tests, 0 warning.** **Doute #1 fermé bout-en-bout (8.1b-preuve) :**
même avec de vraies identités PoW minées pointées sur une victime, la réflexion est BORNÉE — mesuré
**96 perçages/2 s pendant ~10 s puis 0** (l'abandon mord), au lieu du flot infini d'avant.

**8.2 (netcode) ✓ FAIT (19 juin) — AoI à DEUX TIERS, focus COLLANT, invariant TENU.** 8.2a (alloc deux
tiers) → 8.2b (métrique « entendus » qui a DÉMASQUÉ un churn du focus) → 8.2a-bis (focus collant par
hystérésis, churn tué). **Prouvé :** `crowd 160` FOCUS 0,2 → 9,4 ; pair 80↔160 → FOCUS borné (8,8→10,5),
CONSCIENCE scale (68→134 = foule en LOD), **débit ↓ PLAT (43,8→40,4 Ko/s) quand N double**. 51 tests, 0 warning.

**8.2c ✓ FAIT & CONFIRMÉ À L'ÉCRAN (19 juin) — D24 FERMÉ.** Rendu 3D à deux tiers (focus détaillé /
conscience imposteur LOD) + `tools/foule-3d.sh` + spawn éparpillé dans la salle. Capture utilisateur à
`foule-3d.sh 80` : ~8 avatars détaillés (pseudos) + foule d'imposteurs, > 64 visibles, sans lag. **Le
chapitre 8 « VOIR la foule » (Phase A, D22+D24) est bouclé** — sauf 8.3 (scaler à 5000).
*Observé en passant : l'orbe saute en foule 80-fenêtres → diagnostiqué ARTEFACT mono-PC (cf. panneau dans
le registre), NON corrigé ; résidus réels R1 (orbe non interpolée) / R2 (migration split-brain = D11) logués.*

**8.3a ⏸ POSÉ MAIS EN PAUSE (19 juin) — grille de cellules + élection d'hôte, testé, PAS câblé.** Premières
briques de 8.3 écrites et prouvées : `aoi::cell_of` (grille infinie, `floor` pour gérer les négatifs) et
`NetLink::cell_host`/`am_i_cell_host` (hôte = plus petit id connu dans la cellule, même règle que la migration
de l'orbe, mais NON autoritaire → un double hôte ne corrompt rien, juste un résumé redondant). 53 tests, 0
warning (`#[allow(dead_code)]` documenté : pas encore d'émission `KIND_CELL_SUMMARY`). **8.3b/c/d restent.**

**🔀 PIVOT DÉCIDÉ (19 juin) : on attaque le CHAPITRE 9 (confiance dure) AVANT de finir le chapitre 8.**
*Pourquoi (re-think assumé, pas une rustine) :* 8.3 (hôte de cellule) et toute la Phase B (parent agrégateur,
8.4→8.8) bâtissent une couche d'**agrégateurs** où un hôte/parent **résume la foule pour les autres**. Un
agrégateur **malveillant qui ment sur sa région** (cache/invente des gens) = **D5/D9** ; la feuille elle-même
renvoie la corroboration anti-éclipse au ch.9 (note 8.3 doute (b), étape 8.8). Bâtir l'agrégateur sur une
confiance non durcie = béton sur du sable. **De plus, un trou VIVANT avant le ch.9 :** la réputation partagée
(6.7) est *frameable* aujourd'hui — `ACCUSE_QUORUM = 3` + PoW jouet 16 bits (D6) → **3 Sybils bon marché font
taire n'importe quel honnête** (D6/D7/D20). Le ch.9 (9.1 anti-Sybil dur/réglable, 9.2 quorum pondéré, 9.4
anti-éclipse + corroboration des positions, 9.5 rendez-vous résilient) referme ça AVANT qu'on s'appuie dessus.
**Reprise de 8.3** (câblage `KIND_CELL_SUMMARY` + Phase B inclusivité) : APRÈS le ch.9, sur une confiance solide.

**9.1 EN COURS — le trou est PROUVÉ (19 juin), harnais de régression en place.** Nouvelle attaque rouge
`cargo run -- attack sybil-frame` (autonome, auto-mesurée comme `gossip-flood`) : un SEUL attaquant mine
`ACCUSE_QUORUM = 3` identités Sybil (**~2,1 s ici, ≈0,7 s/identité** — PoW 16 bits = jouet, D6), signe 3
accusations contre un INNOCENT, les livre par le VRAI chemin (signer → `encode_accuse` → `decode_accuse` →
garde du récepteur recopiée de [bot.rs] → `record_accusation`) à un témoin honnête → **le témoin met
l'innocent en SOURDINE** (« FRAMING RÉUSSI »). Trou **D6/D7/D20 prouvé bout-en-bout**. L'attaque est sa
PROPRE preuve : après le correctif, le même binaire devra imprimer « framing ÉCHOUÉ ». **53 tests, 0 warning.**

**9.2 (1re couche) ✓ FAIT (19 juin) — le framing BON MARCHÉ est FERMÉ et PROUVÉ.** On a remplacé le quorum
qui comptait des TÊTES par un **quorum PONDÉRÉ par crédibilité** (`record_accusation` + `accusation_weight`
dans [link.rs]) : un accusateur ne pèse que s'il a du **standing** (il m'a déjà envoyé un état signé accepté
→ entrée `replay`) ET selon sa **co-localisation** avec l'accusé (`WITNESS_RADIUS`). Un Sybil conjuré (jamais
entendu) pèse **0**. *Preuve :* `attack sybil-frame` est passée de « FRAMING RÉUSSI » à **« framing ÉCHOUÉ »**
(même binaire = harnais de régression) ; +2 tests unitaires (témoins crédibles → sourdine ; 100 Sybils sans
standing → innocent intact). *Non-régression :* `sim 40 3 15` → **40 sourdines** (vrais tricheurs toujours
neutralisés), orbe 0/40, couverture 100 %, débit plat. **54 tests, 0 warning.**

**9.1 (couche a — socle réglable) ✓ FAIT (19 juin) — la PoW n'est plus figée, et le socle est choisi PAR LA
MESURE.** `POW_BITS` n'est plus une constante de compilation : nouvelle fonction `crypto::pow_bits()` résolue
une fois par processus, **surchargée par la variable d'env `POW_BITS`** (plafonnée à 28 ; comme `RENDEZVOUS_ADDR`).
Tous les sites (minage `generate_pow`, vérif `has_pow`) passent par elle. En `#[cfg(test)]`, `NetLink::new` n'utilise
PAS de minage (sinon `cargo test` minerait à pleine difficulté à chaque lien). **Courbe coût/difficulté MESURÉE**
(ce PC, ×4 par +2 bits) : 16 b ≈ 0,3 s · **18 b ≈ 3 s** · 20 b ≈ 14 s · 22 b ≈ 55 s *par identité*. **Socle par
défaut = 18** (et plus 16) : choix MESURÉ — (1) inclusivité (~25-30 s sur vieux téléphone = coût d'entrée unique
acceptable ; 20+ exclurait les faibles) ; (2) depuis 9.2 la PoW n'a plus à être punitive (le framing est fermé par
la CRÉDIBILITÉ, pas par le prix de l'identité) → 4× le jouet suffit. **54 tests, 0 warning.** *Insight tracé
(challenge de la feuille) : « difficulté bien plus haute » était la mauvaise cible — la data montre que cogner haut
taxe surtout les honnêtes/faibles pour un gain marginal post-9.2. La vraie défense de MASSE dynamique = la couche
(b) ADAPTATIVE (relève la barre LOCALEMENT sous pression), pas un gros nombre fixe.*

**9.4a (corroboration des positions) ✓ FAIT (19 juin) — la fausse co-localisation par gossip est MORTE.**
Champ séparé `confirmed_pos` (écrit seulement par `note_pos`, donc depuis un état SIGNÉ) ; la crédibilité 9.2
le lit au lieu de `peer_pos` (que le gossip peut polluer). Un attaquant ne peut plus gossiper « mon témoin est
collé sur la victime » pour framer. Test dédié + non-régression simu. **55 tests, 0 warning.** *Ferme le levier
GRATUIT du résidu patient de 9.2 ; reste le coûteux (déplacer pour de vrai ses Sybils près de la victime) → 9.4b.*

**9.4b (diversité de RÉSEAU, anti-éclipse) ✓ FAIT (19 juin) — ⚠ avec CHALLENGE assumé.** `record_accusation`
cape la crédibilité par SOUS-RÉSEAU /24 (`subnet_key`) → des Sybils derrière une seule IP = 1 voix, même
co-localisés pour de vrai. *Challenge :* la « diversité d'id façon Kademlia » du plan était inadaptée à notre
modèle (ids PoW aléatoires → buckets non discriminants) ; le vrai levier est l'IP (rare), pas l'id (gratuit).
Loopback distingué par port → simu intacte. Test dédié + non-régression `sim 40 3 15` → 40 sourdines, orbe 0/40.
**56 tests, 0 warning.** *Le résidu patient/co-localisé de 9.2 est fermé pour un attaquant à IP rares.*

> ### 🤖 PLAN DE RUSH AUTONOME (établi le 19 juin, l'utilisateur s'absente)
> *Ordre choisi du MOINS risqué au FORK. Règle absolue : je m'ARRÊTE net au premier vrai doute, mesure qui
> contredit, ou décision qui t'appartient — je ne pose JAMAIS de rustine pour avancer. Chaque étape = design
> (+ challenge si besoin) → code → PREUVE (test unitaire + non-régression `sim`) → commit → push → maj de ce doc.
> Ce plan n'est PAS gravé : si une étape révèle qu'il faut le changer, je le change et je le note.*
> 1. **9.5a — Rendez-vous BORNÉ (anti-DoS mémoire, D21). ✓ FAIT (19 juin).** `MAX_CLIENTS = 8192` + `should_admit`
>    (un connu est toujours rafraîchi ; un nouveau seulement s'il reste de la place) dans [rendezvous.rs] → un
>    flood de HELLO usurpés ne fait plus enfler la table sans fin (au pire elle sature, l'éviction 5 s la draine).
>    57 tests, 0 warning ; non-régression sim 40/40, orbe 0/40. *Résidu ASSUMÉ : table pleine → un honnête peut
>    être refusé ; vraie parade = routabilité (handshake anti-spoofing) → étape ultérieure. Ici on borne la RAM.*
> 2. **9.3 — Réhabilitation (D8). ✓ FAIT (19 juin).** `strikes` est passé d'un compteur `u32` permanent à un
>    `Strike { score, last }` qui DÉCROÎT (`decayed_score`, pure/testée ; `STRIKE_DECAY_SECS = 60 s`/faute) →
>    une faute transitoire se dissipe (réhabilitation après ~5 min sans récidive), un récidiviste ré-accumule et
>    reste muet. `is_muted`/`add_strike`/`muted_count` réécrits ; [bot.rs] `muted()` passe par `muted_count()`.
>    Preuve : tests `score_de_fautes_decroit_avec_le_temps` + `rehabilitation_apres_decroissance_mais_pas_le_recidiviste`
>    (injection de `now` → pas de `sleep`). 59 tests, 0 warning ; non-régression sim → 30 sourdines, orbe 0/40.
> 3. **9.2c — Standing par DURÉE. ⏸ RE-ÉVALUÉ → PAS FAIT (arrêt volontaire au doute, 19 juin).** En le concevant,
>    constat : **9.4b (cap par sous-réseau /24) ferme DÉJÀ l'essentiel du résidu patient** — des Sybils même
>    établis ET co-localisés pour de vrai partagent les IP de l'attaquant → comptés comme UNE voix. 9.2c n'ajouterait
>    qu'un gain marginal, contre une vraie complexité + un tradeoff (métrique durée/quantité ; risque de freiner la
>    propagation de réputation en fenêtre courte). Le faire en autonomie = gold-plating (rustine pour cocher une
>    case). **Décision : on s'arrête, à TRANCHER avec l'utilisateur** (le faire quand même ? le supprimer du plan ?).
> 4. **⛔ STOP — je NE fais PAS en autonomie (forks qui t'appartiennent) :** **9.1(b) PoW adaptative** (comment
>    les nœuds s'accordent sur la difficulté = vraie décision, cf. 🧭 carrefour) et **9.5 fédération** (plusieurs
>    rendez-vous). 
>
> **▶ Rush autonome TERMINÉ : 9.5a ✓ + 9.3 ✓ faits et poussés ; arrêt propre au doute sur 9.2c.**

> ### 🧭 CAP REPRIS (19 juin, soir) — décidé avec l'utilisateur
> Le **chapitre 9 est tenu** (cœur dur Sybil/éclipse/framing/réputation) ; les durcissements avancés sont
> **rangés en ANNEXE H** (optionnels). **Audit des doutes FAIT** → tableau d'état D1→D26 en tête de §C (maj 20 juin) :
> **9 fermés, 7 bornés, 10 ouverts** (sur 26 ; D22 fermé au 8.3d, D14 fermé au 10.1, + D25 banc d'essai et D26
> agrégateur menteur nommés le 20 juin), regroupés en chantiers (inclusivité / vie privée / autorité / longévité). **Plan validé : finir le chapitre 8 (8.3 échelle → Phase B inclusivité), puis le
> chapitre 10 (vie privée + identité) — ça ferme à soi seul 6 des 10 doutes ouverts, les plus proches de la
> vision.** Le portage moteur (Unreal/VRChat, ch.14) reste APRÈS : on contrôle tout, moteur compris, jusque-là.
>
> **8.3 EN COURS (reprise) :** ✅ 8.3a (grille `cell_of` + élection d'hôte `cell_host`, posé/testé) · ✅ **8.3b
> (19 juin) : le RÉSUMÉ de cellule** — nouveau `src/net/cell.rs` (`CellSummary` : cellule + count + échantillon
> RÉPARTI de positions ; `build_cell_summary` + `encode/decode`, purs et testés, sur le modèle de `gossip.rs` ;
> `KIND_CELL_SUMMARY = 9`). 65 tests, 0 warning (`#[allow(dead_code)]` documenté tant que non câblé, comme 8.3a).
> ✅ **8.3c (19 juin) : émission/ingestion CÂBLÉES (épidémique, fanout borné).** L'hôte de cellule émet son
> résumé ; chacun le RELAIE (échantillon tournant, comme le gossip) → propagation en log(N) SANS que l'hôte
> n'inonde personne (O(fanout), pas O(N) : ferme le piège « hôte qui fond », relie D4). Réception → `ingest_summary`
> (dernier par cellule, borné `MAX_CELLS`). Métrique simu `summary_perceived` (somme des cellules suivies). **Mesuré
> `crowd 200` (30 s) : perception par résumé moy 112 / max 154 occupants (sur 199) via O(cellules) flux, débit ↓
> ~46 Ko/s borné (+6 vs pré-8.3 = coût des résumés), orbe 0/200, essaim tenu. 65 tests, 0 warning.** *(Les `#[allow(dead_code)]`
> de 8.3a/8.3b sont levés : cell_of/cell_host/CellSummary sont désormais utilisés.)*
> ✅ **8.3d (20 juin) — UN VRAI BUG DE CONCEPTION TROUVÉ ET FERMÉ : les résumés n'avaient pas d'ordre de
> FRAÎCHEUR.** Le diagnostic (foule centrée dans une cellule + fenêtre longue) a révélé que la perception
> EMPIRAIT avec le temps (`crowd 200` : 112→77 ; `crowd 500` : 18→11 entre 30 s et 60 s). Cause : `ingest_summary`
> faisait « dernier arrivé gagne » → une VIEILLE copie partielle (count faible, d'avant que l'hôte connaisse toute
> la foule) qui circule encore via les relais ÉCRASAIT la fraîche et complète. Plus la fenêtre est longue, plus il
> y a de vieilles copies en vol → pire. **Fix (pas une rustine) :** chaque résumé porte un `ts` (horodatage), les
> relais le portent VERBATIM, et `ingest_summary` ne garde que le PLUS FRAIS par cellule (`ts` strictement
> supérieur) — c'est l'anti-rejeu `accept_seq` des états, appliqué aux résumés. **PROUVÉ par le même diagnostic
> rejoué :** la perception ne s'effondre plus, elle CONVERGE vers N — `crowd 200` 160→**191/200**, `crowd 500`
> 88→**477/500** (30 s→60 s) ; et l'INVARIANT tient : N×2,5 (200→500) garde le débit ↓ **PLAT (~45 Ko/s)**. Orbe
> 0, essaim tenu. **66 tests, 0 warning.** *(Doute #1 levé : foule centrée → un hôte → count ≈ N.)*
> **⚠ DOUTES RESTANTS (écrits noir sur blanc) :**
> 1. **Convergence non instantanée à l'échelle.** À 30 s / 500 nœuds, la moyenne n'est qu'à 88 (max 328) ; il faut
>    ~60 s pour atteindre 477. Pas un bug — temps de propagation épidémique en log(N), constante non négligeable.
>    À surveiller en poussant vers 2000+ (la fenêtre de preuve doit grandir avec N).
> 2. **FRAÎCHEUR pas encore CHIFFRÉE en propre.** La perception ≈ N à débit plat le démontre INDIRECTEMENT (les
>    résumés sont assez frais pour couvrir la foule), mais l'âge moyen de perception d'un lointain AVEC vs SANS
>    résumés n'est pas encore une métrique dédiée. *À faire si on veut la preuve directe du « 1/N tué ».*
> 3. ✅ **INVARIANT À L'ÉCHELLE COURU (20 juin) — prouvé jusqu'à la LIMITE DU BANC.** `crowd 1000` (90 s) :
>    ↑ émis **39,8 Ko/s (cap budget, max 47)**, ↓ reçu 39,4 Ko/s — la réception NE GRANDIT PAS avec N (200→1000
>    restent ~40-47 Ko/s) → O(budget), indépendant de N ; perception moy **775 / max 839 sur 999** ; couverture
>    80 %. **À `crowd 2000` (90 s) le débit CHUTE (22,7 Ko/s, couverture 22 %) — ARTEFACT MONO-PC CONFIRMÉ PAR
>    L'ARITHMÉTIQUE, pas une limite du protocole :** la mesure dit **1 nœud ≈ 1 % d'un cœur**, et la machine a
>    **12 cœurs** (`nproc`) → 2000 nœuds = ~20 cœurs demandés sur 12 → sur-souscription ×1,7 → threads affamés →
>    ils tiquent au ralenti → émettent/reçoivent moins. Le mur du banc est ~**1200-1500 nœuds** ici. **La perception
>    MAX suit quand même N partout (200→500→903→1805).** *(challenge de la feuille :* le « échelle 5000 » littéral
>    de D22 est bloqué par le BANC — un OS-thread par bot — PAS par le protocole. Pour prouver 5000+ sur UN PC il
>    faudrait un simulateur léger (ordonnancement coopératif, N bots/thread) → dette de HARNAIS, voir registre. Le
>    protocole, lui, est démontré : budget capé, perception ∝ N.)*
>
> **▶ 8.3d FERMÉ → le chapitre 8 « foule dense » (Phase A) est BOUCLÉ. PROCHAINE ACTION = Phase B (inclusivité,
> D3/D4/D5) OU chapitre 10 (vie privée + identité) — à arbitrer (les deux ferment des doutes proches de la vision).**
>
> ### 🔬 APRÈS le ch.8/9 — PASSE DE VALIDATION par simulation (proposée par l'utilisateur, avant le ch.10)
> *Finir au PROPRE les chapitres 8 et 9, puis — AVANT d'attaquer le ch.10 — une passe dédiée qui POSE DE VRAIS
> RÉSULTATS MESURÉS sur chaque doute du tableau d'audit (§C) : on ne se contente pas de DÉCLARER « fermé », on
> rejoue l'attaque/la simu et on colle le chiffre. Objectif : que chaque ✅ de l'audit soit adossé à une preuve
> reproductible (orbe 0, sourdines, framing ÉCHOUÉ, débit plat, perception ∝ N…), pas à une affirmation.*

**Le cœur dur de D9 (Sybil + éclipse + framing) est tenu.** Détail du chapitre en §D, Chapitre 9.

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

> ### 📊 AUDIT DES DOUTES — état honnête au 20 juin (ce tableau fait FOI ; l'analyse détaillée suit)
> *Statut : ✅ fermé & prouvé · 🟡 borné/partiel (vit, à finir) · 🔴 ouvert (chapitre dédié). **9 fermés** (D22
> au 8.3d, D14 au 10.1), **7 bornés** (+ D25 banc d'essai), **10 ouverts** (+ D26 agrégateur menteur) — sur 26 doutes.
> Les ouverts se regroupent PROPREMENT en chantiers, ce qui confirme le plan (voir sous le tableau).*
>
> | # | Statut | Où en est-on / qui le ferme |
> |---|---|---|
> | **D1** tests = localhost | ✅ | ch.7 : `tc netem` + NAT réel en namespaces (7.1→7.5) |
> | **D2** bot ≠ jeu | 🟡 | décisions de confiance partagées ; orchestration encore dupliquée → ch.12.2 |
> | **D3** lien faible noyé (réception) | 🔴 | inclusivité, **ch.8 Phase B** (8.4/8.6) |
> | **D4** économie du parent (free-riding) | 🔴 | inclusivité, **ch.8 Phase B** (8.7) |
> | **D5** parent qui censure | 🔴 | inclusivité, **ch.8 Phase B** (8.8) |
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
> | **D16** fuites mémoire long terme | 🔴 | longévité, **ch.12.1** (TTL/éviction) |
> | **D17** NAT symétrique | 🔴 | longévité, **ch.12.3** (relais/IPv6) |
> | **D18** speed-hack grossier | 🟡 | à surveiller, **ch.11.4** |
> | **D19** coût réel jamais mesuré | ✅ | 7.4/7.4b : Ko/s ↑↓, %CPU, RAM par nœud |
> | **D20** attaques combinées | 🟡 | `sim` lance plusieurs attaquants en // ; pas encore de scénario coordonné adaptatif |
> | **D21** sécurité du rendez-vous | 🟡 | 9.5a (cap mémoire + PoW) ; reste rate-limit débit + anti-spoofing |
> | **D22** foule dense (aveugle > 32) | ✅ | 8.1 (plafond cassé) + 8.2 (deux tiers) + 8.2c (rendu) + **8.3d (résumés frais, invariant prouvé jusqu'au banc ~1500 ; 5000 littéral = dette de HARNAIS, pas du protocole)** |
> | **D23** gossip = ampli DDoS | ✅ | 8.1b : prouvé par `attack gossip-flood` |
> | **D24** foule visible plafonnée 64 | ✅ | 8.2c : rendu deux tiers, confirmé à l'écran |
> | **D25** banc d'essai plafonné (~1500) ; « 55k » non mesuré DIRECTEMENT | 🟡 | invariant prouvé à 1000 + argument d'archi (budget capé) ; 5000+ = simulateur léger à bâtir (dette harnais) |
> | **D26** agrégateur/parent MENTEUR (hôte de cellule qui ment sur sa région) | 🔴 | nouvelle surface ouverte par 8.3 ; corroboration multi-informateurs **à fermer AVANT Phase B** (ex-« 8.8 ») |
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
> - **D4 — l'INCITATION (« pourquoi relayer pour les faibles ? »).** Pas du code : du *mécanisme*. Le pilier
>   « les forts aident les faibles » est aspirationnel tant que rien ne récompense le relais. Conditionne D3 + D17.
>   C'est le vrai mur intellectuel du projet. À traiter en TÊTE de Phase B, pas à la fin.
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

**D4 — L'économie du parent n'est pas résolue (free-riding).** 🔴 `[ch. 8]`
*Constat :* *pourquoi* un nœud à bon upload dépenserait-il sa bande passante pour les
autres ? Sans réponse, tout le monde se déclare « faible » et personne ne relaie.
*Pourquoi :* c'est LE problème de mécanisme du « partage de puissance ». *Piste :*
réciprocité façon **BitTorrent** (donnant-donnant : je relaie surtout pour ceux qui me
rendent service / ont bonne réputation ; « optimistic unchoke » pour amorcer), la
réputation devenant une quasi-monnaie. *Vérif :* en simu, des nœuds égoïstes (qui ne
relaient jamais) obtiennent un service dégradé ; les coopératifs, un bon service.

**D5 — Un parent malveillant censure en silence (disponibilité ≠ intégrité).** 🟠 `[ch. 8]`
*Constat :* la signature garantit qu'un parent ne *falsifie* pas ton état, mais rien ne
l'empêche de le *jeter* — tu deviens invisible. *Piste :* plusieurs parents en
parallèle (redondance), + détection du drop (si mes voisins ne confirment jamais avoir
reçu mon état via le parent, je change de parent). *Vérif :* un parent « trou noir »
est détecté et contourné en N secondes.

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

**D26 — L'AGRÉGATEUR (hôte de cellule / parent) peut MENTIR sur sa région.** 🔴 `[ch.8 Phase B — corroboration]`
*Constat (nommé le 20 juin, ouvert par 8.3) :* depuis 8.3, un **hôte de cellule** produit le RÉSUMÉ de la foule de
sa zone (combien, où) ; en Phase B, un **parent** agrégera et dégradera la foule pour ses protégés faibles. Or rien
ne l'empêche de **CACHER des gens** (te rendre aveugle à une partie de la foule = éclipse douce) ou d'en **INVENTER**
(fantômes). La signature prouve l'AUTHENTICITÉ d'un état individuel, pas l'HONNÊTETÉ d'un *résumé* (qui agrège des
tiers). C'est **D5/D9 vus depuis la couche d'agrégation** — une surface NOUVELLE que la feuille renvoyait vaguement à
« 8.8 ». *Pourquoi ça compte :* toute la Phase B (inclusivité) s'appuie sur des agrégateurs ; si on peut mentir
dessus, on bâtit l'inclusivité sur du sable. *Atténué aujourd'hui :* un résumé est **CONSULTATIF, pas autoritaire**
(deux hôtes en désaccord = deux flux, aucune corruption d'état) ; le coût Sybil/éclipse est déjà durci (ch.9). *Piste :*
**corroboration multi-informateurs** — ne croire un résumé que s'il est recoupé par plusieurs sources indépendantes
(diversité IP, comme 9.4b) ; détecter le « trou noir » (mes voisins ne confirment jamais ce que le parent prétend
relayer → je change de parent, D5). *Vérif :* un hôte/parent qui cache ou invente >X % de sa cellule est détecté et
contourné en N secondes ; un protégé derrière un parent menteur garde une perception corroborée. **À fermer EN TÊTE
de Phase B, avant de s'appuyer sur les parents.**

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
**Ferme :** D2, D16, D17. **Vérif :** simu longue (mémoire stable) ; cas NAT symétrique OK.

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
