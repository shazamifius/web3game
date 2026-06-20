# 🤖 PLAN D'ATTAQUE AUTONOME — travail fait pendant l'absence de l'utilisateur

> **À quoi sert ce fichier.** L'utilisateur s'absente longtemps. Ce document est (a) le **contrat**
> de ce que j'ai le droit de faire seul, (b) mon **ancre anti-collapse** relue à CHAQUE itération
> (avec l'ancre §0 de `FEUILLE_DE_ROUTE.md`), (c) le **journal** de ce qui a réellement été fait.
> L'utilisateur le lit/corrige à son retour. Décidé avec lui le 20 juin 2026.

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
- **T0.3 — Extension + chasse au mur.** Seulement si T0.2 vert : rejouer perception ∝ N et débit à
  **1k / 5k / 20k / 50k**. Rapporter : l'invariant tient-il ? où ça plie ? quel coût/nœud ?
  - *Preuve* : un tableau N → {perception moy/max, Ko/s ↑↓, %CPU, RAM}. **Honnêteté** : dire si c'est
    le PROTOCOLE qui plie ou le BANC (et lequel). Ne jamais écrire « 55K prouvé » — écrire ce qui EST mesuré.

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
- **T1.3 — D18 : speed-hack grossier (ch.11.4).** Raffiner la détection de vitesse au-delà du
  téléport déjà couvert (9.x) : une vitesse soutenue au-dessus du plausible → faute/sourdine.
  - *Preuve* : `attack` — un bot « rapide mais sous le seuil téléport » finit en sourdine ; la marche
    normale passe. Additif (réputation existante). *NE fait pas* : pas d'autorité physique complète (ch.11).

> Ordre conseillé : **T0 d'abord** (plus gros gain, risque nul), puis T1.1 → T1.2 → T1.3.

---

## 📓 JOURNAL (rempli au fil des itérations autonomes — le plus récent en HAUT)
*(vide pour l'instant — se remplit dès le premier réveil autonome)*

- _(rien encore)_

---

## 📥 FILE « UTILISATEUR » (ce que j'ai rencontré mais qui T'appartient — à ton retour)
*(les murs durs Tier 2 + tout ce que j'ai dû stopper)*

- **D26 couche 2 (corroboration)** et **D4 « parent par mesure du réel »** : murs de DESIGN, à
  défricher avec toi (risque rustine si fait en aveugle). *(Je peux les pré-défricher sur PAPIER si tu
  passes le périmètre à « + Tier 2 sur papier ».)*
- _(ce que j'ajouterai si je dois stopper un pas)_
