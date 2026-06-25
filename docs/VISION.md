# 🎮 VISION PLATEFORME + LE PREMIER JEU

> Le launcher façon gamejolt (multi-moteur), le hub, et le 1er jeu « île aux étoiles » + l'ordre des jalons.
> *(Doc éclaté depuis l'ancienne `FEUILLE_DE_ROUTE.md` le 25 juin 2026 — voir l'index `FEUILLE_DE_ROUTE.md` pour la carte complète.)*

---

## 🧭 LE CAP — une boussole VIVANTE (⚠ PAS une checklist)

> **⚠⚠ LIRE D'ABORD — ce que cette liste EST et N'EST PAS.** Ce n'est **pas** une liste de cases à cocher.
> C'est une **boussole** : un cap flou, volontairement imparfait, qui **doit évoluer, changer, se métamorphoser**
> au fil des preuves. **Règle d'or de l'utilisateur (25 juin 2026) :** *on ne coche JAMAIS un truc en disant « ça
> passe à peu près ».* On fait les choses **bien**, on prend le **temps** ; **si on doit faire un détour de 400 ans,
> on le fait.** Un item qui « tient à peu près » n'est pas fait — il est en dette (cf. [docs/DOUTES.md](DOUTES.md)).
> Chaque ⚠ ci-dessous est un **déclencheur de remise en cause** : si la condition arrive, on REPENSE, on ne rustine pas.

- **★ La seule chose qui compte : prouver que c'est VIVANT** (D27). Tout le reste sert ça. ⚠ Si un test avec de vrais
  potes montre que ce n'est pas *vivant/fun*, on **repense le jeu** — on ne rajoute pas des features par-dessus un cœur mort.
- **Le LIEU avant la boucle ; le RÉEL avant le joli.** On bâtit l'espace partagé d'abord, en vrai sur Unreal (pas de
  faux murs de proto). ⚠ Si « se retrouver au même endroit » coince, c'est prioritaire sur tout gameplay.
- **La persistance (D28) est une QUESTION OUVERTE, pas un acquis.** ⚠ Elle peut nous forcer à accepter un petit serveur
  de sauvegarde, OU un modèle « ta clé signe ton état », OU l'éphémère assumé. À trancher *par la preuve*, plus tard.
- **Le vocal se DÉ-RISQUE avant de se bâtir.** Tester la prémisse sociale avec un vocal externe (Discord). ⚠ Si « être
  ensemble » est fun sans vocal P2P, on peut le repousser longtemps ; si le fun en dépend, on saura que ça vaut le coût.
- **L'onboarding/distribution deviendra un mur** dès qu'on dépasse 2-3 potes (installer, sidecar, NAT, rendez-vous=SPOF).
  ⚠ Ne pas le sous-estimer : « partageable vite à 10 potes » est un vrai chantier, pas un détail.
- **Le cœur réseau reste INTOUCHABLE** ; tout ajout est **additif et prouvé** (Règle 1). ⚠ Si un pas menace le cœur, on ne le fait pas.
- **La PLATEFORME (gamejolt multi-moteur) reste une ÉTOILE LOINTAINE.** ⚠ On protège le focus : on n'en code pas une
  ligne tant que le 1er jeu n'a pas prouvé que « c'est vivant ». La sirène du scope est le danger n°1.

> *Cette liste se relit et se réécrit à chaque fois qu'une preuve déplace un mur. Si elle a l'air figée, c'est qu'on a oublié de la faire vivre.*

## VISION PLATEFORME + LE PREMIER JEU (posé avec l'utilisateur, 25 juin 2026)

> Le « pourquoi » au-dessus du netcode. À garder comme boussole du travail Unreal ; on n'en code qu'un
> petit bout à la fois (preuve d'abord), mais c'est CE but qui dit dans quel ordre attaquer.

### La plateforme (vision long terme — PAS un clone)
Ce n'est **ni Roblox, ni VRChat, ni un Fortnite Creator**. La référence est **gamejolt** : un **launcher officiel**
(sur Unreal pour l'instant) qui, en réalité, **embarque plusieurs moteurs** (Unity, Unreal, Godot — **une seule
version de chaque** au début ; plus tard p.ex. UE 5.8 *et* UE 6). On arrive dans un **hub 3D** où l'on **retrouve ses
amis** et où l'on **navigue parmi les jeux proposés** (au début, uniquement les jeux de l'auteur ; à terme **n'importe
qui crée le sien**). Entrer dans un jeu = le launcher **télécharge le monde** (depuis des serveurs d'abord, puis un
**système façon BitTorrent**) en indiquant **quel moteur + quelle version**, puis nous fait entrer. Le **cœur réseau
P2P (ce dépôt) est le liant commun** à tous ces mondes, quel que soit le moteur.

### Le 1er jeu — « l'île aux étoiles » (le strict minimum d'abord)
- **Cadre** : une grande île vivante (faune, flore, biomes — arbres, fleurs). On est **humain** au départ.
- **Boucle** : des **étoiles tombent** (sur terre ou dans l'eau) → on les **ramasse** → elles donnent des **cristaux**.
- **Usage 1 des cristaux — le PUITS MAGIQUE** : échanger ses cristaux pour **changer de race animale** *au hasard*,
  parmi tout l'existant. Principe : **aucune race n'est « meilleure »** en déplacement — elles sont **complémentaires**.
- **Usage 2 — AMÉLIORER son animal actuel** : vitesse, énergie, faim, agilité. (Il y a un **système de nourriture +
  de stamina**.)
- **L'intention sociale (le cœur du design)** : **l'évolution est TRÈS LENTE**, volontairement. Le but n'est pas le
  grind : c'est de **forcer l'usage du chat vocal de proximité + du chat texte**, de créer **interactions,
  rapprochements, amitiés**. C'est ça qui répond pour de vrai à **D27 (« est-ce vivant ? »)**.

### L'ordre d'attaque décidé (le LIEU avant la BOUCLE)
On **ne commence pas par la boucle de jeu** (étoiles/cristaux/puits) mais par **l'espace social**, car tout en dépend
et ça réutilise 100 % du cœur déjà prouvé.
- **Jalon 1 ✅ (25 juin)** — **vrais avatars** (au lieu des capsules debug) qui se voient/bougent sur une **même map
  Unreal** (repère partagé ~gratuit puisque deux UE = même niveau). Prouvé en `-game` avec un bot. *(spike01-unreal `a97122d`.)*
- **Jalon 2** — l'**île + le hub** avec les assets (utilisateur = niveau/visuel ; moi = code) ; vrais avatars sur un vrai sol.
- **Jalon 3** — **étoiles déterministes** (calculées d'une **graine partagée** + temps → personne ne réseau-te chaque
  étoile) ; le **ramassage = un événement d'AUTORITÉ** (réutilise la logique **ORBE+OWN** gardée propre : `supersedes`
  /`apply_incoming`/wire signé, palier 4 du contrat sidecar) → l'étoile disparaît pour tous, le ramasseur gagne un cristal.
- **Jalon 4** — puits magique (changement de race = mesh), stats (vitesse/énergie/faim/stamina, surtout local).
- **Jalon 5** — **chat texte** de proximité, **puis chat VOCAL** (chapitre 13 — gros morceau, **assumé différé** : on ne
  bloque pas des semaines sur l'audio avant d'avoir vu un pote sur l'île).
