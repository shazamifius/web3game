# Sécurité — ce qu'on a cassé chez nous, et comment

*[English version](en/SECURITE.md)*

Ce document raconte un audit de sécurité mené sur ce projet le **22 juillet 2026**, les défauts qu'il
a trouvés, et la méthode employée. Il est publié parce qu'un dépôt qui montre ce qu'il a corrigé est
plus utile — et plus honnête — qu'un dépôt qui affirme que tout va bien.

Le projet est un réseau pair-à-pair : chaque participant fait tourner un nœud qui parle directement
aux autres. Il n'y a pas de serveur central pour arbitrer, donc **chaque nœud doit se défendre
seul**, contre des paquets écrits par n'importe qui.

## Résultat en une ligne

**16 défauts trouvés, 16 corrigés**, chacun avec un test qui échoue si la correction disparaît.

| Gravité | Défaut | Ce qui était possible |
|---|---|---|
| 🔴 | Journal de présence servi publiquement | Lire l'adresse IP, les horaires de connexion et le nom de machine de chaque participant |
| 🔴 | `PUNCH` non authentifié | Faire réfléchir le flux d'un nœud vers une victime arbitraire — 34 octets envoyés, 728 émis, indéfiniment |
| 🟠 | Rejeu d'une redirection signée | Renvoyer toute la flotte vers un serveur mort ou réattribué |
| 🟠 | Aucun lien version ↔ contenu | Réinstaller un ancien exécutable vulnérable, **définitivement** |
| 🟠 | `HELLO` non signé | S'inscrire sous l'identité d'autrui : son trafic arrive chez vous, elle devient injoignable |
| 🟠🟡 | 5 dénis de service | Épuiser la mémoire ou remplir le disque du serveur |
| 🟡 | 2 défauts sur l'objet partagé | Le voler à distance ; le verrouiller à vie |
| 🟡 | Vol d'objet trouvé par le banc d'attaque | Prendre un objet libre sans jamais l'approcher, puis le rendre inatteignable |

Deux autres défauts n'étaient visibles qu'en interrogeant le **serveur réel**, pas le code : une
sonde de vivacité qui visait une URL inexistante (le client croyait donc en permanence que tous les
serveurs étaient morts), et une vérification cryptographique placée avant son propre garde-fou de
débit — une correction de sécurité qui en ouvrait une autre.

## Comment ils ont été trouvés

Aucune méthode n'a suffi seule. C'est le point le plus important de ce document.

### 1. Audit par agents avec vérification adverse

82 agents lancés en parallèle sur les zones sensibles, en deux temps : chercher, puis **réfuter**.
Chaque trouvaille est soumise à des sceptiques indépendants dont la consigne est de la démolir.

25 trouvailles brutes → **14 confirmées, 8 réfutées**. Un tiers des alertes étaient fausses. Sans
l'étape de réfutation, on aurait « corrigé » du code qui n'était pas cassé.

### 2. Tests de propriété

Un test d'exemple dit *« pour cette entrée, voilà la sortie »*. Il ne dit rien du cas auquel personne
n'a pensé — or un défaut de sécurité est exactement ça.

Quatre propriétés, énoncées comme des règles universelles :

- aucun message signé n'est accepté après altération d'**un seul bit** (exhaustif : chaque octet,
  chaque bit) ;
- aucune version acceptée ne redescend jamais, sur **aucun** canal ;
- on ne renvoie jamais vers une adresse non validée plus d'octets qu'elle n'en a envoyé ;
- aucune table ne grandit sans borne sous flux hostile.

La première a immédiatement signalé un octet non couvert par une signature. Vérification faite,
c'était délibéré — mais l'exception est désormais **déclarée et justifiée dans le code**, et le test
exige qu'elle reste réelle.

### 3. Test de mutation — la discipline qui compte le plus

Pour chaque correction : remettre le code vulnérable, **vérifier que le test échoue**, restaurer.

C'est ce qui distingue un test utile d'un test décoratif. Sur l'objet partagé, la démonstration est
sans appel : en remettant les deux comportements d'origine, les 2 nouveaux tests tombent et **aucun
des 8 tests existants ne bouge**. Ces 8 tests passaient depuis toujours, à côté de la faille.

