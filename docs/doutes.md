# Les doutes — le registre ouvert (et leurs réponses)

> Un projet de R&D honnête se reconnaît à ses **doutes assumés**. Voici l'inventaire complet des risques et
> questions ouvertes, numérotés et suivis — **avec, pour chacun, la réponse ou la piste précise**. *Un doute n'est
> pas une faiblesse à cacher : c'est l'objet du travail.* Et un doute n'est « fermé » que lorsqu'une **mesure** le
> prouve ; sinon, il reçoit une *réponse* (une piste), et son statut reste ouvert.
>
> Voir aussi : [l'idée en clair](comprendre-le-p2p.md) · [l'état chiffré du projet](etat-du-projet.md) · [README](../README.md).

**Statut** — ✅ fermé & prouvé · 🟡 borné / partiel · 🔴 ouvert · 🧭 reclassé (principe, périmètre, ou veille).

> **🔭 Une réponse transversale — l'« estime corroborée ».** Plusieurs doutes (D4/D5, D6, D11, D26, D28) convergent
> vers **un seul mécanisme** : on **naît invisible** et l'on **gagne sa visibilité** par son comportement ; les
> pairs **recoupent** ce que chacun déclare (on ne croit personne sur parole quand il parle d'un *groupe*). Une
> identité jetable ne vaut rien tant qu'elle n'a pas été socialement gagnée → le Sybil, le mensonge d'un agrégateur
> et la forge d'état perdent leur intérêt, **sans monnaie de réputation ni preuve de travail lourde**. *Élégant —
> mais à manier prudemment : un mécanisme qui porte cinq défenses devient un point de fragilité s'il a un défaut.*

---

## Le réalisme des tests

- **D1 — Les tests « mentent » comme du localhost.** ✅ *Une simu sans latence/perte/NAT peut être parfaite et
  s'écrouler sur Internet.* **Réponse :** conditions réseau réelles injectées avec `tc netem` (latence, gigue, perte,
  ré-ordonnancement) + NAT en namespaces (`ip netns`) — une seule machine suffit à reproduire un vrai mauvais réseau.
- **D2 — Le bot de simulation n'est pas exactement le jeu.** 🟡 *Deux boucles réseau dupliquées divergent en silence
  (déjà vu : un coût mesuré faux, 89 au lieu de 34 Ko/s).* **Réponse :** faire piloter au bot **les mêmes fonctions**
  que le client (déjà fait pour le calcul de débit) ; le pont *sidecar* fera du cœur Rust **l'unique boucle**, que le
  bot et le moteur pilotent — la duplication disparaît par construction.

## L'inclusivité (le cœur de la vision)

- **D3 — Un lien faible ne peut pas suivre, en réception.** 🔴 *L'aire d'intérêt borne l'émission, pas la réception :
  en foule un joueur reçoit ~43 Ko/s et se noie.* **Réponse (3 niveaux) :** (1) aire d'intérêt **bilatérale** — le
  receveur **annonce un budget en Ko/s**, les émetteurs le respectent ; (2) **détail adaptatif** — le faible affiche
  moins de voisins nets + la foule en résumé ; (3) pour les très faibles, un **parent agrège** et n'envoie qu'un
  résumé basse fréquence.
