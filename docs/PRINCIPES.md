# 🧱 PRINCIPES, ÉTAT DES LIEUX & DÉCISIONS

> La boussole (principes directeurs), ce qui est FAIT, l'ordre conseillé, les décisions prises, et les options reportées.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

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

---

