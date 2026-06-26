# Chantier réseau — confronter le réel, durcir la confiance, posséder son identité

> Le détail technique de trois fronts du cœur réseau : (1) le confronter à de vraies conditions Internet
> (latence, perte, NAT) ; (2) durcir la confiance contre la triche coordonnée ; (3) une identité persistante et
> privée. On y montre aussi, **sans les cacher, les hypothèses que la mesure a réfutées** — c'est là que se gagne
> la rigueur.
>
> Voir : [les doutes traités](doutes.md) · [l'état chiffré](etat-du-projet.md) · [l'idée en clair](comprendre-le-p2p.md).

---

## 1. Confronter le réseau au réel (latence, perte, NAT)

**Le problème (D1).** Une simulation sur une seule machine — sans latence, sans perte, sans NAT — peut être
parfaite et pourtant s'écrouler sur Internet.

**L'approche.** Injecter de vraies conditions réseau avec `tc netem` sur l'interface loopback (latence, gigue,
perte, ré-ordonnancement), selon trois profils (`bon` ~30 ms · `moyen` ~120 ms + 2 % · `mauvais` ~250 ms + 5 % +
ré-ordonnancement), puis faire tourner la simulation derrière. *(Détail subtil tranché en route : sur le loopback,
le délai compte double — aller-retour sur la même interface — donc le script applique la moitié du ping visé.)*

**Le résultat — et une hypothèse réfutée par la mesure.** Premier constat : le débit honnête chutait de **−70 %**
sous le mauvais profil. Hypothèse de départ : l'anti-rejeu strict (qui rejette les paquets ré-ordonnés). On l'a
donc corrigé — passage à un anti-rejeu **à fenêtre glissante** (comme IPsec / WireGuard), utile et nécessaire car
les vrais réseaux ré-ordonnent. **Mais la mesure a tranché : ça ne récupérait que +15 %, pas les 70 %.** La vraie
cause était un **artefact du banc de test** : `tc netem` plafonne sa file d'attente à 1000 paquets par défaut, ce
qui impose un plafond de débit ≈ file ÷ délai. File élargie → le débit honnête **remonte à l'optimal** (≈ −9 % sous
le pire profil, soit essentiellement la seule perte de 5 %).

→ **Conclusion :** le protocole **tient sous réseau réel** (250 ms + gigue + 5 % de perte + ré-ordonnancement) ; la
chute n'était pas dans le protocole, mais dans l'instrument. *La leçon vaut le détour : sans la discipline « une
mesure contradictoire prime sur l'hypothèse », on aurait poli le mauvais endroit.*

**La sécurité tient aussi sous mauvais réseau** : à 250 ms + ré-ordonnancement, orbe **0 volée**, téléport / triche
d'incrément / Sybil neutralisés.

**NAT réel.** Le hole-punching est prouvé entre vraies « box » (montées en namespaces réseau) : **maillage complet**
pour les NAT « cône », échec attendu pour le NAT symétrique → repli sur relais (cf. D17).

## 2. Durcir la confiance (Sybil, éclipse, accusations)

**Le problème.** Sans autorité centrale : comment empêcher un attaquant de fabriquer de fausses identités (Sybil),
d'isoler une victime (éclipse), ou de faire bannir un innocent (*framing*) ?

**Des identités coûteuses — trois couches, pas trois rivales (D6).** Rendre une identité « chère » oppose deux
objectifs : plus c'est cher (en calcul), plus on punit le joueur **faible** (un téléphone qui devrait « miner »
longtemps avant de jouer). Le bon design est *cher pour l'attaquant en masse, léger pour l'honnête isolé*. Trois
couches complémentaires : **(a)** un socle minimal ; **(b)** une difficulté qui **monte localement sous attaque** et
redescend au calme ; **(c)** plus tard, un **parrainage social** (le coût devient une relation, pas du calcul —
l'ami des faibles). *Ce ne sont pas des options à choisir, mais des couches à poser dans le bon ordre.*

**Le « témoin crédible » (anti-framing, D7).** Compter des accusateurs distincts est naïf : trois fausses identités
suffiraient. La réponse : **sommer un poids de crédibilité**, pas des têtes. Un accusateur ne pèse que s'il a
**déjà participé** au monde (envoyé de vrais états signés — un Sybil qui ne fait que cracher des accusations pèse
**zéro**) et s'il était un **témoin plausible** (assez proche pour avoir pu voir la triche). Prouvé par l'attaque
`sybil-frame`, qui bascule alors de « framing réussi » à « framing échoué ».

**Anti-éclipse — et une deuxième hypothèse réfutée (D9).** Plan initial : diversifier le voisinage « façon Kademlia »
(par distance d'identités). **Mauvais outil dans notre modèle** : une identité étant une clé ~aléatoire, les Sybils
se répartissent exactement comme les honnêtes → ça ne distingue rien. Le bon levier (celui de Bitcoin / Ethereum)
est la **diversité d'adresses IP** : un attaquant fabrique des identités gratuitement, mais n'a qu'une poignée
d'adresses. La contribution d'une accusation est donc **plafonnée par sous-réseau** → mille faux comptes derrière
une seule IP = **une seule voix**.

**Positions corroborées (D9).** Les positions « rapportées par un tiers » (gossip) sont falsifiables : elles ne
servent qu'à la découverte, **jamais** à juger une accusation — pour cela, on n'utilise que les positions **signées**
par le pair lui-même.

**Réhabilitation (D8).** Une faute n'est plus une condamnation à vie : les fautes **s'estompent** dans une fenêtre
glissante.

> *Limite honnête :* la corroboration par sous-réseau n'est prouvée qu'en logique et en simulation ; le test sur de
> **vraies adresses IP diverses** (et le résidu « botnet ») reste à faire — c'est une limite de fond du pair-à-pair.

## 3. Identité persistante & vie privée

**Identité persistante — fait (D14).** La clé est minée **une fois**, sauvée localement (comme une clé SSH, en
accès restreint) et **rechargée** au lancement → un vrai « compte » d'une session à l'autre, sans serveur de comptes.

**Vie privée — à venir (D15).** Aujourd'hui les positions circulent **en clair** (la signature garantit
l'authenticité, pas le secret). Le chiffrement de transport par paire (échange de clés X25519) est planifié, mais
**volontairement différé** : en R&D, le clair facilite la compréhension et le débogage du réseau.

---

*Faits techniques tenus à jour au fil des mesures. Les chiffres de coût et de débit : [revue de l'état](etat-du-projet.md).*