- **D4 + D5 — Qui relaie pour les faibles, et comment empêcher un relais « trou noir » ?** 🟡 *Le rôle faible est
  auto-déclaré ; un parent qui jette les paquets rend invisible en silence.* **Réponse :** choisir le parent par
  **capacité réellement observée** (le débit/fraîcheur qu'on reçoit de lui — infalsifiable), **jamais déclarée** ;
  l'affectation est **locale** (chacun, dans sa zone, prend le meilleur parent disponible) ; en pénurie on **dégrade
  équitablement** plutôt que d'exclure ; le relais « trou noir » est détecté quand les voisins ne confirment jamais
  avoir reçu mon état via lui → on change de parent. **Pas de chef imposé.**

## Sybil & réputation

- **D6 — La preuve de travail est un « jouet ».** 🟡 *Miner une identité coûte ~1 s → des centaines en minutes.*
  **Réponse :** plutôt qu'une preuve de travail lourde (mauvaise ici), l'**estime sociale** (cf. encadré) — on naît
  invisible, on gagne sa visibilité ; un tricheur banni qui recrée une identité **repart de zéro**, donc l'évasion
  de ban ne rapporte rien. La sanction ultime, dans un espace social, c'est **l'invisibilité**.
- **D7 — Un quorum d'accusation permet le *framing*.** ✅ *3 identités minées pourraient faire bannir un innocent.*
  **Réponse (faite) :** accusations **pondérées par la crédibilité** de l'accusateur + **diversité d'adresses** (cap
  par sous-réseau /24) → un attaquant seul ne fait pas quorum. Prouvé par l'attaque `sybil-frame`.
- **D8 — Aucune réhabilitation, aucune expiration.** ✅ *Une mise en sourdine était définitive (injuste + fuite
  mémoire).* **Réponse (faite) :** les fautes **décroissent dans une fenêtre glissante** ; un nœud injustement muté
  redevient audible après une période de bon comportement.

## La confiance topologique

- **D9 — La position n'est pas vérifiée → attaque par éclipse.** ✅ *Un nœud pouvait mentir sur sa position pour
  s'insérer dans tous les voisinages et isoler une victime.* **Réponse (faite) :** **diversité forcée** du voisinage
  (ne pas prendre tous ses voisins de la même source/sous-réseau) + **corroboration** des positions + **cap /24**.
- **D10 — Le point de rendez-vous reste une centralisation.** 🟡 *Il voit les adresses ; c'est une panne unique.*
  **Réponse :** borné aujourd'hui (rate-limit + preuve de travail à l'entrée) ; à terme, téléchargement des mondes
  en **BitTorrent** et hébergement par des **machines bénévoles pondérées par l'estime**. *Précision honnête :*
  l'infrastructure restera « **pas de serveur central autoritaire** », jamais « zéro serveur ».

## L'autorité des objets partagés

- **D11 — La reprise d'autorité d'un objet est le point mou.** 🟡 *Un attaquant patient attend le silence du maître
  puis prend l'objet.* **Réponse :** deux pistes — (a) valider la reprise par un **quorum de voisins** (un attaquant
  seul ne fait pas quorum) ; (b) **mieux : se passer de maître**. Une résolution par **rang déterministe convergent**
  (déjà éprouvée sur un cas) attribue l'objet sans maître ni migration, donc rien à voler — c'est ce modèle qu'on
  cherche à généraliser.
- **D12 — Tout est codé pour un seul objet.** 🟡 *Un vrai monde a des milliers d'objets partagés.* **Réponse :** un
  **registre générique** `{id, type, règle d'autorité, état}` ; chaque type d'objet (ramassable, portable, soumis à
  gravité, porte…) branche **sa** règle. Deux patrons existent déjà (autorité par migration ; autorité par rang
  convergent) → on extrait le registre commun au fil du câblage.
- **D13 — Pas d'horloge commune → conflits mal arbitrés.** 🟡 *« Qui a touché en premier ? »* **Réponse :** un
  départage **déterministe (version, identité)** est cohérent pour tous et **suffit** aux jeux calmes ; seul le
  compétitif « à la milliseconde » exige un vrai temps partagé → ce sera un **service optionnel du socle**, activé
  par les jeux qui en ont besoin, jamais imposé à tous.

## Identité & vie privée

- **D14 — L'identité n'était pas persistante.** ✅ *Pas de « compte » d'une session à l'autre.* **Réponse (faite) :**
  la clé est minée **une fois**, sauvée localement (comme une clé SSH, permissions restreintes) et **rechargée** au
  lancement → même identité entre sessions, profils distincts = clés distinctes.
