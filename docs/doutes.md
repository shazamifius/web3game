# Les doutes — le registre ouvert

> Un projet de R&D honnête se reconnaît à ses **doutes assumés**. Voici l'inventaire complet des risques et
> questions ouvertes, numérotés et suivis. **Un doute n'est pas une faiblesse à cacher : c'est l'objet du travail.**
> Quand l'un est traité, on écrit aussi ce qui **reste** non résolu — un doute reçoit une *réponse* (une piste),
> il n'est « fermé » que lorsqu'une mesure le prouve.
>
> Voir aussi : [l'idée en clair](comprendre-le-p2p.md) · [l'état chiffré du projet](etat-du-projet.md) · [README](../README.md).

**Légende de statut** — ✅ fermé & prouvé · 🟡 borné / partiel (vit, à finir) · 🔴 ouvert (chantier dédié) ·
🧭 reclassé (devenu principe, périmètre, ou point de veille).

---

## Le réalisme des tests

- **D1 — Les tests « mentent » comme du localhost.** ✅ *Une simu sur une seule machine, sans latence ni perte ni
  NAT, peut être parfaite et s'écrouler sur Internet.* Traité : conditions réseau réelles injectées avec `tc netem`,
  et NAT en namespaces (`ip netns`) — une seule machine suffit.
