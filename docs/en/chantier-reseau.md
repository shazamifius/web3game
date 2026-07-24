# Network work — confront reality, harden trust, own your identity

*[Version française](../chantier-reseau.md)*

> The technical detail of three fronts of the network core: (1) confronting it with real Internet conditions
> (latency, loss, NAT); (2) hardening trust against coordinated cheating; (3) a persistent and private
> identity. We also show here, **without hiding them, the hypotheses that measurement refuted** — that is
> where rigour is won.
>
> See: [the doubts addressed](doutes.md) · [the measured state](etat-du-projet.md) · [the idea in plain words](comprendre-le-p2p.md).

---

## 1. Confront the network with reality (latency, loss, NAT)

**The problem (D1).** A simulation on a single machine — no latency, no loss, no NAT — can be perfect and yet
collapse on the Internet.

**The approach.** Inject real network conditions with `tc netem` on the loopback interface (latency, jitter,
loss, reordering), following three profiles (`good` ~30 ms · `medium` ~120 ms + 2 % · `bad` ~250 ms + 5 % +
reordering), then run the simulation behind it. *(A subtle detail settled along the way: on loopback, the
delay counts double — round-trip on the same interface — so the script applies half the targeted ping.)*

**The result — and a hypothesis refuted by measurement.** First finding: honest throughput dropped by
**−70 %** under the bad profile. Starting hypothesis: strict anti-replay (which rejects reordered packets). So
we fixed it — moving to a **sliding-window** anti-replay (like IPsec / WireGuard), useful and necessary since
real networks reorder. **But measurement decided: that only recovered +15 %, not the 70 %.** The real cause
was a **test-bench artefact**: `tc netem` caps its queue at 1000 packets by default, which imposes a
throughput ceiling ≈ queue ÷ delay. Widened queue → honest throughput **climbs back to optimal** (≈ −9 %
under the worst profile, i.e. essentially only the 5 % loss).

→ **Conclusion:** the protocol **holds under a real network** (250 ms + jitter + 5 % loss + reordering); the
drop was not in the protocol, but in the instrument. *The lesson is worth it: without the discipline "a
contradicting measurement outranks the hypothesis", we would have polished the wrong place.*

**Security holds too under a bad network**: at 250 ms + reordering, orb **0 stolen**, teleport / increment
cheat / Sybil neutralised.

**Real NAT.** Hole-punching is proven between real "routers" (set up in network namespaces): **full mesh**
for "cone" NATs, expected failure for symmetric NAT → fallback to relay (see D17).

**Characterise each link — and a third hypothesis refuted (D36).** You cannot help a link you do not know.
Each node therefore **probes** its own, with no external dependency: **NAT type** (query two public STUN
servers from a single socket — same public port seen from both sides = *punchable cone*, different port =
*symmetric*; hand-made STUN implementation), **latency** and **jitter**, and above all the **nature of the
loss** (a short rising-rate burst: if loss rises with the rate, it is **congestion**; if it stays flat, it is
**random**). *Starting hypothesis: "a 4G/5G mobile link is symmetric NAT, so to be relayed by default."* **The
probe refuted it:** a consumer phone tested turned out to be **cone — directly punchable**. What varies with
coverage is the **quality** (latency, jitter), not the NAT type.

**Redundancy is not free — hence the adaptive approach.** On a lossy relay, one can send each state in several
copies (concretely: a single packet carrying the **K last states**, so with no packet-count overhead). Theory
says that with **independent random** loss `p`, residual loss falls as `pᴷ`. But that is true **only** for
random loss: on a **saturated** link (congestion), bigger copies only worsen the saturation. We checked it two
ways:

- a **deterministic bench** (`phase1`, fixed seed) that models the three regimes: random → redundancy divides
  (~`pᴷ`); burst → the gain collapses (consecutive copies fall in the same burst); severe congestion →
  redundancy **useless** (0 gain for 2–3× the bytes);
