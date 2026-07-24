# The doubts — the open register (and their answers)

*[Version française](../doutes.md)*

> An honest R&D project is recognised by its **acknowledged doubts**. Here is the complete inventory of risks
> and open questions, numbered and tracked — **with, for each, the answer or the precise lead**. *A doubt is
> not a weakness to hide: it is the object of the work.* And a doubt is only "closed" when a **measurement**
> proves it; otherwise it receives an *answer* (a lead), and its status stays open.
>
> See also: [the idea in plain words](comprendre-le-p2p.md) · [the project's measured state](etat-du-projet.md) · [README](../../README.en.md).

**Status** — ✅ closed & proven · 🟡 bounded / partial · 🔴 open · 🧭 reclassified (principle, scope, or watch).

> **🔭 A cross-cutting answer — "corroborated esteem".** Several doubts (D4/D5, D6, D11, D26, D28) converge on
> **a single mechanism**: you are **born invisible** and **earn your visibility** through your behaviour;
> peers **cross-check** what each one declares (you take no one at their word when they speak about a
> *group*). A throwaway identity is worth nothing until it has been socially earned → Sybil, an aggregator's
> lie and state forgery lose their point, **with no reputation currency or heavy proof of work**. *Elegant —
> but to handle with care: a mechanism carrying five defences becomes a point of fragility if it has a flaw.*

---

## The realism of the tests

- **D1 — The tests "lie" like localhost.** ✅ *A sim with no latency/loss/NAT can be perfect and collapse on
  the Internet.* **Answer:** real network conditions injected with `tc netem` (latency, jitter, loss,
  reordering) + NAT in namespaces (`ip netns`) — a single machine is enough to reproduce a truly bad network.
- **D2 — The simulation bot is not exactly the game.** 🟡 *Two duplicated network loops drift silently
  (already seen: a wrong measured cost, 89 instead of 34 KB/s).* **Answer:** have the bot drive **the same
  functions** as the client (already done for the throughput computation); the *sidecar* bridge will make the
  Rust core **the single loop**, driven by both the bot and the engine — the duplication disappears by
  construction.

## Inclusivity (the heart of the vision)

- **D3 — A weak link cannot keep up, on reception.** 🔴 *The area of interest bounds emission, not reception:
  in a crowd a player receives ~43 KB/s and drowns.* **Answer (3 levels):** (1) **bilateral** area of
  interest — the receiver **announces a budget in KB/s**, senders respect it; (2) **adaptive detail** — the
  weak one shows fewer sharp neighbours + the crowd as a summary; (3) for the very weak, a **parent
  aggregates** and only sends a low-frequency summary.
- **D4 + D5 — Who relays for the weak, and how do you prevent a "black hole" relay?** 🟡 *The weak role is
  self-declared; a parent that drops packets makes you invisible silently.* **Answer:** choose the parent by
  **actually observed capacity** (the throughput/freshness we receive from it — unforgeable), **never
  declared**; the assignment is **local** (everyone, in their zone, takes the best available parent); in
  scarcity we **degrade fairly** rather than exclude; the "black hole" relay is detected when neighbours never
  confirm having received my state through it → we change parent. **No imposed leader.**

## Sybil & reputation

- **D6 — Proof of work is a "toy".** 🟡 *Mining an identity costs ~1 s → hundreds in minutes.* **Answer:**
  rather than a heavy proof of work (a bad fit here), **social esteem** (see box) — you are born invisible,
  you earn your visibility; a banned cheater who recreates an identity **starts from zero**, so ban evasion
  yields nothing. The ultimate sanction, in a social space, is **invisibility**.
- **D7 — An accusation quorum allows *framing*.** ✅ *3 mined identities could get an innocent banned.*
  **Answer (done):** accusations **weighted by the credibility** of the accuser + **address diversity** (cap
  per /24 subnet) → a lone attacker does not reach quorum. Proven by the `sybil-frame` attack.
- **D8 — No rehabilitation, no expiry.** ✅ *A muting was permanent (unjust + memory leak).* **Answer
  (done):** faults **decay in a sliding window**; an unjustly muted node becomes audible again after a period
  of good behaviour.

## Topological trust

- **D9 — Position is not verified → eclipse attack.** ✅ *A node could lie about its position to insert
  itself into every neighbourhood and isolate a victim.* **Answer (done):** **forced diversity** of the
  neighbourhood (don't take all your neighbours from the same source/subnet) + **corroboration** of positions
  + **/24 cap**.
- **D10 — The rendezvous point remains a centralisation.** 🟡 *It sees the addresses; it is a single point of
  failure.* **Answer:** bounded today (rate-limit + proof of work at entry); eventually, downloading worlds
  over **BitTorrent** and hosting by **volunteer machines weighted by esteem**. *Honest note:* the
  infrastructure will remain "**no central authoritative server**", never "zero servers".

## The authority of shared objects

- **D11 — Reclaiming an object's authority is the soft spot.** 🟡 *A patient attacker waits for the master's
  silence then takes the object.* **Answer:** two leads — (a) validate the takeover with a **quorum of
  neighbours** (a lone attacker doesn't reach quorum); (b) **better: do without a master**. A resolution by
  **convergent deterministic rank** (already tested on one case) assigns the object with no master and no
  migration, so there is nothing to steal — that is the model we are trying to generalise.
- **D12 — Everything is coded for a single object.** 🟡 *A real world has thousands of shared objects.*
  **Answer:** a **generic registry** `{id, type, authority rule, state}`; each object type (pickable,
  portable, gravity-bound, door…) plugs in **its** rule. Two patterns already exist (authority by migration;
  authority by convergent rank) → we extract the common registry as we wire things up.
- **D13 — No common clock → conflicts poorly settled.** 🟡 *"Who touched first?"* **Answer:** a
  **deterministic (version, identity)** tie-break is consistent for everyone and **is enough** for calm
  games; only millisecond-competitive play requires a real shared time → it will be an **optional service of
  the base layer**, enabled by the games that need it, never imposed on all.

## Identity & privacy

- **D14 — Identity was not persistent.** ✅ *No "account" from one session to the next.* **Answer (done):**
  the key is mined **once**, saved locally (like an SSH key, restricted permissions) and **reloaded** at
  launch → same identity across sessions, distinct profiles = distinct keys.
- **D15 — Everything travels in the clear.** 🟠 *Positions (and soon voice) readable; the signature proves
  authenticity, not secrecy.* **Answer:** **per-pair** transport encryption (X25519 key exchange).
  **Deliberately deferred** to the public release: in R&D, plaintext makes understanding and debugging
  easier. *Note:* throughput/latency will need to be **re-measured** afterwards (encryption changes packet
  size).

## Robustness & longevity

- **D16 — Long-term memory leaks.** 🟡 *Per-peer records accumulate.* **Answer (partly done):** **eviction by
  lifetime** — a long-absent peer yields its place, **never an active one**. Housekeeping, with no impact on
  the current game.
- **D17 — Symmetric NAT → relay mandatory.** 🟡 *Some connections (mobile) do not punch through directly.*
  **Answer:** the **relay via the rendezvous point is proven for real** (two distinct networks, both ways).
  Still open: a **decentralised** relay (any good node relays) + an **automatic trigger** ("punching
  abandoned → relay"). It is the **floor** of a reliable onboarding (see D34). ⚠️ *And the relay is **not
  free**: its quality depends on the link type — a naive redundancy can even **worsen** a saturated link (see
  D36).*
- **D18 — The anti-cheat speed threshold is crude.** 🟡 *A subtle cheater stays just under the threshold.*
  **Answer:** two layers — (1) a **non-negotiable base** that protects the network for everyone (signatures,
  anti-Sybil, anti-forgery), always on; (2) game rules **tunable by the world's creator** (an "aggressive /
  loose anti-cheat" slider), calibrated on the real speed of *their* game.

## Meta-doubts (about the approach)

- **D19 — The real per-node cost had never been measured.** ✅ **Answer (measured):** **~34 KB/s ↑ (max ~38),
  ~31 KB/s ↓, ~0.7 % of a core, ~38 MB of RAM** per node → ~0.27 Mbit/s, **bounded by the neighbourhood
  (~32), not by the total** → does not move at 55,000 (scaling is done by adding machines).
- **D20 — Combined / adaptive attacks never tested together.** 🟠 *A real adversary combines and adapts
  (plays honest then betrays).* **Answer:** a coordinated "scenario" mode — a **long-term goal**, not urgent.
- **D21 — The security of the rendezvous point itself.** 🟡 *It can be flooded with valid messages.*
  **Answer:** rate-limit + eviction already in place; for the raw volumetric layer, **rely on proven tools**
  (anti-DDoS), while keeping **in-house** the *application* limits (that only our code understands).
- **D22 — The dense crowd.** ✅→🟡 **Answer (partial):** the **bounded bandwidth is proven** (perception ~87 %
  at 1,000 nodes, flat inbound). **But reopened:** the *feel* and *relevance* of the crowd are not proven →
  that is the object of D29/D30.
- **D23 — Gossip is a DDoS amplifier.** ✅ **Answer (done):** proof of work required on each "business card",
  abandonment of uncorroborated punching (~10 s), learning rate-limit per source — proven by `gossip-flood`
  (0 reflected punching).
- **D24 — The visible crowd was capped at 64.** ✅ **Answer (done):** "two-thirds" rendering (a few detailed
  close ones + a crowd of cheap impostors) → far more than 64 silhouettes on screen with no performance drop.
- **D25 — The bench caps (~1,500 nodes); "55,000" is never measured directly.** 🟡 *The bench spawns one
  thread per node → it is the machine that saturates, not the protocol.* **Answer / framing:** from a
  player's viewpoint, the world is **always ~32 sharp neighbours + 1 aggregated crowd** — the total never
  touches one machine. The "1,500" is a **bench artefact**, not a game limit. Beyond ~2,000, it is
  **architectural extrapolation**, never "proven".
- **D26 — A crowd's aggregator can lie** (hide or invent people). 🟡 *The signature proves who speaks, not
  that the summary is honest.* **Answer:** never believe **a single** node about a **group** →
  **corroborate** (cross-check K independent informants + one's own perception). **Esteem** closes the rest:
  an invented "ghost", with no esteem, is **uncountable**. *(Key measure: electing a "cell leader" was the
  dominant wall — we removed it.)*

## Scope — the doubts that can decide the project

- **D27 — "The empty fortress".** 🟡 *Have we built a fine infrastructure in which two humans have never
  really met, in motion, over the real Internet?* **Answer:** there is only one honest one — **the test in
  the wild**, a pre-registered criterion (latency ≤ 500 ms = playable), hostile conditions included. **First
  hard fact (28 June):** a **measurement instrument** (an agent that volunteers run) recorded, on **real
  remote links** (several countries, incl. CGNAT), an **alive** presence — freshness **p95 ~200–335 ms**
  (< 500 ms), **real loss ~0**, verdict "alive". The infrastructure is therefore **no longer empty**: the
  substrate transports real, alive, remote presence over the real Internet. **What remains (and keeps the
  doubt open):** it measures the **substrate**, not the **feel** — humans who **move and play together** and
  **sense** it alive (and the role of voice, D35). **Progress (29 June):** an [objective measurement
  bench](chantier-vivant.md) turns "is it alive?" into figures (shape fidelity, freshness, smoothness); a
  lever — predicting motion with **acceleration** — reaches the "alive" verdict from as little as **~100 ms
  of effective delay** on the worst link (congested 4G), beating the 150 ms ambition. But it is a **reference
  model**, not the played experience. The doubt lightens; it does not close.
- **D28 — Player-state persistence without a server.** 🔴 *Progress that survives sessions, with no central
  store: where does state live, and who prevents forging it?* **Answer:** for a first attempt,
  **ephemeral/local** is enough (among friends). The real answer — "**your key signs your state, peers
  corroborate**" — rests on esteem: a sudden, uncorroborated jump (e.g. an inventory that explodes) is
  **rejected**.

## Perception, scale & onboarding

- **D29 — Area of interest by *proximity* ≠ by *social relevance*.** 🟠 *Seeing "the 32 closest" is not
  seeing "the 32 that matter": if a neighbour talks to someone out of sight, that person stays invisible —
  annoying.* **Answer:** a **tiered** interest set, recomputed continuously — (T0) spatial neighbours
  (bounded by a *budget*, plus an arbitrary cap); (T1) the neighbours' **interaction partners**, pulled by
  **transitivity** (each signed state announces "I am engaged with {a few identities}"); (T2) explicit
  relations (friends). *Acknowledged limit:* we don't rank what we don't yet perceive, and a physical ceiling
  remains at 55,000 — it simply becomes **smart**. **Measured progress (30 June):** the T1 level is **wired
  and proven on the bench, end-to-end** — each player can announce "I am engaged with {a few identities}" (a
  **signed** message, separate from the state stream so as not to weigh down the hot path), and selection
  takes it into account by transitivity: in a growing crowd, a **distant partner keeps full rate** (20 Hz,
  flat) where proximity-only selection would starve it progressively — up to a **×100 fidelity gap**.
  *Honest limit: proven on a perfect-network bench; real conditions (latency, loss, NAT) remain to be
  measured — the status does not move until it is seen in the wild.*
- **D30 — Level of detail is not adaptive, and the crowd was never *rendered*.** 🟠 *The system jumps
  abruptly from "sharp" to "nothing" beyond the neighbourhood, whether the crowd is 40 or 55,000; and no one
  has yet looked at a rendered crowd.* **Answer:** a **continuous** fidelity — under budget, everyone sharp;
  above, degradation **sharp → silhouette → density field** ("the crowd is going that way") — adapted to each
  one's hardware and link, with **guaranteed mutual visibility** for pairs that interact. **Progress (30
  June):** the "who is relevant" brick is laid and measured (see D29); continuous degradation and the
  *rendered* crowd — the heart of this doubt — remain untouched.
- **D31 — A world's geometry must never throttle the network.** 🧭 *Became a principle.* **Answer:** since
  anyone will build any world, **the network adapts to the creators, never the other way around**; it
  degrades gracefully (down to the physical limit), never blocks or excludes.
- **D32 — "Is the game *fun*?"** 🧭 *Out of scope.* **Answer:** fun is designed and built (the creators'
  craft); this repository does the R&D of the **substrate**. The quantity that concerns it is **latency**
  (≤ 500 ms).
- **D33 — Can a single core serve the *rich* state of a game AND the *thin* state of a 55,000 crowd?** 🧭 *A
  watch item.* **Answer:** these are two technical regimes; we **watch** that they coexist rather than
  "forking", instead of deciding too early.
- **D34 — Onboarding could starve every test.** 🟠 *Install, cross the routers, meet up: a wall of real-world
  usage that can make a test fail "because no one managed to connect".* **Answer:** a launcher that
  **learns, over real connections, to cross a maximum of configurations**; with the **relay (D17) as the
  floor** guaranteeing "never zero: failing direct, relay".
- **D35 — Proximity voice, a load-bearing wall of "is it alive?".** 🟠 *The social feel rests partly on it,
  and it is the most deferred piece.* **Answer:** "who I hear" = "who is in my relevance set" → proximity
  voice **is** the area of interest (D29) applied to audio, to be built **on top of** this work (the
  capture/spatialisation on the engine side can be prepared in parallel).
- **D36 — The diversity of connections is a wall we have not (yet) mapped.** 🟠 *We have only validated
  transport on a handful of links; yet each type of connection breaks **differently**, and the real risk is
  the one we have not yet met.* **Hard finding (29 June):** on a **real degraded mobile link**, adding
  **emission redundancy** (sending each state twice over the relay) did **not** reduce loss — it **worsened**
  it. The lesson: when loss comes from **congestion** (saturated throughput), duplicating **aggravates**;
  redundancy only helps **random** loss (a link that still has headroom). **What we did since — and proved:**
  **(1) ✅ characterise each link** — a **probe** now measures, per node, the NAT type (punchable cone vs
  symmetric, via hand-made STUN), latency, jitter and the **nature of the loss** (random vs congestion). It
  has already **refuted a hypothesis**: a consumer 4G/5G mobile we thought *symmetric* is in fact
  **punchable**. **(2) ✅ ADAPTIVE redundancy** — a node only duplicates if its probe sees *random* loss with
  headroom, **never** on a saturated link; observed for real (a congested link **gave up** duplicating on its
  own), and the **gain** on random loss is proven on a real bench (`tc netem`: 30 % → 9 % at 2 copies, ≈
  `pᴷ`; in bursts, the gain collapses — consistent with theory). **What remains wide open: (3) the map of the
  real regimes** — **satellite** (Starlink: CGNAT + latency spikes + changing IPs; geostationary: > 500 ms by
  nature), **public wifi**, networks that **block UDP** (enterprise/hotel), **double-NAT**, **handover between
  antennas** in motion. We now have the **tool** to diagnose them; we simply have not yet **met** these
  links. *Method honesty: it was the chance of a mediocre test link that opened this doubt — without it, we
  would not have seen it.*

---

*This register is alive: we add a doubt as soon as we discover one, and reality — a measurement, a test, a
real player — is always right against this document.*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **The register of doubts** — a step of the **🌱 Discover**, **🔎 Judge fast** and **🧭
Understand everything** paths.

**Continue the thread:**
- 🔎 *Judge fast* · 🧭 *Understand everything* → **[Behind the scenes](coulisses.md)**
- 🌱 *Discover* → ✓ you are at the end of this path. To go further: ⚙️ [the code](ARCHITECTURE.md) or 🧭 [understand everything](README.md).

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
