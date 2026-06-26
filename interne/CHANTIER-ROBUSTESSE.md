# 🛡️ CHANTIER ROBUSTESSE & DIVERS (ch. 11, 12, 14)

> Autorité généralisée & ordre (11), robustesse/longévité/unification (12), portabilité moteurs (14).
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

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
> OKLM** (O haut/L bas/K gauche/M droite, axe additif), **menu Contrôles v1** (UMG, ouvert par Tab :
> sliders sensibilité souris/OKLM + inverser souris, persistés via `UGameUserSettings`). Reste pour le
> menu : **remap des touches (clavier + manette/Steam Deck)** via Enhanced Input Player Mappable Keys.
> Carnet technique « où on trouve quoi » = `COMPREHENSION_UNREAL.md` (gitignoré).

### Chapitre 13 — Voix spatiale
**But :** chat vocal P2P, priorité au volume (loudness priority), spatialisé. Profite du
chiffrement (10.2) et de l'inclusivité (ch. 8 — la voix s'adapte au lien).

### Chapitre 14 (plus tard, pas maintenant) — Portabilité moteurs
**But :** extraire un `net-core` portable (ABI C) pour Unreal/Unity. **Décidé : reporté.**

---