Chaque test d'attaque porte aussi une **garde anti-test-creux** : il exige d'abord que le chemin
normal fonctionne. Sans elle, « aucun octet n'est parti chez la victime » serait vrai simplement
parce que rien n'est parti du tout.

### 4. Confrontation à des outils extérieurs

| Outil | Ce qu'il cherche | Résultat |
|---|---|---|
| **ThreadSanitizer** | courses entre threads | 0 sur 351 tests instrumentés |
| **Kani** (AWS) | preuve formelle d'absence de panique | **1 défaut trouvé** ; 6 preuves sur 8 (2 sur du texte trop coûteuses, couvertes autrement) |
| **Wycheproof** (Google) | pièges connus de la vérification de signature | 150 cas, verdict identique à la référence — 62 pièges tous refusés |
| **CodeQL** (GitHub) | analyse statique sémantique (SAST) du Rust | 63 fichiers sur 63 analysés, 0 alerte *(dépôt alors public ; suspendu depuis — voir plus bas)* |
| **Fuzzing déterministe** | paquets hostiles | 240 000 000 décodages à la main, 0 panique — désormais rejoué en continu (campagne nocturne à graine variable) |
| **Clippy** | qualité | 0 remarque |

**Kani** a fait mieux que confirmer : il a **trouvé un défaut que 240 millions d'essais de fuzzing
avaient manqué**. Une fonction anti-amplification ramenait une taille démesurée au maximum
représentable au lieu de la refuser — l'invariant « jamais plus d'octets émis que reçus » était faux
dans un cas extrême. Une preuve essaie *toutes* les valeurs à la fois ; un test, seulement celles
qu'on a imaginées. Corrigé, puis **prouvé**. Deux preuves portant sur de l'analyse de texte restent
hors de portée pratique de Kani (l'exploration symbolique des chaînes explose — limite connue du
procédé, pas un défaut du code) : elles sont couvertes par les tests de propriété et le fuzzing, et
le code le dit sans arrondir.

**Wycheproof** est la batterie de tests de l'équipe sécurité de Google, écrite pour *casser* les
implémentations de cryptographie. Notre vérification de signature donne le même verdict qu'elle sur
les 150 cas, dont les 62 pièges (signatures malléables, clés dégénérées, encodages non canoniques).
Vérifié par mutation : une vérification qui accepterait tout laisse passer 50 de ces pièges.

**CodeQL** (l'analyseur statique de GitHub) construit un modèle sémantique du code — pas une simple
recherche de motifs — et y cherche des schémas de failles connus. Il comprend le Rust nativement
depuis fin 2025. Passé sur le cœur réseau, il a analysé **63 fichiers Rust sur 63** et n'a levé
**aucune alerte**. Ce « zéro » ne compte que parce que les journaux du run prouvent que l'extraction
Rust a réellement eu lieu (base de données construite, requêtes exécutées) — la même exigence que
plus loin : *un « 0 problème » doit prouver que quelque chose a tourné.*

> **Mise à jour (juillet 2026) — CodeQL est suspendu, honnêtement.** L'analyse était rejouée à chaque
> poussée tant que le dépôt était **public**. Le dépôt du code est depuis passé en **privé**, et GitHub
> n'offre CodeQL gratuitement que sur les dépôts publics (sur un dépôt privé il faut *GitHub Advanced
> Security*, payant) : l'analyse n'est donc **plus rafraîchie**. Le résultat ci-dessus reste vrai pour la
> version analysée — il n'est simplement plus reconduit à chaque commit. Le **fuzzing déterministe**, lui,
> continue de tourner en intégration continue.

### 5. Le banc d'attaque — ce qui a trouvé ce que rien d'autre n'a vu

Le projet embarque son propre programme attaquant : 11 attaques réelles, vraies sockets, vrais
paquets forgés, contre de vrais nœuds.

**Une attaque a réussi.** L'objet partagé a été volé chez les deux victimes. Au même moment :
les 11 tests unitaires passaient, les 4 propriétés passaient, ThreadSanitizer ne voyait rien, et
240 millions de paquets de fuzzing non plus.

Seule l'exécution réelle l'a révélé.

## Ce que la preuve formelle a apporté que 240 millions d'essais n'ont pas apporté

