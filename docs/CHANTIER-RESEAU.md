# 🌐 CHANTIER RÉSEAU — confrontation au réel, confiance, identité (ch. 7, 9, 10)

> Le programme détaillé des chapitres réseau : NAT/latence/perte (7), durcissement de la confiance (9), identité persistante & vie privée (10).
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

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

