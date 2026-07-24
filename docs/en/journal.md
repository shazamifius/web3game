# Development journal — how the core was built

*[Version française](../journal.md)*

> The project's story, step by step: from the 3D sandbox to the hardened network core. We advanced **by coding
> for real**, from the simplest (two machines talking) to the hardest (hundreds of players, anti-cheat). Each
> step was compiled, tested and proven before the next.
>
> See: [the idea in plain words](comprendre-le-p2p.md) · [the measured state](etat-du-projet.md) · [the doubts](doutes.md).

---

## Chapter 0 — The 3D sandbox
A room, an articulated character, a first-person view. The minimal playground on which everything else will
be built.

## Chapter 1 — Raw transport
Hand-made UDP: encode a position into bytes, send it, receive it. Two windows see each other move. Orientation
(body + head) and a colour travel in the packet.

## Chapter 2 — Netcode: smoothness and prediction
The heart of "it moves well" despite the network. Positions arrive ~20 times per second; each is stored in a
queue of timestamped snapshots, and the avatar is drawn **~100 ms in the past** (interpolation delay), sliding
between the two snapshots that surround it. When a packet is missing, we **predict** the continuation by
extrapolating the velocity rather than freezing the avatar, then **reconcile** smoothly (damped spring) when
the real packet arrives. Each avatar even has its own reading clock, which speeds up or slows down a little to
catch up **by walking** rather than teleporting.

> Design note: prediction is done "by hand" — the physics of human inertia is enough over 100 ms, it is
> deterministic, readable, and free in computation (no need for a neural network for that).

## Chapter 3 — Topology and scaling
- **N players + a rendezvous point**: a directory introduces the players; each registers, then sends its
  state **directly** to the others.
- **NAT & hole-punching**: the router drops unsolicited incoming packets, but *sending* opens a "return
  hole". Both peers therefore send a simultaneous salvo: the first packets die, the following ones get
  through → **direct connection, no relay**.
- **Area of interest by budget**: we never cut anyone off by rule; we **distribute an emission budget** among
  peers according to their relevance. Few people → full rate for all; a crowd → gentle degradation, never
  zero.

## Chapter 4 — Authority and migration
- **A first shared object** that belongs to no one: the last to touch it becomes its **master** (it simulates
  its physics and broadcasts it), and authority **jumps from hand to hand**. Conflicts are settled by a
  (version, identity) pair — a **deterministic** tie-break, with no server.
- **Migration**: if the master goes silent, everyone elects the **same** deterministic replacement, without
  voting; any "split-brain" resolves on its own.
- **The relay ("parent")**: a player with low upstream sends its state once to a parent, which copies it to
  its neighbours — the identity stays the author's. *The relay carries the bytes, it does not arbitrate*: two
  distinct roles.

## Chapter 5 — Trust and anti-cheat (the foundations)
- **Signed identity**: each session has a key pair; every state is **signed** and verified. You can no longer
  impersonate someone else, nor falsify the state you relay. *(Cryptography lives in a single file — the only
  acknowledged "black box"; we never write our own crypto.)*
- **Anti-replay**: a monotonic counter prevents replaying an old packet.
- **Signed orb + bounds**: the master signs the object, and an aberrant version jump (an attempt to lock it
  for life) is refused **and** counted as a fault.
- **Local reputation**: an attributable cheater is muted. Key anti-*framing* rule: we **never** accuse on an
  invalid (unattributable) signature, only on a packet **validly signed but cheating**.
- **A real attack harness**: an adversarial program, on real sockets, that proves the robustness.

## Chapter 6 — The "concrete" overhaul: full hardening
We went back over every piece to close the gap between "a few known attacks neutralised" and the real goal: a
crowd in pure peer-to-peer, facing a maximum of adversaries, **that holds**.

> *Honest frame set from the outset:* serverless P2P at this scale, facing byzantine adversaries, is at the
> frontier of research — we do not promise the absolutely inviolable (it does not exist). We aim for: **each
> attack becomes either impossible, or expensive, or attributable and banned.**

Ten "holes" were closed or bounded, of which the most structural:

- **Self-certifying identity** (the keystone): identity **is** the public key, carried in every packet; the
  rendezvous **can no longer lie** about "this key = this player". Along the way, the 255-player wall and
  identity collisions disappear.
- **An anti-Sybil entry cost** (proof of work): an identity is only valid once "mined" → a banned one no
  longer comes back for free.
- **Movement validation**: a teleport (incoherent distance and time) is refused and counted as a fault.
- **Proof of contact**: to become the master of an object, you must have been **near** it.
- **Bounded DoS**: capped memory (eviction), capped relay amplification.
- **Bounded neighbourhood** (~32 closest): it is *the* bound that makes scale possible — thousands of small
  neighbourhoods rather than one giant mesh.
- **Shared reputation**: an attributable cheater is banned by a **quorum** of distinct accusers (each costing
  a proof of work) → forging a fake quorum is expensive, and a lone liar can do nothing.
- **A massive simulation**: 50 then 300 nodes + a swarm of attackers on a single machine → all up,
  neighbourhood capped at 32, **shared object never stolen**. *What holds for large scale: the load per node
  does not depend on the total number — real scale is done by adding machines, not by overloading one.*

## Chapter 7 — In the wild: the measurement instrument and the first real remote nodes
Until now, all the proofs came from **simulation** or from **two machines**. But the project's founding doubt
— the "empty fortress" — does not lift like that: it demands **real people, on real networks, at real
distances**.

