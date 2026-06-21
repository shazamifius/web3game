# Bonus à rajouter (plus tard)

> Trois idées qu'on GARDE pour après le premier petit jeu jouable.
> Écrit en mots simples + ma note d'ingénieur honnête (facile / dur, et pourquoi).
> On ne les code PAS maintenant — on fait d'abord : île + météorites + ramassage + balle-gravité + saut.

---

## Bonus 1 — Compteur de météorites SAUVEGARDÉ (« impossible de mentir »)

**L'idée simple :** chaque joueur a un compteur de météorites ramassées. Il est sauvé sur sa
machine, et quand il se reconnecte, il retrouve son score. Lié à SON identité (sa clé).

**Ma note honnête (deux choses différentes !) :**
- **« Impossible de mentir sur QUI possède le score »** → FACILE et déjà à portée. Le score est
  signé par ta clé (le système d'identité persistante existe déjà : `~/.web3game/<profil>.key`).
  Personne ne peut réclamer le score d'un autre.
- **« Impossible de mentir sur le score LUI-MÊME »** → DUR sans serveur. Tout seul chez toi, rien
  ne t'empêche d'écrire « 9999 » dans ton fichier. Pour que le compteur soit VRAIMENT incheatable,
  il faut que d'autres joueurs aient VU que tu as ramassé (des témoins) → c'est le Bonus 2.
- **Conclusion :** persistance + signature = simple et réel tout de suite ; compteur incheatable =
  a besoin du Bonus 2 (horloge/témoins). On peut faire la version simple d'abord, l'incheatable après.

---

## Bonus 2 — Système d'horloge : QUI a ramassé, et pas de doublon

**L'idée simple :** quand deux joueurs arrivent sur la même météorite en même temps, savoir qui l'a
eue pour de vrai, et ne JAMAIS la compter deux fois.

**Ma note honnête :**
- C'est **le même problème dur** que la balle partagée (qui en est le « maître », qui décide). Et ce
  problème, on l'a DÉJÀ résolu en partie pour la balle (autorité + passage de maître). On réutilise
  ce schéma : une « horloge logique » (qui a réclamé en premier) + un seul décideur par météorite.
- Difficulté : **moyenne** (on s'appuie sur du code existant).
- C'est CE bonus qui rend le compteur du Bonus 1 digne de confiance.

---

## Bonus 3 — Chat vocal

**L'idée simple :** se parler à la voix avec les joueurs autour de soi.

**Ma note honnête :**
- C'est **la grosse pièce** : capter le micro → compresser → envoyer en continu en temps réel →
  rejouer en mélangeant plusieurs voix. Ce n'est PAS un après-midi, c'est un chantier dédié.
- Mais ça colle **parfaitement** à l'archi : **voix spatiale** = les gens proches sonnent plus fort,
  exactement notre système des ~32 voisins. C'est le truc qui rend une foule VIVANTE.
- Donc : haute valeur, à faire **bien**, comme un bloc à part (pas bâclé).

---

*(Note : ces trois bonus se tiennent. Bonus 2 débloque le vrai Bonus 1. Bonus 3 est indépendant et
peut venir quand on veut, mais c'est le plus lourd.)*
