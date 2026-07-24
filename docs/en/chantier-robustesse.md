# Robustness & longevity work — generalise authority, endure, cross hard NATs

*[Version française](../chantier-robustesse.md)*

> Beyond the network core: generalise the authority of a single object to thousands, hold over long sessions,
> and above all **make two players meet even when their routers refuse the direct connection**. We show here,
> without hiding it, **a bug no lab test could see — and that only a real player in the wild revealed**.
>
> See: [the doubts addressed](doutes.md) (especially D11, D12, D17, D16) · [the measured state](etat-du-projet.md) · [the idea in plain words](comprendre-le-p2p.md).

---

## 1. Generalise the authority of shared objects

The whole authority machinery was first written for **a single object** (a test "orb"). A real world has
thousands (doors, scores, projectiles). The plan:

- **A generic registry** `{id, type, authority rule, state}`: each object type plugs in its own rule, reusing
  the existing machinery (D12).
- **Authority takeover by quorum**: when an object's "master" goes silent, the takeover is only valid if a
  quorum of neighbours confirms — a lone attacker cannot steal the object during the silence (D11). *A
  master-less approach (resolution by deterministic rank) is otherwise preferred where possible: nothing to
  steal.*
- **A signed timestamp** to arbitrate "races" (who touched first?) where a competitive game requires it (D13),
  and a **finer movement anti-cheat** (velocity + acceleration + coherence) (D18).

## 2. Endure, and not grow with the crowd

**Hold the memory (D16).** Per-peer records would accumulate over a long session: an **eviction by lifetime**
gives back the place of the absent (never that of an active one).

**A check that nothing explodes at scale.** A legitimate question: at 5,000 players, do too-large packets
become unmanageable? **Answer measured, not argued: no packet grows with the number of players** — all the
structures are capped by construction. The detail:

| Packet type | Max size | Grows with N? |
|---|---|---|
| Player state (signed) | 182 bytes | no (fixed) |
| Initial introduction (cap 32 neighbours) | ~1221 bytes | no (capped) |
| Discovery card (cap 16) | ~740 bytes | no (capped) |
| Cell summary v1 (cap 16) | ~759 bytes | no (capped) |
| Cell summary v2 (with signed proofs) | ~3672 bytes | no (capped) |

Everything stays under the safe size of a UDP packet (~1200 bytes), **except the v2 summary** (the
signed-proofs format): it exceeds it → it will have to be **split into independent chunks** before being used
at large scale. It is a **debt identified in advance** (not a scale-day surprise), and the format already
allows it (each proof is self-certifying, so it travels alone).

## 3. The relay for hard NATs (D17) — and a bug revealed by reality

**The problem.** Hole-punching fails on the most closed NATs (called "symmetric"). Those players cannot
connect directly → they need a **relay** (a third party that copies the packets). *Honest framing: it is a
**fallback** for the non-punchable minority, not the normal path — and our [link probe](chantier-reseau.md)
has since shown that this case is rarer than we thought (a consumer mobile we believed symmetric turned out
punchable).*

**Security is free, by construction.** A game state stays **signed end-to-end**: the relay can only **carry**
the bytes, **never falsify them**. So there is nothing new to prove on the trust side — just to bound the
amplification (one relay in = one out, capped rate, recipient necessarily registered).

**Proven for real.** Two humans, two real networks that **did not punch through** to each other directly,
**see each other move via the relay**. The judge is neutral: the server log shows the relayed traffic **both
ways**. *It is the first time the code brings two real humans into the same space over the Internet — a first
element of answer to the "empty fortress" (D27).*

**The bug only reality could show.** On the first attempt, the experience was **asymmetric**: A saw B, but B
did not see A. Cause: on receiving a *relayed* state, the code still opened a direct-connection "hole" → A
believed it was connected directly, never abandoned punching, and therefore never relayed back. **No lab test
could see it**: on a single machine, punching always succeeds, so the "relay" case never happens. Fix: only
open the direct hole if the packet does not come from the relay. *The value of a human in the wild,
demonstrated in black and white.*

**The relay also carries the world's objects.** Beyond the avatar, the **relay envelope was generalised** to
transport any already-signed packet (a shared object, for example) — without changing by a single byte the
already-proven avatar transport. Verified for real: through the NAT, two players share **a single object,
with a coherent master** on both sides.

**Measure the link rather than assume it (28 June).** To know whether these distant links are *really* alive,
we wrote a **measurement instrument**: an autonomous agent volunteers run, which records the freshness, loss,
jitter and reordering of the links it perceives, and sends back the figures. Two lessons came out of it:

- **An instructive false alarm.** The first readings shouted "89 % loss" on the relay path. Investigation: it
  was not the network, but **our own bandwidth thrift poorly measured** (the instrument compared the received
  low-frequency trickle to the full rate emitted). The instrument now knows how to **separate "not sent on
  purpose" from "sent then lost"**. Full story: [behind the scenes](coulisses.md).
- **The verdict, once the instrument was honest.** On real distant links (several countries, some on CGNAT),
  presence is **alive**: freshness **p95 ~200–335 ms** (< 500 ms), **real loss ~0**. Two hardenings drawn from
  reality stayed: a **silence tolerance** on the rendezvous side raised from **5 s to 20 s** (a CGNAT link
  re-registers constantly — at 5 s we were wrongly evicting it), and an **emission redundancy** that has since
  matured into **ADAPTIVE redundancy**: a node only duplicates if it has **measured** *random* loss with
  headroom (gain proven on the bench: `30 % → 9 %` at two copies), and **never** on a link that *saturates*
  (where duplicating aggravates). The detail of this investigation: [network work](chantier-reseau.md) and
  doubt **D36**.

## 4. Beyond (deferred, acknowledged)

- **Spatial proximity voice**: a peer-to-peer, spatialised voice chat that benefits from encryption and
  inclusivity (voice adapts to the link). A big piece, deliberately pushed back.
- **Portability across engines**: extract a portable network core for different 3D engines. Postponed — the
  current local bridge is enough to prove it (two engines already share the same space).

## What this work does not prove (honesty)

- The relay is, at this stage, **centralised** (the rendezvous point relays): an acknowledged fallback, to be
  **decentralised** later (which reopens the question of the incentive to relay, D4).
- It is proven **with two players**, not at scale.
- The measurement in the wild bears on the **substrate** (the presence transported, alive) — **not yet on the
  feel** of humans moving and playing together (D27 "the empty fortress" lightens, without closing).
- The generalisation of authority (registry, quorum, clock) is largely **planned**, not yet built.

---

*Technical facts kept up to date as measurements come in. The cost and throughput figures: [state
review](etat-du-projet.md).*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Robustness work** — a step of the **⚙️ The code** and **🧭 Understand everything** paths.

**Continue the thread:**
- ⚙️ *The code* · 🧭 *Understand everything* → **[How to run & test](TESTS.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