So we built a **measurement instrument**: a small **autonomous agent** (with no dependency at all, like the
core) that a volunteer runs at home. It joins the network, measures the **liveliness of the distant links** it
perceives — freshness, loss, jitter, reordering — and **sends back the figures**. The instrument itself is
required to be **honest**: visible, launched knowingly, and at low-consumption rest when it is not measuring.

**A false lead, first — and the most instructive.** The first readings announced **89 % loss** on the relay
path. Legitimate panic… then investigation: it was **not** a network failure, but **our own bandwidth thrift,
poorly measured** (the instrument compared the low-frequency "trickle", sent on purpose to distant peers, to
the full rate reserved for the close neighbourhood). We taught the instrument to **distinguish "not sent on
purpose" from "sent then lost"**. The full story lives in [behind the scenes](coulisses.md).

**The result, once the instrument was honest.** Volunteers spread across **several countries**, on **real home
networks** (some behind the hardest NAT, CGNAT), were measured **alive**: freshness **p95 ~200–335 ms** —
below the **500 ms** threshold we set for "alive" —, **real loss ~0**, verdict "alive".

**What it proves, and what it does not prove.** It is a **hard fact**: the **substrate** transports **real,
distant** presence, alive, over the real Internet — the infrastructure is no longer *empty*. But it does
**not yet** prove the **feel**: humans who **move and play together** in the same world, and **sense** it
alive. That test — the most important one — is still ahead. The doubt lightens; it does not close.

## Chapter 8 — The network learns to know its links
Chapter 7 had put the instrument in the wild and lifted a false alarm. A real question remained, raised by a
mediocre test link: **emission redundancy** (sending a state twice to resist loss) had, on that link,
*worsened* things instead of helping. Why?

The answer required teaching the network to **know itself**. We wrote a **link probe** (with no dependency,
like the rest) that, on each node, measures: the **NAT type** (punchable or not, by querying two public
servers from a single socket), the **latency**, the **jitter**, and above all the **nature of the loss** — a
short rising-rate burst reveals whether the link *saturates* (congestion) or loses *at random*. First
surprise, and a lesson in method: a consumer mobile phone we thought "blocked" (symmetric NAT) turned out to
be **punchable**. Measurement, once again, corrected the intuition.

Above all, the probe explains the failed redundancy: this mobile link was not losing *at random*, it was
**saturating**. Yet duplicating a state on a saturated link doubles the traffic of an already-full pipe → it
gets worse. Redundancy only helps *random* loss. Hence the answer: an **ADAPTIVE redundancy** — each node
reads its own probe and only duplicates if it sees random loss with headroom, **never** on a link that
saturates. We saw it happen for real (a congested link gave up duplicating on its own); and we proved the
other half on a controlled *random*-loss bench (network loss injected by the kernel): **30 % → 9 %** at two
copies, **2.8 %** at three — loss divided as theory predicts, on real packets.

The full story of this double correction lives in [behind the scenes](coulisses.md) (investigation #3); the
technical detail, in [network work](chantier-reseau.md).

## Chapter 9 — The hardening: breaking your own code on purpose
The network core "worked". But in a peer-to-peer network, each node receives packets written by anyone:
*working* is not enough, it has to **hold under attack**. So we ran, on 22 July 2026, a deliberate hardening
pass — not "does it run?", but "what breaks if someone wants it to?". It found **16 defects**, including a
personal-data leak and a way to turn a node into an **amplification weapon**. All 16 are fixed, each
accompanied by a test that fails if the fix disappears.

The lesson is not the list, it is the **method**: no single technique is enough. A shared-object theft slipped
past 11 unit tests, property tests, a thread-race detector and **240 million fuzzing packets** — only a
**real-conditions attack bench** saw it. Conversely, a formal proof
([Kani](https://model-checking.github.io/kani), from AWS) broke, in seconds, a function that those 240 million
attempts had judged sound. The full, quantified story is in **[the security page](SECURITE.md)**.

## Chapter 10 — The launcher: the invisible network becomes a gesture
Until now, all the work was **headless** — network you measure, not that you see. This chapter gives it a
body: a native **launcher** that makes two Unreal worlds coexist (a hub, an island) and orchestrates the
**crossing from one to the other**. You cross a portal, you change world — and the old one **never** closes
before the new one has proven that it displays a frame (the same reflex as everywhere: do not break what works
before the replacement has proven itself). It is judged **smooth** in real play.

This chapter also offered the project's finest lesson in humility: a 200,000-step simulation validated the
switch… on a **false model of the real world**, and only the real machine disproved it. The mechanism, what
it proves and what it still lacks (the island is a dead-end): **[launcher work](chantier-launcher.md)**, and
the full investigation in **[behind the scenes, #5](coulisses.md)**.

## What comes next
The crossing mechanism holds; what is missing is the **world at the end** — a return portal, places you want
to stay in. The work continues in the dedicated work items: the [real network](chantier-reseau.md), [the dense
crowd](chantier-foule.md), [robustness](chantier-robustesse.md), and the [launcher](chantier-launcher.md).

---

*History kept up to date as development goes.*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **The development journal** — a step of the **🧭 Understand everything** path (the natural
sequel to [behind the scenes](coulisses.md)).

**Continue the thread:**
- 🧭 *Understand everything* → **[Architecture & code](ARCHITECTURE.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
