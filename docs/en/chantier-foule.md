# Dense-crowd & inclusivity work — see the crowd with no ceiling, without blowing up bandwidth

*[Version française](../chantier-foule.md)*

> The biggest piece of architecture: making it so that in the middle of a crowd of 200, 500, 5,000, **everyone
> perceives the whole crowd** (close ones sharp, distant ones degraded) **with no hard ceiling** and **without
> blowing up their bandwidth**. We show here, without hiding them, **the defects that measurement revealed**
> and **a decision a measurement imposed against the starting intuition** — that is the heart of the approach.
>
> See: [the doubts addressed](doutes.md) (especially D22, D3, D29, D30) · [the measured state](etat-du-projet.md) · [the idea in plain words](comprendre-le-p2p.md).

---

## The problem (D22): blind beyond 32

The core's "free" scaling bought its constant cost with a **hard ceiling**: the rendezvous point only
introduced the **32 closest**, and clients exchanged nothing between themselves. So the **33rd neighbour did
not exist and could never exist**. In a crowd of 200, everyone was **blind to 168**. Yet, in a social space,
**seeing the crowd is the game**.

**The guiding idea — separate two things the ceiling was confusing:**
- **focus** = the peers I hold a full-rate link with (~20 Hz, prediction, detailed avatar). This **must** stay
  bounded (~8 to 32).
- **awareness** = everyone I perceive at low fidelity (hundreds, as silhouettes). This must **not** be capped.

**The invariant to hold (the trap never to hide).** The reception cost must stay **independent of the crowd's
size**. The proof of success is therefore not "coverage rises", but **"coverage rises AND inbound bandwidth
stays flat as the crowd grows"**. (Raising the ceiling would be cheating: it would move the wall, not break
it.)

## Stage 1 — Decentralised discovery: the 33rd becomes learnable

The rendezvous point stops being the authoritative enumerator and goes back to being a simple **bootstrap**.
Then each peer announces at low rate, to its neighbours, **a few other peers it knows** ("business cards") — a
word-of-mouth discovery, with no enumerating server, no vision ceiling.

**Measured:** in a crowd of 200, coverage goes from **16 % to 98 %** (everyone ends up learning all 200) — and
**inbound bandwidth does not grow** from 200 to 500 (it stays flat, even drops). The 32 ceiling was indeed
arbitrary; discovery was the only wall, and it falls without reopening the traffic explosion.

**The honest price of gossip (D23).** This word-of-mouth opened a door: we were learning unproven "cards" → an
attacker could pollute the tables or **reflect a flood toward a victim**. We closed it **before** building on
top (rule: no concrete on sand), with four defences: proof of work required on each card, a ban on
overwriting a known peer's address, abandonment of uncorroborated punching after ~10 s, and per-source
limiting. **Proven by a real attacker** (`gossip-flood`): **0 reflected punching** received by the target,
even when the attacker mines new identities relentlessly (the burst is bounded to ~10 s, measured).

## Stage 2 — Two sharp/blurry thirds, and a defect revealed by measurement

In a dense crowd, everyone is at ~the same distance → relevance by distance no longer discriminates →
**everyone becomes equally blurry**. We had to make **sharp close ones emerge**: reserve most of the budget
for a **focus** of a few peers (full rate), and only sprinkle crumbs on **awareness** (the rest, at low
fidelity).

**A metric did its job — it exposed a defect.** By measuring the peers actually *heard* (not only *known*), we
saw that in a crowd of 160, focus collapsed to **0.2** (instead of ~8) — and it did not recover over time: **a
real defect, not an artefact**. Cause: focus was **"churning"** — recomputed at every instant, the set of "8
most relevant" kept changing → no sustained full-rate link. The fix: a **sticky focus** (with hysteresis — we
only replace a member if another is clearly more relevant).

**Result — the invariant held, measured.** When the crowd doubles (80 → 160): **focus stays bounded** (~9),
**awareness grows with the crowd** (68 → 134, the LOD of the whole crowd), and **inbound bandwidth stays flat**
(~44 → ~40 KB/s). That is exactly the proof sought: coverage rises without bandwidth exploding.

**Confirmed on screen.** The two-thirds rendering (detailed close ones + a crowd of cheap impostors) was
verified on a real crowd (80 windows open at once): **far more than 64 visible silhouettes, with no
performance drop** (D24).

## Stage 3 — Making the distant crowd hold, and a decision imposed by measurement

At very large scale, even at flat bandwidth, a distant peer's **freshness** collapses (one update per minute):
the distant crowd becomes a frozen mush. The idea: replace N distant individual streams with **a few summary
streams** per region ("cell").

**Measurement decided against the starting design.** The first design gave each cell an **elected leader** (the
smallest known identifier in the cell), who produced the summary. Measured on the bench: it was **the dominant
wall**. Since each node knows a different subset of the cell, nodes did not agree on "who is the leader" →
they **rejected otherwise legitimate summaries**. Perception collapsed from **91 % to 10 %** as the crowd
grew.

**The mistake of elegance, and its fix.** We had borrowed from an object's authority (where a master *must*
settle the physics) a model of a **single elected leader**. But **perceiving a crowd is not an act of
authority — it is an observation.** There is nothing to settle. So we **removed the need for a leader**: a
summary is no longer "a leader's word", it is a **bundle of proofs** — a sample of **signed positions**
(self-certifying, like a player's state). We no longer verify *who* sends, we verify **the signatures inside**,
and we **union** the verified samples received from **several independent sources**.

This gesture **dissolves** the attacks by construction: inventing ghosts is impossible (each carries its
signature), and **hiding people is bounded** (unioning proofs is *monotone* — a liar can omit, never subtract
from what the others have seen).

**Measured result.** Once the leader was removed, density is restored: **~87 % at 1,000 nodes, at flat inbound
bandwidth**. Beyond (2,000, 5,000), perception is limited not by this mechanism but by a **second, distinct
wall**: the **slowness of discovery** at bootstrap (49 % at 5,000, and still rising at the end of the
measurement).

**Density, secured without reopening the wall.** A cell's "count" could be inflated by a liar. The counter
reuses the anti-eclipse principle: we keep **one count per distinct subnet** and retain a **high quantile**
(the 3rd largest) → inflating requires **diverse IP addresses** (a scarce resource), not free computation; and
omitting never lowers this quantile. (At this stage, banked: we only revisit it when testing on real IP
addresses.)

## What these results do not prove (method honesty)

- **Coverage** counts *known* peers, not always *heard in time* — it is optimistic.
- The **felt freshness** of a distant peer has not been quantified live.
- **"55,000" is never measured directly**: density measured at ~1,000–2,000, extrapolated beyond.
- **Anti-inflation per subnet** is only proven in logic and simulation; the **real diverse IP addresses** (and
  the "botnet" residue) remain to be tested.

## What remains — inclusivity (next phase)

Now that the crowd is visible, the next front is to **exclude no one**: **bilateral** area of interest (the
receiver announces a budget the senders respect, D3), **graceful degradation** (thin out the distant ones
before the close ones), a **parent that aggregates** for the very weak, and **"black hole" relay detection**
(D4/D5). Add to that the redesign **by social relevance** rather than mere proximity (D29) and continuous
**adaptive detail** (D30).

---

*Technical facts kept up to date as measurements come in. The cost and throughput figures: [state
review](etat-du-projet.md).*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Dense-crowd work** — a step of the **⚙️ The code** and **🧭 Understand everything** paths.

**Continue the thread:**
- ⚙️ *The code* · 🧭 *Understand everything* → **[Robustness work](chantier-robustesse.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
