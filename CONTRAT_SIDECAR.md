# CONTRAT SIDECAR — l'interface cœur-Rust ↔ Unreal (palier 0)

> **Ce que c'est.** Le **keystone** de la bascule Unreal : le cœur réseau Rust (déjà prouvé : relais NAT,
> gossip, foule dense) tourne en **process séparé** (« sidecar ») ; Unreal est un **client mince** qui lui
> parle par une **socket locale**. Ce fichier FIGE le contrat AVANT de coder (méthode : chercher le mur sur
> le papier). Règle 1 intacte : le cœur Rust reste l'autorité, on n'y touche pas — on lui ajoute une SORTIE.
>
> **Statut : PALIER 0 (papier).** Rien n'est codé. Les décisions ci-dessous sont tranchées (pas à arbitrer) ;
> elles sont réversibles tant qu'on n'a pas mesuré le palier 1.

---

## 0. Pourquoi un sidecar (et pas un binding FFI)

Le cœur fait DÉJÀ du réseau (boucle `Bot::step`, [src/net/bot.rs](src/net/bot.rs)) : le faire tourner en
process séparé ne réécrit quasi rien. Unreal pousse sa position et lit les avatars distants par une socket —
zéro couplage de boucle. Bonus mesuré gratuitement : le **gel d'orbe quand la fenêtre est occultée**
(sim couplée au rendu Bevy) DISPARAÎT, le cœur tournant hors de la boucle de rendu.

Un binding FFI/cdylib (option A) reste possible plus tard SI la frontière IPC se révèle un coût — mais on ne
le saura qu'après l'avoir **mesurée** (palier 1). On commence donc par le sidecar (option B, décidée le 22 juin).

---

## 1. Transport — TCP loopback

- **`127.0.0.1:47800`** (TCP). Réglable par `SIDECAR_ADDR`.
- **Pourquoi TCP loopback et pas une socket Unix** : le PC de test **A est Windows**, et le sidecar tourne
  chez CHAQUE joueur en local. TCP loopback marche identique sur Windows ET Linux ; Unreal le parle nativement
  (`FSocket`). En loopback, TCP n'a ni perte ni réordonnancement → pas besoin de gérer ça nous-mêmes.
