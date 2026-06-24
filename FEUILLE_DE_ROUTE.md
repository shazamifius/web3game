# Feuille de route — web3game (INDEX)

> **Ce fichier est désormais un INDEX.** Le plan complet vivait dans un seul énorme document ;
> il a été **éclaté le 25 juin 2026** en plusieurs docs focalisés (≤ ~450 lignes chacun, une
> fonction par doc) pour mieux s'y retrouver et éviter la dérive. **Rien n'a été perdu** : tout
> le contenu est dans `docs/` ci-dessous. On feuillette le doc utile et on le remplit au besoin.
>
> **Règle d'or rappelée.** On ne vise pas l'inviolable absolu. On vise : *chaque attaque devient
> soit impossible, soit chère, soit attribuable et bannie* — et *chaque joueur, du 0 connexion au
> 2 Gb/s, a la meilleure expérience possible pour SON lien*. Méthode : **compile → test → preuve →
> commit → push**, petits pas, la base réseau qui marche est INTOUCHABLE (Règle 1).

## 🗺️ La carte des documents

| Doc | Fonction | Quand le lire / remplir |
|-----|----------|--------------------------|
| **[docs/ETAT.md](docs/ETAT.md)** | ⭐ **L'ANCRE** : le pourquoi, les 5 règles, le plan d'attaque, l'**état courant + la prochaine action**, le registre de dettes. | **À LIRE EN PREMIER** chaque session ; à mettre à jour à chaque fin de session. |
| [docs/PRINCIPES.md](docs/PRINCIPES.md) | La boussole (principes directeurs), ce qui est FAIT, l'ordre conseillé, les décisions prises, les options reportées. | Quand on doute d'une direction ou d'une priorité. |
| [docs/DOUTES.md](docs/DOUTES.md) | 🤔 L'inventaire des **doutes D1→D27** (le cœur) et par quel chapitre chacun se ferme. | Quand on attaque/ferme un risque ; ajouter un Dn quand on en découvre un. |
| [docs/CHANTIER-RESEAU.md](docs/CHANTIER-RESEAU.md) | Programme des chapitres **réseau** : réel/NAT (7), confiance (9), identité & vie privée (10). | Quand on travaille le netcode/sécurité réseau. |
| [docs/CHANTIER-FOULE.md](docs/CHANTIER-FOULE.md) | Chapitre **8** (foule dense & inclusivité, D22) — focus/conscience, gossip, cellules, corroboration. | Quand on travaille la perception à l'échelle. |
| [docs/CHANTIER-ROBUSTESSE.md](docs/CHANTIER-ROBUSTESSE.md) | Chapitres **11/12/14** : autorité généralisée, robustesse/longévité, portabilité moteurs. | Quand on travaille l'autorité d'objets, la longévité, ou le portage. |
| [docs/PRESENCE-VIVANTE.md](docs/PRESENCE-VIVANTE.md) | ✨ Chapitre **13** (D27, l'âme) : pourquoi un avatar doit SEMBLER vivant ; l'option A déjà codée. | Quand on travaille le ressenti/rendu des avatars. |
| [docs/VISION.md](docs/VISION.md) | 🎮 La **vision plateforme** (launcher façon gamejolt) + le **1er jeu** (île aux étoiles) + les jalons. | Pour la boussole produit ; à enrichir quand le jeu avance. |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | 🏗️ L'organisation du code (`src/`), l'en-tête de paquet, l'archi cible « Own + Shields ». | Pour comprendre le code en profondeur. |
| [docs/TESTS.md](docs/TESTS.md) | 🧪 Comment **lancer & tester** (cœur headless, `tc netem`, NAT en namespaces). | Pour reproduire une mesure / monter un banc. |
| [docs/JOURNAL.md](docs/JOURNAL.md) | 📒 Le **journal de bord** daté (ce qui a été bâti, chapitres 0→6 + le « où on en est »). | À relire au besoin (via `grep`) ; on n'y rejoue pas tout l'historique. |
| [docs/CONTRAT_SIDECAR.md](docs/CONTRAT_SIDECAR.md) | 🔌 Le contrat de l'interface cœur-Rust ↔ Unreal (sidecar : transport, messages, paliers). | Quand on touche au pont Unreal. |

> Le **[README.md](README.md)** est, lui, une **intro simple** du projet (sans détails de test) — pour les nouveaux.
> Les notes PRIVÉES (infra, sessions, Unreal « où trouver quoi » = `prive/COMPREHENSION_UNREAL.md`) sont gitignorées : dossier `prive/*`.