- a **real bench** (`netem-bench`) that runs the **real** mechanism through **real** loss injected by the
  kernel (`tc netem`), in a disposable network namespace (so without touching the machine). Result on
  **random** loss: `30 % → 9.0 %` at 2 copies (predicted 9.5), `2.8 %` at 3 (predicted 2.9); `50 % → 25.2 %`
  then `12.8 %` — the measurement **matches** `pᴷ`. On **correlated** loss (burst), the gain degrades,
  exactly as predicted.

**Conclusion: ADAPTIVE redundancy.** A node only duplicates if it has measured **random** loss with headroom;
never on a saturating link. This decision, made by the node from its own probe, has already been observed
under real conditions (a congested mobile link, on its own, **gave up** duplicating). *What remains open: the
great variety of real-world connections — that is doubt **D36**, which we are only starting to map.*

## 2. Harden trust (Sybil, eclipse, accusations)

**The problem.** With no central authority: how do you prevent an attacker from manufacturing fake identities
(Sybil), isolating a victim (eclipse), or getting an innocent banned (*framing*)?

**Costly identities — three layers, not three rivals (D6).** Making an identity "expensive" pits two goals
against each other: the more expensive it is (in computation), the more it punishes the **weak** player (a
phone that would have to "mine" for a long time before playing). The good design is *expensive for the
mass attacker, light for the isolated honest one*. Three complementary layers: **(a)** a minimal base;
**(b)** a difficulty that **rises locally under attack** and comes back down when calm; **(c)** later, a
**social sponsorship** (the cost becomes a relationship, not computation — the friend of the weak). *These are
not options to choose between, but layers to lay in the right order.*

**The "credible witness" (anti-framing, D7).** Counting distinct accusers is naive: three fake identities
would suffice. The answer: **sum a credibility weight**, not heads. An accuser only weighs if they have
**already participated** in the world (sent real signed states — a Sybil that only spits out accusations
weighs **zero**) and if they were a **plausible witness** (close enough to have been able to see the cheat).
Proven by the `sybil-frame` attack, which then flips from "framing succeeded" to "framing failed".

**Anti-eclipse — and a second hypothesis refuted (D9).** Initial plan: diversify the neighbourhood
"Kademlia-style" (by identity distance). **Wrong tool in our model**: since an identity is a ~random key, the
Sybils spread out exactly like the honest ones → it distinguishes nothing. The right lever (Bitcoin /
Ethereum's) is **IP address diversity**: an attacker manufactures identities for free, but has only a handful
of addresses. An accusation's contribution is therefore **capped per subnet** → a thousand fake accounts
behind a single IP = **a single voice**.

**Corroborated positions (D9).** Positions "reported by a third party" (gossip) are forgeable: they serve only
discovery, **never** to judge an accusation — for that, we only use the positions **signed** by the peer
itself.

**Rehabilitation (D8).** A fault is no longer a life sentence: faults **fade** in a sliding window.

> *Honest limit:* corroboration per subnet is only proven in logic and simulation; the test on **real diverse
> IP addresses** (and the "botnet" residue) remains to be done — it is a fundamental limit of peer-to-peer.

## 3. Persistent identity & privacy

**Persistent identity — done (D14).** The key is mined **once**, saved locally (like an SSH key, restricted
access) and **reloaded** at launch → a real "account" from one session to the next, with no account server.

**Privacy — to come (D15).** Today positions travel in **the clear** (the signature guarantees authenticity,
not secrecy). Per-pair transport encryption (X25519 key exchange) is planned, but **deliberately deferred**:
in R&D, plaintext makes understanding and debugging the network easier.

---

*Technical facts kept up to date as measurements come in. The cost and throughput figures: [state
review](etat-du-projet.md).*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Network work** — a step of the **⚙️ The code** and **🧭 Understand everything** paths.

**Continue the thread:**
- ⚙️ *The code* · 🧭 *Understand everything* → **[Dense-crowd work](chantier-foule.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