- **Rôles** : le **cœur Rust = serveur** (il écoute, il a l'autorité, il tourne 24/7 côté joueur) ; **Unreal =
  client** (il se connecte au lancement, se reconnecte s'il tombe).
- **Un seul client à la fois** par sidecar (un joueur = un UE = un cœur). Connexion suivante = on remplace.

## 2. Cadre (framing)

Chaque message = **préfixe de longueur** + corps :

```
[u32 LE : longueur du corps][u8 : type][payload …]
```

`longueur` couvre `type` + `payload` (pas les 4 octets de préfixe eux-mêmes). Little-endian partout (comme le
wire réseau existant, cf. [message.rs](src/net/message.rs)). Pas de JSON : binaire à champs fixes, qu'UE décode
au `FMemoryReader`/`memcpy`.

## 3. Types de message

> Convention : **types < 128 = UE → Rust** ; **types ≥ 128 = Rust → UE**. Tous les `f32`/`u*` en little-endian.

### UE → Rust

| type | nom         | payload                                  | sens |
|------|-------------|------------------------------------------|------|
| `1`  | `HELLO`     | `u16 version`                            | UE s'annonce à la connexion (version = `1`). |
| `2`  | `PUSH_SELF` | `f32 x, y, z, yaw, pitch` (20 o)          | UE pousse MA pose courante (à sa cadence, plafonnée ~60 Hz). |
| `3`  | `PING`      | `u64 nonce`                              | mesure de latence IPC (palier 1) : UE attend le `PONG`. |

> La **couleur** de mon skin reste choisie par le cœur (`random_color`, [skin.rs](src/net/skin.rs)) et
> renvoyée dans `WELCOME` — UE ne la décide pas. L'orbe et les objets partagés = **palier 4** (pas dans v1).

### Rust → UE

| type  | nom        | payload                                                                 | sens |
|-------|------------|-------------------------------------------------------------------------|------|
| `128` | `WELCOME`  | `u8 id[32]` + `f32 r, g, b`                                              | mon identité (clé pub) + ma couleur, à la connexion. |
| `129` | `SNAPSHOT` | `u16 count`, puis `count` × **AvatarRec** (76 o chacun)                  | l'état des avatars distants, à ~20 Hz (`SEND_HZ`). |
| `130` | `PONG`     | `u64 nonce`                                                             | réponse immédiate au `PING` (RTT IPC). |

**AvatarRec (76 octets)** — un avatar distant, miroir de `PlayerState` sans `parent`/`seq` (inutiles au rendu) :

```
u8  id[32]      // clé publique = identité stable (UE en dérive un nom court : 8 hex)
f32 x, y, z     // position
f32 vx, vy, vz  // vitesse réelle (m/s) → UE INTERPOLE/extrapole côté rendu
f32 yaw, pitch  // orientation corps / tête (radians)
f32 r, g, b     // couleur du skin
```

## 4. Cadence & responsabilités

- **Rust** émet un `SNAPSHOT` à **20 Hz** (`SEND_HZ`, [state.rs:12](src/net/netcode/state.rs#L12)) : la liste
  COMPLÈTE des distants connus frais (timeout `REMOTE_TIMEOUT` = 5 s). Pas de delta v1 (simple d'abord ; à 20 Hz
  × ~32 voisins × 76 o ≈ 49 Ko/s en loopback = négligeable).
- **UE** pousse `PUSH_SELF` à sa cadence (plafonnée ~60 Hz pour ne pas noyer le cœur) et **interpole** les
  distants localement avec `vx,vy,vz` + un petit retard (`INTERP_DELAY` = 0,10 s). **Le ressenti se fabrique
  côté client** (leçon de l'utilisateur : on triche le vivant, on ne le court pas en ms).
- Le cœur garde toute l'autorité : anti-triche, anti-rejeu, sceau, relais NAT — UE ne voit que des poses déjà
  validées. UE ne peut PAS injecter d'avatar : il ne fait que pousser SA pose et lire les distants.

## 5. Les paliers (chacun PROUVÉ avant le suivant — « béton », pas « d'un coup »)

- **Palier 0 — CE fichier.** Contrat figé sur le papier. ✅
- **Palier 1 — preuve de vie IPC + MESURE de latence.** Sidecar Rust **bidon** : 2-3 faux avatars en cercle,
  `SNAPSHOT` à 20 Hz, accepte `PUSH_SELF`, répond `PONG`. Sous-commande `jeu sidecar` (réutilise le cœur déjà
  engine-agnostic, **sans extraire de crate** pour l'instant).
  - ✅ **Côté Rust PROUVÉ** (`jeu sidecar`, faux-UE Python) : les deux sens vivent (61 SNAPSHOT/3 s à 20 Hz,
    177 PUSH_SELF reçus), **RTT IPC loopback = médian 47 µs / p95 67 µs / max 95 µs**. L'IPC n'est PAS un mur
    (~0,05 ms ≪ le budget réseau dizaines de ms / rendu 16 ms). ⚠ Mesuré avec un faux-UE Python, pas encore le
    vrai client C++ Unreal.
  - 🎯 **RESTE** : client C++ Unreal qui se connecte, bouge des capsules depuis les `SNAPSHOT`, pousse
    `PUSH_SELF` depuis le perso, et affiche le RTT `PING→PONG` à l'écran → palier 1 réellement clos.
- **Palier 2 — vrai cœur branché.** `PUSH_SELF` alimente l'émission réelle (gossip/relais) ; `SNAPSHOT` vient
  des vrais `last_state`. Testable en loopback réseau sur une machine.
- **Palier 3 — preuve réelle 2 humains.** Deux joueurs (UE + sidecar chacun) via le **rendez-vous relais déjà
  prouvé**, qui se voient bouger DANS Unreal. **Juge = le log serveur** (point neutre), pas les fenêtres.
- **Palier 4 — orbe & objets partagés.** Après nettoyage des types Bevy de [orb.rs](src/net/orb.rs)
  (`Vec3`/`Res<Time>`) et extension du contrat (`OBJECT` msg). Hors v1.

## 6. Doutes gardés en tête (à confirmer par la mesure, pas l'argument)

- **Latence/jitter IPC** : l'inconnue n°1 → c'est tout l'objet du palier 1. Tant que non chiffrée : ne rien
  déclarer « prouvé ».
- **Cadence d'écriture TCP** : à 20 Hz un `write` par snapshot ; si Nagle ajoute du retard, poser `TCP_NODELAY`
  des deux côtés (à vérifier au palier 1, pas à supposer).
- **Reconnexion** : si UE tombe et revient, le cœur continue seul (il garde l'identité persistante `a.key`) ;
  UE re-`HELLO` et reçoit le prochain `SNAPSHOT`. Pas d'état à resynchroniser (snapshots complets).
- **`web3core` pas encore extrait** : on ne l'extrait PAS au palier 1 (raccourci propre : `jeu sidecar` n'importe
  pas Bevy). Le 1er vrai besoin d'extraction = quand l'orbe (palier 4) traînera des types Bevy.