- **D15 — Tout circule en clair.** 🟠 *Positions (et bientôt la voix) lisibles ; la signature prouve l'authenticité,
  pas le secret.* **Réponse :** chiffrement de transport **par paire** (échange de clés X25519). **Volontairement
  différé** à la sortie publique : en R&D le clair facilite la compréhension et le débogage. *Note :* il faudra
  **re-mesurer** débit/latence après (le chiffrement change la taille des paquets).

## Robustesse & longévité

- **D16 — Fuites mémoire à long terme.** 🟡 *Les fiches par pair s'accumulent.* **Réponse (en partie faite) :**
  **éviction par durée de vie** — un pair absent depuis longtemps cède sa place, **jamais un actif**. Du ménage,
  sans impact sur le jeu courant.
- **D17 — NAT symétrique → relais obligatoire.** 🟡 *Certaines connexions (mobile) ne se percent pas en direct.*
  **Réponse :** le **relais via le point de rendez-vous est prouvé en réel** (deux réseaux distincts, dans les deux
  sens). Reste ouvert : un relais **décentralisé** (n'importe quel bon nœud relaie) + un **déclenchement automatique**
  (« perçage abandonné → relais »). C'est le **plancher** d'un onboarding fiable (cf. D34). ⚠️ *Et le relais n'est
  **pas gratuit** : sa qualité dépend du type de lien — une redondance naïve peut même **empirer** un lien saturé (cf. D36).*
- **D18 — Le seuil anti-triche de vitesse est grossier.** 🟡 *Un tricheur subtil reste juste sous le seuil.*
  **Réponse :** deux couches — (1) un **socle non négociable** qui protège le réseau pour tous (signatures,
  anti-Sybil, anti-forge), toujours actif ; (2) des règles de jeu **réglables par le créateur** du monde (curseur
  « anti-triche agressif / souple »), calibrées sur la vitesse réelle de *son* jeu.

## Méta-doutes (sur la démarche)

- **D19 — Le coût réel par nœud n'avait jamais été mesuré.** ✅ **Réponse (mesurée) :** **~34 Ko/s ↑ (max ~38),
  ~31 Ko/s ↓, ~0,7 % d'un cœur, ~38 Mo de RAM** par nœud → ~0,27 Mbit/s, **borné par le voisinage (~32), pas par le
  total** → ne bouge pas à 55 000 (l'échelle se fait en ajoutant des machines).
- **D20 — Attaques combinées / adaptatives jamais testées ensemble.** 🟠 *Un vrai adversaire combine et s'adapte
  (joue honnête puis trahit).* **Réponse :** un mode « scénario » coordonné — **objectif de long terme**, pas urgent.
- **D21 — La sécurité du point de rendez-vous lui-même.** 🟡 *Il peut être inondé de messages valides.* **Réponse :**
  rate-limit + éviction déjà en place ; pour la couche volumétrique brute, **s'appuyer sur des outils éprouvés**
  (anti-DDoS), en gardant **chez nous** les limites *applicatives* (que seul notre code comprend).
- **D22 — La foule dense.** ✅→🟡 **Réponse (partielle) :** le **débit borné est prouvé** (perception ~87 % à 1 000
  nœuds, débit reçu plat). **Mais rouvert :** le *ressenti* et la *pertinence* de la foule ne sont pas prouvés → c'est
  l'objet de D29/D30.
- **D23 — Le gossip est un amplificateur de DDoS.** ✅ **Réponse (faite) :** preuve de travail exigée sur chaque
  « carte de visite », abandon du perçage non corroboré (~10 s), rate-limit d'apprentissage par source — prouvé par
  `gossip-flood` (0 perçage réfléchi).
- **D24 — La foule visible était plafonnée à 64.** ✅ **Réponse (faite) :** rendu « à deux tiers » (quelques proches
  détaillés + une foule d'imposteurs bon marché) → bien plus de 64 silhouettes à l'écran sans chute de performance.
- **D25 — Le banc plafonne (~1 500 nœuds) ; « 55 000 » n'est jamais mesuré directement.** 🟡 *Le banc lance un fil
  d'exécution par nœud → c'est la machine qui sature, pas le protocole.* **Réponse / cadrage :** du point de vue d'un
  joueur, le monde est **toujours ~32 voisins nets + 1 foule agrégée** — le total ne touche jamais une machine. Le
  « 1 500 » est un **artefact du banc**, pas une limite du jeu. Au-delà de ~2 000, c'est de l'**extrapolation
  d'architecture**, jamais « prouvé ».
- **D26 — L'agrégateur d'une foule peut mentir** (cacher ou inventer des gens). 🟡 *La signature prouve qui parle,
  pas que le résumé est honnête.* **Réponse :** ne jamais croire **un seul** nœud sur un **groupe** → **corroborer**
  (recouper K informateurs indépendants + sa propre perception). L'**estime** ferme le reste : un « fantôme » inventé,
  sans estime, est **incomptable**. *(Mesure clé : élire un « chef de cellule » était le mur dominant — on l'a retiré.)*

## Périmètre — les doutes qui peuvent décider du projet

- **D27 — « La forteresse vide ».** 🟡 *A-t-on bâti une belle infrastructure dans laquelle deux humains ne se sont
  jamais vraiment retrouvés, en mouvement, via le vrai Internet ?* **Réponse :** il n'y en a qu'une honnête — **le
  test dehors**, critère pré-enregistré (latence ≤ 500 ms = jouable), conditions hostiles incluses. **Premier fait
  dur (28 juin) :** un **instrument de mesure** (un agent que des volontaires lancent) a relevé, sur de **vrais liens
  distants** (plusieurs pays, dont CGNAT), une présence **vivante** — fraîcheur **p95 ~200–335 ms** (< 500 ms),
  **perte réelle ~0**, verdict « vivant ». L'infrastructure n'est donc **plus vide** : le substrat transporte de la
  présence distante réelle, vivante, sur le vrai Internet. **Ce qui reste (et garde le doute ouvert) :** ça mesure
  le **substrat**, pas le **ressenti** — des humains qui **bougent et jouent ensemble** et le **sentent** vivant (et
  le rôle de la voix, D35). Le doute s'allège ; il ne se ferme pas.
- **D28 — La persistance d'état joueur sans serveur.** 🔴 *Une progression qui survit aux sessions, sans magasin
  central : où vit l'état, et qui empêche de le forger ?* **Réponse :** pour un premier essai, **éphémère/local**
  suffit (entre amis). La vraie réponse — « **ta clé signe ton état, les pairs corroborent** » — s'appuie sur
  l'estime : un saut soudain et non corroboré (p. ex. un inventaire qui explose) est **rejeté**.

## Perception, échelle & onboarding

- **D29 — L'aire d'intérêt par *proximité* ≠ par *pertinence sociale*.** 🟠 *Voir « les 32 plus proches » n'est pas
  voir « les 32 qui comptent » : si un voisin parle à quelqu'un hors de vue, cette personne reste invisible — gênant.*
  **Réponse :** un ensemble d'intérêt **par niveaux**, recalculé en continu — (T0) voisins spatiaux (bornés par un
  *budget*, plus un plafond arbitraire) ; (T1) **partenaires d'interaction** des voisins, tirés par **transitivité**
  (chaque état signé annonce « je suis engagé avec {quelques identités} ») ; (T2) relations explicites (amis).
  *Limite assumée :* on ne classe pas ce qu'on ne perçoit pas encore, et un plafond physique demeure à 55 000 — il
  devient simplement **intelligent**.
- **D30 — Le niveau de détail n'est pas adaptatif, et la foule n'a jamais été *rendue*.** 🟠 *Le système passe
  brutalement de « net » à « rien » au-delà du voisinage, que la foule fasse 40 ou 55 000 ; et personne n'a encore
  regardé une foule rendue.* **Réponse :** une fidélité **continue** — sous le budget, tout le monde net ; au-dessus,
  dégradation **net → silhouette → champ de densité** (« la foule va par là ») — adaptée au matériel et au lien de
  chacun, avec **visibilité mutuelle garantie** pour les paires qui interagissent.
- **D31 — La géométrie d'un monde ne doit jamais brider le réseau.** 🧭 *Devenu un principe.* **Réponse :** puisque
  n'importe qui bâtira n'importe quel monde, **le réseau s'adapte aux créateurs, jamais l'inverse** ; il dégrade
  gracieusement (jusqu'à la limite physique), ne bloque ni n'exclut jamais.
- **D32 — « Le jeu est-il *fun* ? »** 🧭 *Hors périmètre.* **Réponse :** le fun se conçoit et se bâtit (métier des
  créateurs) ; ce dépôt fait la R&D du **substrat**. La grandeur qui le concerne est la **latence** (≤ 500 ms).
- **D33 — Un seul cœur peut-il servir l'état *riche* d'un jeu ET l'état *mince* d'une foule de 55 000 ?** 🧭 *Point
  de veille.* **Réponse :** ce sont deux régimes techniques ; on **surveille** qu'ils cohabitent au lieu de « forker »,
  plutôt que de trancher trop tôt.
- **D34 — L'onboarding pourrait affamer chaque test.** 🟠 *Installer, traverser les box, se retrouver : un mur
  d'usage réel qui peut faire échouer un test « parce que personne n'a réussi à se connecter ».* **Réponse :** un
  lanceur qui **apprend, au fil des connexions réelles, à franchir un maximum de configurations** ; avec le **relais
  (D17) comme plancher** garantissant « jamais zéro : à défaut de direct, relais ».
- **D35 — La voix de proximité, mur porteur du « est-ce vivant ? ».** 🟠 *Le ressenti social repose en partie sur
  elle, et c'est la pièce la plus différée.* **Réponse :** « qui j'entends » = « qui est dans mon ensemble de
  pertinence » → la voix de proximité **est** l'aire d'intérêt (D29) appliquée à l'audio, à bâtir **par-dessus** ce
  chantier (la capture/spatialisation côté moteur, elle, peut être préparée en parallèle).
- **D36 — La diversité des connexions est un mur qu'on n'a pas cartographié.** 🔴 *On n'a validé le transport que sur
  une poignée de liens ; or chaque type de connexion casse **différemment**, et le vrai risque est celui qu'on n'a pas
  encore croisé.* **Constat dur (29 juin) :** sur un **vrai lien mobile dégradé**, ajouter de la **redondance d'émission**
  (envoyer chaque état en double sur le relais) n'a **pas** réduit la perte — elle l'a **empirée**. La leçon : quand la
  perte vient de la **congestion** (débit saturé), dupliquer **aggrave** ; la redondance n'aide que la perte **aléatoire**
  (un lien qui a encore de la marge). **Réponse / pistes :** (1) **caractériser chaque lien** à l'arrivée — latence,
  gigue, **nature** de la perte (aléatoire vs congestion), type de NAT, débit soutenable ; (2) **redondance ADAPTATIVE** —
  ne dédoubler que si la perte est aléatoire *et* qu'il reste de la marge, **jamais** sur un lien saturé ; (3) **dresser
  la carte des régimes** qui posent problème — mobile congestionné, **satellite (Starlink : CGNAT + pics de latence + IP
  qui changent ; géostationnaire : > 500 ms par nature)**, wifi public (perte aléatoire), réseaux qui **bloquent l'UDP**
  (entreprise/hôtel), double-NAT, bascule entre antennes en mouvement. *Honnêteté de méthode : c'est le hasard d'un lien
  de test médiocre qui nous l'a montré — sans lui, on ne l'aurait pas vu. D'où ce doute, ouvert exprès.*

---

*Ce registre est vivant : on y ajoute un doute dès qu'on en découvre un, et la réalité — une mesure, un test, un
vrai joueur — a toujours raison contre ce document.*

> **🔎 Parcours « juger vite » →** étape suivante : **[Le journal de développement](journal.md)**.