Le fuzzer avait soumis 240 millions de paquets à la fonction de crédit anti-amplification sans rien
trouver. [Kani](https://model-checking.github.io/kani) l'a cassée en quelques secondes.

La fonction ramenait une taille trop grande au maximum représentable, au lieu de la refuser. Avec un
crédit maximal, un envoi de plusieurs milliards d'octets passait donc en n'étant facturé que quatre
milliards — l'invariant « jamais plus d'octets émis que reçus » était **faux**.

Non exploitable en l'état (un datagramme plafonne à 64 Ko), mais la garde aurait sauté en silence le
jour où cette fonction servirait ailleurs. Les tests ne l'avaient pas vu parce qu'ils n'essayaient
que des tailles réalistes. **Une preuve, elle, essaie tout.**

La règle générale en sort renforcée : *sur une quantité à payer, saturer vers le haut sous-facture
toujours ; le sens sûr est de refuser.*

## Les leçons, qui valent plus que les correctifs

**Une garde posée sur UN chemin n'est pas une garde.** Quatre défauts sur seize sont des variantes
de ça : la protection existait sur un canal et manquait sur un autre. Un point de confiance unique
n'a de valeur que si chaque barrière y est complète.

**Signer prouve QUI, jamais QUAND.** Toute donnée signée reste rejouable à vie. Il faut une version
à l'intérieur du contenu signé, et un plancher qui **survit à la suppression du fichier** —
supprimer un fichier est à la portée de l'attaque.

**Budgets d'abord, cryptographie ensuite.** Vérifier une signature coûte cher. Placer cette
vérification avant le garde-fou de débit transforme une protection en déni de service.

**Mesurer avant de fixer un seuil.** Un détecteur de à-coups réglé à l'intuition (150 ms) ne
détectait rien, même sous 48 processus concurrents. Mesure faite — 0 ms au repos, 189 ms en charge —
le seuil est passé à 50 ms. Un instrument mal calibré est pire que pas d'instrument : il rassure.

**Un « 0 problème » doit prouver que quelque chose a tourné.** ThreadSanitizer a d'abord annoncé
« aucune course détectée »… alors que rien n'avait compilé. La preuve que l'analyse a bien eu lieu
fait partie du résultat.

**Un message écrit en dur n'est pas une mesure.** Le programme attaquant affichait « l'objet n'est
PAS volé » au moment précis où il venait de le voler. Le verdict se lit côté victime.

**Ce ne sont pas des portes dérobées.** Une porte dérobée est délibérée ; ce sont des erreurs. La
distinction n'est pas un détail de vocabulaire.

## Choix assumés, et leurs coûts

La sécurité se paie. Ce qui est payé ici, écrit noir sur blanc :

- **Une identité ne se déplace jamais tant qu'elle est vivante.** Ferme l'usurpation ; coûte une
  reconnexion plus lente à qui change réellement d'adresse en cours de session.
- **L'adresse d'un pair ne s'apprend jamais du trafic reçu**, seulement d'une source corroborée. Un
  état signé prouve qu'une clé l'a émis un jour, jamais qu'elle se trouve à cette adresse maintenant.
- **La liste des fichiers servis publiquement est blanche, pas noire.** Une liste noire oublie
  toujours quelque chose ; une liste blanche oublie du côté sûr. C'est ce qui a fermé la fuite du
  journal de présence.
- **Zéro dépendance, sauf la cryptographie.** On n'écrit jamais sa propre cryptographie ; tout le
  reste du protocole est fait main et lisible.

## Limites — ce que ce document ne prétend pas

- Le journal de présence a été exposé pendant une durée inconnue. Il fallait connaître l'URL, ce qui
  rend une consultation peu probable. « Peu probable » n'est pas « personne ».
- ThreadSanitizer ne voit que les chemins que les tests exécutent : « aucune course » signifie
  « aucune course là où les tests passent ».
- Le fuzzing n'est pas guidé par la couverture : 240 millions de paquets ne valent pas une preuve.
- Les preuves formelles portent sur l'absence de panique des décodeurs, pas sur la correction du
  protocole entier.
- **Seize défauts trouvés ne veut pas dire zéro restant.** Ce document dit ce qui a été cherché et
  comment, pas que la recherche est terminée.

## Signaler un problème

Ouvrez une *issue* sur ce dépôt. Si le sujet est sensible, indiquez-le sans détailler la faille dans
le fil public, et un canal privé sera proposé.
