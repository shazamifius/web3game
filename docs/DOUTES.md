# 🤔 L'INVENTAIRE DES DOUTES (D1 → D27) — le cœur

> Tous les doutes/risques d'ingénieur, numérotés, et par quel chapitre chacun se ferme.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

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

## 💬 RÉPONSES & PRÉCISIONS — session du 26 juin 2026

> Long échange avec l'utilisateur, doute par doute. **RÈGLE POSÉE PAR LUI : un doute n'est pas RÉSOLU parce
> qu'il reçoit une réponse — il garde son statut 🔴/🟡 tant que ce n'est pas PROUVÉ.** Une réponse est une
> PISTE (il peut y en avoir plusieurs). On range ici la réponse en regard de chaque Dn ; le détail des doutes
> reste dans les catégories ci-dessous (texte d'origine intact = honnêteté).

- **D2** (bot ≠ jeu) — *se ferme par construction avec le SIDECAR* : le cœur Rust devient l'UNIQUE boucle,
  Unreal ET le bot le pilotent → plus de double orchestration. À acter quand le sidecar porte le vrai jeu.
- **D3** (lien faible noyé en réception) — *rendre l'AoI BILATÉRALE* : le receveur annonce un budget
  (« max X Ko/s »), les émetteurs s'y plient (water-filling bilatéral) ; + LOD adaptatif (afficher MOINS) ;
  + parent qui agrège pour les très faibles. = le bas de la courbe d'adaptativité (cf. D29/D30).
- **D4+D5** (parent) — *clusters (= cellules) auto-formés ~40, qui se parlent ; parent = celui qui LIVRE le
  mieux*. **3 précisions dures :** (1) **PAS de chef de cluster** (mesuré 8.3★ : l'élection = le mur dominant) ;
  (2) capacité **MESURÉE, pas déclarée** (infalsifiable) ; (3) diffusion bornée AU cluster, jamais aux 55K.
- **D6** (PoW jouet) — *abandonner la PoW façon Bitcoin ; sanction = INVISIBILITÉ ; modèle « naître invisible,
  GAGNER sa visibilité »* → ferme le Sybil SANS PoW (la nouvelle identité d'un banni est aussi worthless).
  ⚠ Sans plancher de coût (ici SOCIAL), l'évasion de ban est gratuite — le « gagner sa visibilité » EST ce plancher.
- **D10** (rendez-vous central) — *trop prématuré (post-RPO)*. Direction : téléchargement = BitTorrent ;
  hébergement = machines bénévoles pondérées par l'estime. **Précision honnête actée : ce ne sera JAMAIS
  « zéro serveur » — c'est « pas de serveur central AUTORITAIRE ».** On arrête de se vendre « sans serveur » tout court.
- **D11** (migration orbe) — *quorum de voisins pour valider une reprise* ; **MAIS les étoiles l'ont déjà
  CONTOURNÉ** (rang convergent SANS maître ni migration). Reco : généraliser le modèle « étoiles » (sans maître)
  plutôt que « orbe » (avec migration) partout où c'est possible.
- **D12** (tout codé pour 1 objet) — *« exactement ce qu'on doit faire maintenant »* (utilisateur) : le jeu = banc
  pour TOUS les systèmes (objet sous gravité, portable, ramassable). Reco concrète : **registre générique**
  `{id, type, règle d'autorité, état}` ; on a déjà 2 patrons (orbe = migration, étoile = rang convergent) →
  extraire le registre commun en câblant.
- **D13** (pas d'horloge commune) — *le départage (version, id) suffit aux jeux cozy ; seul le twitch-compétitif
  a besoin du vrai temps* → service du SUBSTRAT activé par le profil « compétitif » (rejoint l'idée d'un langage
  de plugins visuel : plus de plugins = plus compétitif). ⚠ Un plugin ne CRÉE pas une horloge ; le substrat la fournit.
- **D15** (tout en clair) — *chiffrement à la TOUTE FIN (sortie publique), jamais en R&D* (il cache les paquets
  → tue la compréhension/debug). ✅ bon sens. ⚠ Re-mesurer débit/latence APRÈS (paquets + handshake bougent les chiffres).
- **D16** (fuites mémoire) — *du MÉNAGE, pas de la vision* ; éviction TTL faite ; zéro impact MVP. Ignoré sans
  risque jusqu'aux longues sessions massives.
- **D17** (NAT symétrique) — *le RELAIS, ressuscité PORTEUR (plancher de D34)*. **Précision majeure (incohérence
  doc levée) : le relais via rendez-vous est DÉJÀ PROUVÉ EN UNREAL** (palier 3 du contrat sidecar — un humain en
  NAT symétrique mobile rendu via relais). Reste l'OUVERT : relais DÉCENTRALISÉ + déclenchement auto « perçage
  abandonné → relais ». **D17 + D27 = le même test terrain.**
- **D18** (speed-hack grossier) — *anticheat RÉGLABLE par le créateur (curseur agressif/doux)*. **2 couches :**
  (1) anticheat qui protège le SUBSTRAT (signatures, anti-Sybil, anti-forge) = TOUJOURS actif, non négociable ;
  (2) anticheat de RÈGLES DE JEU = réglable. Le curseur s'applique à la couche 2.
- **D20** (attaques combinées) — *objectif LONG TERME* (mode « adversaire malin coordonné »). Prématuré.
- **D21** (sécurité rendez-vous) — *réutiliser des outils tout-faits* (rate-limit / anti-spoofing existants,
  performants). ⚠ Eux gèrent le volumétrique BRUT ; les caps APPLICATIFS (éviction, PoW à l'entrée) restent chez nous.
- **D25** (banc plafonne ~1500 ; 55K non direct) — *« on n'aura jamais 15000 nœuds, on aura 33 »* : du point de
  vue d'UN joueur, le monde = **32 voisins détaillés + 1 foule agrégée**, toujours. Le total ne touche jamais une
  machine → le « 1500 » est un artefact du SIMULATEUR (un thread OS/bot), PAS un mur du jeu réel (ni du kernel).
  **2 précisions :** (a) le nœud #33 (foule) est nourri par les résumés/gossip → sa QUALITÉ + HONNÊTETÉ = D22/D26/D29/D30 ;
  (b) **visibilité MUTUELLE garantie pour les paires qui interagissent** (sinon A salue B qui ne voit pas A — budgets asymétriques).
- **D26** (résumé menteur) — *= le « vérifié par le cluster » du D4, version générale : ne jamais croire UN
  nœud sur un GROUPE, recouper*. Partiellement adressé par le RETRAIT du chef (8.3★, résumé auto-certifiant).
  **L'estime corroborée (D6) le nourrit : un fantôme à zéro estime est INCOMPTABLE.**
- **D28** (persistance sans serveur) — *MVP = éphémère/local (trichable, mais entre amis OSEF — NE PAS résoudre
  maintenant)* ; vraie solution « ta clé signe ton état, les pairs corroborent » roule sur le MOTEUR D'ESTIME
  (un saut à 1M cristaux non corroboré = rejeté).

> **🔭 LA RÉCOLTE DE LA SESSION — un seul mécanisme pour six doutes.** L'**ESTIME SOCIALE CORROBORÉE**
> (« naître invisible, gagner sa visibilité ; les pairs recoupent les déclarations ») répond à **D6, D26, D11,
> D28, D4/D5** — la plus belle simplification du projet : un substrat de corroboration au lieu de six défenses
> séparées. ⚠ **Revers à garder en lumière :** un mécanisme qui porte six défenses = un POINT UNIQUE DE
> DÉFAILLANCE CONCEPTUELLE ; et la corroboration P2P est le mur ouvert le plus dur (D26 couche 2). Élégance
> réelle, MAIS risque concentré.
> **🚫 LEÇON « PAS DE CHEF ».** Toute conception de foule/cluster gravite vers l'élection d'un chef ; nos
> MESURES (8.3★) disent NON (l'élection était le mur dominant, perception 91 %→10 %). À re-rappeler dès que l'idée revient.

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

**D28 — La PERSISTANCE d'état JOUEUR en P2P sans serveur.** 🔴 `[doute de conception, ouvert — posé le 25 juin]`
*Constat :* le 1er jeu suppose une **progression PERSISTANTE et LENTE** (cristaux, race animale, stats faim/stamina
qui montent au fil des sessions). Or « **pas de serveur de jeu** » + « **progression long terme** » sont en **tension
de fond** : sans magasin canonique, **où vit l'état d'un joueur entre deux sessions, et qui empêche de le forger ?**
Le proto Bevy le sauvait dans un **fichier LOCAL** (`~/.web3game/ile_score.txt`) → trivialement trichable ET ne
synchronise rien. *Distinct de D14* (qui ne persiste que l'**identité** = la clé, pas l'**état de jeu**). *Pistes
non tranchées (= un fork qui touche la vision, pas juste la technique) :* (a) **éphémère assumé** (l'état ne survit
pas à la session — simple et honnête, mais tue la « progression lente ») ; (b) **« ta clé signe ton état »** (tu
portes ton état signé, les pairs corroborent — anti-forge partielle, mais le vol/dup d'objets *uniques* reste dur en
P2P) ; (c) **petit serveur de SAUVEGARDE assumé** (un astérisque de plus à « sans serveur », comme le rendez-vous) ;
(d) un mélange (perso éphémère pour le MVP, persistance corroborée plus tard). *Vérif (quand on tranchera) :* un
joueur revient avec son état d'avant, et un état forgé/dupliqué est rejeté/borné. **À NE PAS résoudre maintenant** :
pour le MVP « 10 potes » l'éphémère/local suffit ; ce doute existe pour ne pas se mentir sur « progression » + « sans serveur ».

### Catégorie 11 — Perception, échelle & onboarding (nés le 26 juin 2026)

> Doutes sortis de l'échange « densité de foule » + la révélation du cap RPO (l'OASIS). Le fil rouge :
> **l'architecture réseau doit s'ADAPTER aux créateurs, jamais l'inverse** (→ devenu un PRINCIPE, cf.
> `PRINCIPES.md` A.8). Rappel de la règle de la session : une RÉPONSE n'est pas une CLÔTURE.

**D29 — L'AoI par PROXIMITÉ ≠ par PERTINENCE sociale.** 🟠 `[chantier AoI-pertinence, Phase 2]`
*Constat :* on sélectionne les 32 plus PROCHES (`keep_nearest`, [rendezvous.rs](../src/net/rendezvous.rs)),
pas les 32 qui COMPTENT. Si A près de moi parle à B (loin, hors de mes 32), B m'est INVISIBLE alors qu'il est
socialement pertinent → situations gênantes. *Piste :* ensemble d'intérêt = union de niveaux — **T0** voisins
spatiaux (borné par BUDGET, plus de cap dur 32) ; **T1** partenaires d'interaction de T0 tirés par TRANSITIVITÉ
(chaque état signé annonce « engagé avec {≤4 ids} ») ; **T2** social explicite (amis). *Limite honnête :*
amorçage (on ne classe pas ce qu'on ne voit pas) + le plafond ne DISPARAÎT pas à 55K (physique) — il devient
INTELLIGENT. *Dette :* T1 ouvre une variante D26 (« mentir sur qui je fréquente ») — bénin (n'attire que MA
bande passante vers de vrais pairs). *Vérif (ligne d'arrivée headless) :* B (engagé avec mon voisin) entre dans
mon set en ≤ X ticks ; C (loin, non engagé) non. **C'est le chantier que JE prends en Phase 2.**

**D30 — LOD NON-ADAPTATIF + foule JAMAIS rendue + distribution sim irréaliste.** 🟠 `[chantier AoI-pertinence, Phase 2]`
*Constat :* (a) le système saute brutalement « au-delà de 32 = blob », que N=40 ou 55K — **bourrin** ; (b) on
n'a JAMAIS regardé une foule rendue au-delà de 32 ; (c) dans `sim` les bots sont co-localisés dans 3 m = un
NUAGE de points, PAS une foule réaliste (sous-groupes, cercles, scène). *Piste :* fidélité CONTINUE =
f(N visibles, bande passante) — sous budget : tout le monde NET ; au-dessus : dégradation continue
**net → imposteur → CHAMP de densité/flux** (« la foule va par là »). = l'« auto-équilibré selon N » de
l'utilisateur. *Vérif :* N=40 → 40 nets ; N=500 → ~100 nets + reste résumé, débit reçu plat ; **une foule
réellement REGARDÉE** (jamais fait).

**D31 — La map ne doit JAMAIS être une barrière → DEVENU UN PRINCIPE.** ✅→principe `[PRINCIPES.md A.8]`
*Constat (mon erreur) :* j'avais proposé de contraindre le level-design pour ménager le netcode. *Renversé par
l'utilisateur (cap RPO/OASIS) :* n'importe qui bâtira n'importe quel monde → **le réseau ne peut JAMAIS
contraindre le créateur ; toute la charge (échelle, géométrie, densité) est sur l'archi ADAPTATIVE.** Quitte le
registre, entre dans les principes. *Borne honnête :* « adaptatif » = dégradation gracieuse **jusqu'à la limite
physique**, PAS capacité infinie — le créateur n'est jamais BLOQUÉ, l'expérience extrême DÉGRADE (jamais ne casse,
jamais n'exclut).

**D32 — « Le jeu est-il FUN ? » → hors PÉRIMÈTRE.** 🟡→périmètre `[clarification]`
*Constat :* on doute de tout le réseau, jamais du cœur ludique (même angle mort que D27, un cran plus haut).
*Réponse de l'utilisateur :* le fun ne se TESTE pas, il se BÂTIT — et ce n'est pas notre métier (on fait la R&D
du SUBSTRAT ; le fun est le métier des créateurs, cf. RPO). Métrique qui compte = **latence ≤ 500 ms = jouable,
au-delà = mort**. *Reclassement :* l'« île aux étoiles » n'est plus « le produit » mais un **INSTRUMENT** (faire
venir des humains pour mesurer le substrat) qui DEVIENDRA un produit poli plus tard.

**D33 — « Un seul cœur pour le jeu RICHE ET le 55K MINCE ? » → veille périphérique.** 🟡 `[veille]`
*Constat :* l'état RICHE (cristaux/races/stats persistants) et l'état MINCE (positions/LOD à 55K) sont deux
régimes de code (rejoint D12). *Réponse de l'utilisateur :* une seule étoile (l'OASIS), des petites autour pour
l'atteindre ; la ligne droite = la mort ; la vision PÉRIPHÉRIQUE (les pauses) est l'outil pour surveiller que
riche et mince COHABITENT au lieu de forker. *Statut :* ni fermé ni ouvert — **point de veille** explicite.

**D34 — L'ONBOARDING pourrait AFFAMER chaque test.** 🟠 `[stratégie posée]`
*Constat :* pour jouer, un pote doit installer un launcher + lancer un démon sidecar + percer le NAT + toucher un
rendez-vous → BRUTAL ; le test « est-ce vivant » (D27) est bloqué EN AMONT par « mon pote arrive-t-il à lancer ».
*Stratégie de l'utilisateur :* exposer au monde un jeu BIEN construit → amasser un max de logs sur des MILLIERS de
box → un **launcher auto-réparateur** (répare les fichiers corrompus, télécharge les versions, connaît à terme
toutes les box). *Dépendance DURE (ma précision) :* démarrage à froid — la boucle d'apprentissage n'existe que si
le **RELAIS (D17)** garantit un plancher (box impercible → relayée + loguée → apprise). Promesse honnête : pas
« jamais de mur » mais **« jamais zéro : à défaut de direct, relais »**. Le relais n'est PAS une side-quest : c'est
le socle de cette stratégie.

**D35 — Le VOCAL : mur porteur de « est-ce vivant », le plus différé.** 🟠 `[séquencé sur D29]`
*Constat :* le design « force l'usage du vocal de proximité » → le vocal PORTE D27 ; chaque test humain SANS lui
teste une version mutilée de la prémisse. *Réponse :* « qui j'entends » = « qui est dans mon ensemble de
pertinence » → **la voix de proximité = l'AoI-pertinence (D29) appliquée à l'AUDIO.** Donc ORDRE : AoI-pertinence
(headless) → la voix monte dessus (jours après, pas mois). *La partie Unreal de l'audio (capture / lecture /
spatialisation 3D) est du MOTEUR, indépendante du réseau → spikable EN PARALLÈLE dès maintenant.* (Utilisateur :
« on attend quoi ? » — réponse : pas différé, **séquencé**.)

---