- **D2 — Le bot de simulation n'est pas exactement le jeu.** 🟡 *Deux boucles réseau dupliquées peuvent diverger en
  silence (c'est arrivé : un coût mesuré faux, 89 au lieu de 34 Ko/s).* Direction : un cœur de session unique que le
  bot et le client pilotent — le pont *sidecar* va naturellement le fermer.

## L'inclusivité (le cœur de la vision)

- **D3 — Un lien faible ne peut pas suivre, en réception.** 🔴 *L'aire d'intérêt borne ce qu'on émet, pas ce qu'on
  reçoit : en foule dense un joueur reçoit ~43 Ko/s et se noie.* Direction : rendre l'aire d'intérêt **bilatérale**
  (le receveur annonce un budget que les émetteurs respectent) + dégradation de détail + agrégation par un relais.
- **D4 + D5 — Qui relaie pour les liens faibles, et comment empêcher un relais « trou noir » ?** 🟡 *Le rôle faible
  est aujourd'hui auto-déclaré ; un parent qui jette les paquets rend invisible en silence.* Direction : choisir le
  parent par **capacité mesurée (infalsifiable), pas déclarée** ; dégrader équitablement en pénurie ; réputation
  réservée aux tricheurs.

## Sybil & réputation

- **D6 — La preuve de travail est un « jouet ».** 🟡 *Miner une identité coûte ~1 s → on en fabrique des centaines.*
  Piste explorée : remplacer une économie de coût pur par une **estime sociale** — on « naît invisible » et l'on
  gagne sa visibilité ; un banni qui recrée une identité repart de zéro (l'évasion de ban ne gagne rien).
- **D7 — Un quorum d'accusation permet le *framing*.** ✅ *3 identités minées pourraient faire bannir un innocent.*
  Traité : accusations pondérées par la crédibilité + diversité d'adresses (cap par sous-réseau) — prouvé par
  l'attaque `sybil-frame`.
- **D8 — Aucune réhabilitation, aucune expiration.** ✅ *Une mise en sourdine était définitive.* Traité : les fautes
  s'estompent dans une fenêtre glissante.

## La confiance topologique

- **D9 — La position n'est pas vérifiée → attaque par éclipse.** ✅ *Un nœud pouvait mentir sur sa position pour
  s'insérer dans tous les voisinages.* Traité : diversité forcée du voisinage + corroboration + cap par sous-réseau.
- **D10 — Le point de rendez-vous reste une centralisation.** 🟡 *Il voit les adresses, c'est une panne unique.*
  Borné (rate-limit, preuve de travail à l'entrée). Décentralisation complète (fédération, découverte pair-à-pair) =
  chantier lointain ; honnêtement, l'infrastructure restera « pas de serveur central **autoritaire** », pas « zéro serveur ».

## L'autorité des objets partagés

- **D11 — La reprise d'autorité d'un objet est le point mou.** 🟡 *Un attaquant patient attend le silence du maître
  puis prend l'objet.* Directions : validation par **quorum de voisins**. Une approche **sans maître** (résolution
  par rang déterministe convergent) a déjà été éprouvée sur un cas et semble plus robuste à généraliser.
- **D12 — Tout est codé pour un seul objet.** 🟡 *Un vrai monde a des milliers d'objets partagés.* Direction : un
  **registre générique** `{id, type, règle d'autorité, état}` — deux patrons d'autorité existent déjà à fusionner.
- **D13 — Pas d'horloge commune → conflits mal arbitrés.** 🟡 *« Qui a touché en premier ? »* Un départage
  déterministe (version, identité) suffit aux jeux calmes ; seul le compétitif « à la milliseconde » exige un temps
  partagé — ce sera un service optionnel du socle.

## Identité & vie privée

- **D14 — L'identité n'était pas persistante.** ✅ *Pas de « compte » d'une session à l'autre.* Traité : la clé est
  sauvée localement (comme une clé SSH) et rechargée au lancement.
- **D15 — Tout circule en clair.** 🟠 *Positions (et bientôt la voix) sont lisibles ; la signature garantit
  l'authenticité, pas le secret.* Direction : chiffrement de transport par paire (X25519). **Volontairement différé**
  à la sortie publique : en R&D, le clair facilite la compréhension et le débogage du réseau.

## Robustesse & longévité

- **D16 — Fuites mémoire à long terme.** 🟡 *Les fiches par pair s'accumulent sans être nettoyées.* Traité en partie :
  éviction par durée de vie (un pair absent cède sa place, jamais un actif).
- **D17 — NAT symétrique → relais obligatoire.** 🟡 *Certaines connexions (mobile) ne se percent pas en direct.* Le
  relais via le point de rendez-vous est **prouvé en réel** (deux réseaux distincts, dans les deux sens). Reste
  ouvert : un relais **décentralisé** et son déclenchement automatique. C'est le **plancher** d'un onboarding fiable.
- **D18 — Le seuil anti-triche de vitesse est grossier.** 🟡 *Un tricheur subtil reste juste sous le seuil.* Direction :
  bornes plus fines, et surtout **réglables par le créateur** du monde (curseur « anti-triche agressif / souple »),
  au-dessus d'un socle de protections non négociables (signatures, anti-Sybil).

## Méta-doutes (sur la démarche)

- **D19 — Le coût réel par nœud n'avait jamais été mesuré.** ✅ Mesuré : **~34 Ko/s ↑ (max ~38), ~31 Ko/s ↓, ~0,7 %
  d'un cœur, ~38 Mo de RAM** par nœud → ~0,27 Mbit/s, **borné par le voisinage, pas par le total**.
- **D20 — Attaques combinées / adaptatives jamais testées ensemble.** 🟠 *Un vrai adversaire combine et s'adapte.*
  Objectif de long terme : un mode « scénario » coordonné.
- **D21 — La sécurité du point de rendez-vous lui-même.** 🟡 *Il peut être inondé.* Rate-limit + éviction en place ;
  pour le reste, on s'appuiera sur des outils éprouvés (anti-DDoS), en gardant chez nous les limites *applicatives*.
- **D22 — La foule dense.** ✅→🟡 *Le débit borné est prouvé (perception ~87 % à 1 000 nœuds, débit reçu plat).*
  **Rouvert** : le *ressenti* et la *pertinence* de la foule ne sont pas prouvés (voir D29/D30).
- **D23 — Le gossip est un amplificateur de DDoS.** ✅ Traité : preuve de travail exigée sur chaque « carte »,
  abandon du perçage non corroboré, rate-limit — prouvé par l'attaque `gossip-flood` (0 réflexion).
- **D24 — La foule visible était plafonnée à 64.** ✅ Traité : rendu « à deux tiers » (proches détaillés + foule
  d'imposteurs) — confirmé à l'écran, bien plus de 64 silhouettes sans chute de performance.
- **D25 — Le banc de simulation plafonne (~1 500 nœuds) ; « 55 000 » n'est jamais mesuré directement.** 🟡 *Le banc
  lance un fil d'exécution par nœud → il sature la machine, pas le protocole.* Cadrage : du point de vue d'un joueur,
  le monde est toujours **~32 voisins nets + 1 foule agrégée** — le total ne touche jamais une machine. Au-delà de
  ~2 000, c'est de l'**extrapolation**, jamais « prouvé ».
- **D26 — L'agrégateur d'une foule peut mentir** (cacher ou inventer des gens). 🟡 *La signature prouve qui parle,
  pas que le résumé est honnête.* Direction : **corroboration** (recouper plusieurs informateurs indépendants) ;
  l'estime sociale aide (un « fantôme » sans estime est incomptable).

## Périmètre — les doutes qui peuvent décider du projet

- **D27 — « La forteresse vide ».** 🔴 *A-t-on bâti une belle infrastructure dans laquelle deux humains ne se sont
  jamais vraiment retrouvés, en mouvement, via le vrai Internet ?* La seule grandeur qui décide « vivant vs mort »
  (la fraîcheur ressentie) ne se mesure pas en simulation. **Le test décisif est dehors, avec de vrais joueurs.**
- **D28 — La persistance d'état joueur sans serveur.** 🔴 *Une progression qui survit aux sessions, sans magasin
  central : où vit l'état, et qui empêche de le forger ?* Pour un premier essai, l'éphémère/local suffit ; la vraie
  réponse (« ta clé signe ton état, les pairs corroborent ») s'appuierait sur l'estime sociale.

## Perception, échelle & onboarding

- **D29 — L'aire d'intérêt par *proximité* ≠ par *pertinence sociale*.** 🟠 *Voir « les 32 plus proches » n'est pas
  voir « les 32 qui comptent » : si un voisin parle à quelqu'un hors de vue, cette personne reste invisible.*
  Direction : un ensemble d'intérêt par niveaux (voisins + partenaires d'interaction + relations), pas un simple
  rayon. C'est un problème d'**architecture**, pas de réglage — et le chantier d'ouverture de la phase d'échelle.
- **D30 — Le niveau de détail n'est pas adaptatif, et la foule n'a jamais été *rendue*.** 🟠 *Le système passe
  brutalement de « net » à « rien » au-delà du voisinage, quelle que soit la taille de la foule.* Direction :
  une fidélité **continue** (net → silhouette → champ de densité), adaptée à la machine et au lien de chacun.
- **D31 — La géométrie d'un monde ne doit jamais brider le réseau.** 🧭 *Devenu un principe :* puisque n'importe qui
  bâtira n'importe quel monde, **le réseau s'adapte aux créateurs, jamais l'inverse** ; il dégrade gracieusement,
  ne bloque ni n'exclut jamais.
- **D32 — « Le jeu est-il *fun* ? »** 🧭 *Hors périmètre :* le fun se conçoit et se bâtit (c'est le métier des
  créateurs) ; ce dépôt fait la R&D du **substrat**. La grandeur qui le concerne, c'est la **latence** (≤ 500 ms
  pour rester jouable).
- **D33 — Un seul cœur peut-il servir l'état *riche* d'un jeu ET l'état *mince* d'une foule de 55 000 ?** 🧭 *Point
  de veille :* deux régimes techniques différents cohabitent ; on surveille qu'ils ne « forkent » pas.
- **D34 — L'onboarding pourrait affamer chaque test.** 🟠 *Installer, traverser les box, se retrouver : un mur
  d'usage réel.* Direction : un lanceur qui apprend, au fil des connexions réelles, à franchir un maximum de
  configurations — avec le **relais (D17) comme plancher** garantissant « jamais zéro : à défaut de direct, relais ».
- **D35 — La voix de proximité, mur porteur du « est-ce vivant ? ».** 🟠 *Le ressenti social repose en partie sur
  elle, et c'est la pièce la plus différée.* Cadrage : « qui j'entends » = « qui est dans mon ensemble de
  pertinence » — la voix de proximité est l'aire d'intérêt (D29) appliquée à l'audio, à bâtir par-dessus.

---

*Ce registre est vivant : on y ajoute un doute dès qu'on en découvre un, et la réalité — une mesure, un test, un
vrai joueur — a toujours raison contre ce document.*
