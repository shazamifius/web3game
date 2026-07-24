# Behind the scenes — our problems, and how we solved them

*[Version française](../coulisses.md)*

> **This document is a bit unclassifiable**, and that is on purpose. It is neither a manual nor a feature
> list. It is a **logbook**: the walls we hit, the false leads we followed, and **what we went through to
> understand what was really going on**.
>
> Why keep it? Because on this project, the rule is simple: **a proof = a reproducible figure, and a doubt is
> never hidden.** The best advances did not come from code we wrote, but from **honest questions** ("what if
> we were wrong?") and **measurements that corrected the plan**. This notebook tells those moments. It will
> grow as the investigations go.

---

## Investigation #1 — "the relay loses 89 % of the packets" (28 June 2026)

### The symptom
Our measurement instrument (an agent deployed on real machines, behind real networks) reported an alarming
figure: on the **relay path** — the one taken by a machine behind a locked-down network ("CGNAT") that cannot
connect directly — we measured **~89 % packet loss**. On screen, the distant avatars seemed to stutter,
almost dead. The project's founding doubt ("what if we had built a fine infrastructure in which two humans
never *really* meet?") was poking its nose out.

### The false leads
We first suspected the relay's **anti-abuse cap**: maybe it was throttling the sender. Check in the code: the
relay allows **30 packets/second**, the sender only produces **20**. Enough margin. **Hypothesis dismissed —
by measurement, not intuition.** We also tried a "brute force" fix: send each position in **triple**
(redundancy). Measured for real: loss fell from 89 % to 68 %, exactly as the calculation predicts
(`0.89³ ≈ 0.68`). It "worked"… but it still did not tell us **why** we were losing so much.

### The investigation
Rather than pile on band-aids, we went back to reading the code — the **neutral judge**. Three questions: how
does the instrument compute "loss"? Is the message serial number global or per-recipient? Does the sender
actually throttle what it sends to distant machines?

### The revelation
The code decided, and the verdict was unexpected: **there was almost no loss.**

Our presence engine spreads its attention: it sends **full rate (20 times/second)** to the ~8 most relevant
people around you, and a simple **trickle (2 times/second)** to everyone else — deliberately, to save
bandwidth. That is the heart of our "bounded-cost crowd" approach.

But our instrument was comparing what the distant machine received (**2/second**) with the sender's **global
counter (20/second)**. It saw 2 messages out of 20 arrive and concluded "90 % lost" — while the other 18 **had
never been sent, by design**. The "89 % loss" was not a network failure: it was **our own bandwidth thrift,
poorly measured**. The relay was not broken. The CGNAT link might be perfectly fine.

### The fix (elegant, not brute)
We taught the instrument to **separate two things it was confusing**: "not sent on purpose" and "sent then
lost". It now **infers the real cadence** (the effective rhythm of received messages) and measures loss
**relative to that cadence**: a normal interval = healthy; a "doubled" interval = a genuinely lost message.
Two distinct figures now come up — the **apparent** loss (vs full rate) and the **real** loss. A deterministic
test proves it: a throttled stream **with no loss at all** does show `loss ≈ 88 %` *apparent* but `real loss
= 0`.

### What it taught us (the proof redrew the plan)
1. **The real subject is not loss, it is relevance.** If a person you **interact** with ends up classed as
   "distant", they receive the trickle (2/s) and appear dead. The right fix is therefore not to send harder,
   but to **give full rate to whoever really matters**, even far away. That is our next work item.
2. **For the "trickle" (2/s), we will not send frozen positions but *trajectories*** — a compact description
   of the motion, replayed in a lively way at the others' end. A few messages per second are enough to
   describe a curve, and it is **naturally loss-resistant**.
3. **A brute-force fix can mask the real question.** The redundancy "worked" in figures, but repaired a false
   problem. We only build complex machinery **when a measurement demands it** — never "just in case".

### The follow-up — verified in the wild (28 June 2026)
The notebook said "to be continued": here it is. We **re-measured in the wild**, with the corrected
instrument, on **real remote links** (volunteers across several countries, some behind the hardest NAT,
CGNAT). The neutral verdict of the server log: **alive** presence, freshness **p95 ~200–335 ms** (below our
500 ms threshold), **real loss ~0**, verdict "alive". In other words: once the instrument was honest, **the
CGNAT link was fine** — it was indeed our measurement that lied, not the network.

Two hardenings drawn from the investigation stayed, because they serve **a real case, not a false one**: a
**silence tolerance** on the rendezvous side raised from **5 s to 20 s** (a CGNAT link re-registers
constantly — at 5 s we were wrongly evicting it), and an *optional* **emission redundancy** (sending a state
twice via the relay), kept **on hand** for proven cases — not turned on "just in case".

*Honest status: the mechanism is proven by the code, a deterministic test **and** a measurement in the wild.
What remains open is no longer loss, but **relevance** (giving full rate to whoever matters, even far) — the
next work item. And it measures the **substrate** (the presence transported), not yet the **feel** of humans
playing together (D27, "the empty fortress", lightens without closing).*

---

## Investigation #2 — "some distant links are *dead*" (28 June 2026)

### The symptom
The corrected instrument was running in the wild, and most distant links were **alive**. But some came back
with a blunt verdict: **`DEAD (>500 ms)`**, freshness p95 ~950 ms. The doubt pointed back: could we, here,
have real links that do not get through?

### The nice hypothesis (the "season 2" of investigation #1)
The reflex, after #1: *it is again our own bandwidth thrift, poorly measured.* Our engine sends **full rate**
to a small circle (focus) and a **low-frequency trickle** (~2/second, "awareness" tier) to everyone else. A
peer in that trickle is **fresh to ~500 ms by design** — the flat "> 500 ms = dead" threshold would wrongly
condemn it, exactly like the "89 % lost" of #1. Elegant… and we corrected the verdict in that direction.

### The twist (measurement, again, corrected the story)
Except the data said otherwise. The "dead" windows did not have a low cadence (a 2/s trickle) — they had
**zero reception**. The same link alternated, on a ~7-second cycle, between **full rate (fresh ~200 ms)** and
**total silence (~950 ms)**. Not "low fidelity by design": **bimodal — all or nothing.** The "awareness"
trickle simply did **not** reach these out-of-focus peers.

### What we fixed — and what we, honestly, only *revealed*
We made the verdict **aware of the cadence AND of reception**, in three states instead of two: **alive** ·
**distant (low fidelity)** — throttled on purpose, so *alive* even beyond 500 ms · **dead (silent)** — known
but **zero packet received** this window. And the instrument **now always displays reception** (`recv:0` =
silence made visible), so we no longer confuse "deliberately discreet" and "truly mute".

But let us be clear: **it fixed nothing at the root.** It made the instrument *honest*, and so the real wall
appears in full light: **why does an out-of-focus peer go completely silent**, instead of staying alive at
low fidelity via the trickle? It is a question of **area-of-interest inclusivity** (the weak, the distant,
must stay *perceived*) — the next deep work item, on the core side. *Like #1: the best advance is not the code
written, it is the question made visible. To be continued.*

---

## Investigation #3 — "4G was not the wall we thought, and redundancy is not free" (29 June 2026)

### The symptom
For a link too closed to connect directly, we go through a **relay**, and a relay **loses packets**. A
common-sense idea to compensate: send each position **twice**. The theory is even reassuring — if loss is
random with probability `p`, two copies are both lost only with probability `p²`, three with `p³`, etc.
Except that on trial over a **real mobile link**, redundancy did **not** help: it **worsened** the loss. The
calculation said "better", reality said "worse". Something was missing.

### The two starting beliefs
We carried, without having checked them, two comfortable hypotheses: **(1)** "a 4G/5G mobile link is
*symmetric* NAT — the hard case, to relay by default"; **(2)** "duplicating always helps, it is just a matter
of how many copies". Two reasonable beliefs… and both false, as the measurement would show.

### The twist (measurement, twice, corrected the story)
We built a **link probe** that runs inside the instrument, with no external dependency. It does two things.
First, it determines the **NAT type** by querying two public servers from a single socket (same public
address seen from both sides = *punchable* NAT; different address = *symmetric*). Verdict: the consumer phone
tested was **directly punchable** — not symmetric at all. **Belief #1: refuted.**

Then the probe **characterises the loss**: it sends a short **rising-rate** burst and watches how the link
reacts. On fibre, everything stays flat (~28 ms, ~0 loss): a *healthy* link. On the mobile link, latency
**swells with the rate** — from ~60 ms to over **100 ms** — then loss appears at the highest rate. That is
not random noise: it is **congestion** (the link saturates, its buffers overflow). And then it all becomes
clear: when loss comes from saturation, **two copies = twice the traffic = even more saturation**. The `pᴷ`
formula assumes *independent* losses; on a congested link, they are not. **Belief #2: refuted.**

### The fix — measure, then only help where it helps
The answer is not "more copies" nor "never copies", but **adaptive**: each node reads its own probe and
**only duplicates if it has measured random loss with headroom**. We saw it happen under real conditions — a
congested mobile link, **on its own**, chose **not** to duplicate (not to worsen what saturates). It remained
to prove the other half: that on *genuine* **random** loss, duplicating does help. As none of our test links
had that profile, we injected a known random loss with the kernel tool (`tc netem`), in a disposable network
namespace (without touching the machine), and ran the **real** mechanism through it. Result: loss **30 % →
9 %** with two copies, **2.8 %** with three — exactly the `pᴷ` curve. And on *correlated* loss (in bursts),
the gain collapses, as expected. The loop is closed: we know **when** redundancy pays, and we only turn it on
there.

### What it taught us
1. **Measurement beats intuition — again.** Two common-sense beliefs ("4G = symmetric", "duplicating always
   helps"), twice corrected by an instrument we took the trouble to build. It has become the signature of
   this project.
2. **A network that knows itself.** Before acting on a link, we **measure** it: NAT type, latency, jitter,
   nature of loss. A blind strategy helps at random; a strategy that measures first helps **right**.
3. **Redundancy is a targeted tool, not a magic wand.** It saves a random-loss link and harms a saturated
   one: the same action, two opposite effects depending on the terrain. Hence the importance of **diagnosing
   before treating** — and the doubt **D36** (the real diversity of connections) stays wide open.

---

## Investigation #4 — "half of these 'dead' links were alive" (29 June 2026)

### The symptom
The instrument was running in the wild, and a stubborn share of distant links came back with the verdict
**`DEAD (>500 ms)`**: their last news was too old for the project's "alive" threshold. After investigations #1
and #2, we had learned suspicion — but this time the verdict seemed solid: 500 ms is 500 ms.

### The false obviousness
A freshness threshold judges a **symptom** (the age of the last news), not a **cause**. Yet two very
different realities produce the same symptom: a **genuinely broken** link (that loses most of its packets),
and a **perfectly healthy but deliberately little-refreshed** link (our own bandwidth thrift sends a mere
low-frequency "trickle" to distant peers — a choice, not a failure). The flat verdict condemned both with the
same word.

### The investigation — replay, rather than re-measure
We corrected the verdict so it reads **the real loss** (the measure that distinguishes "not sent on purpose"
from "sent then lost", inherited from investigation #1): a link that is old but faithfully delivers what it
promises is *distant* — alive at low fidelity —, not *dead*. And to prove it, rather than launch a new live
session (whose every comparison suffers from confounding factors: a different crowd, a different moment — a
lesson already paid for), we **replayed the new verdict offline on 685 real links already recorded**: same
data, only the judgement code changes. Zero confounding variable.

### The revelation
**52 % of the "dead" (66 links out of 127) were alive** — healthy links, simply throttled on purpose. And the
two populations separate **with no overlap**: the rehabilitated ones have a median real loss of **0 %** (all
between 0 and 20), the truly dead ones a median of **60 %** (all between 21 and 90). The 20 % threshold does
not cut a continuous distribution in two: it falls into the **void between two clouds** — it separates two
distinct physical realities. *(The exact formula of the three-state verdict: [the measurements, in
equations](les-mesures.md).)*

### What it taught us
1. **A verdict must read the cause, not the symptom.** Freshness alone mixed "broken" and "discreet by
   design"; real loss separates them cleanly.
2. **Replaying on constant data beats re-measuring.** A new session would have compared two different
   moments; the replay compares two *judgements* on the same reality — the cleanest proof we have produced.
3. **Third time the liar was the instrument, not the network** (after the "89 % loss" and the "silent ones"
   of investigation #2). It has become a reflex: before believing an alarming figure, we put the measurement
   itself on trial.

---

## Investigation #5 — "200,000 simulation steps validated a false guarantee" (23 July 2026)

### The symptom
A native launcher makes it possible to move from one Unreal world to another (from a **hub** to an
**island**). In use, the switch was **"extremely random"**: sometimes perfect, sometimes **ghost windows**
impossible to close other than through the task manager. Unacceptable for a component meant to be beyond
reproach.

### The false assurance
After a first fix, we wrote a **fuzz of the switch state machine**: 4,000 random sequences × 50 steps, i.e.
**200,000 steps**, with four invariants checked at *every* step ("never two active worlds at once", "never
close the old one before the new one has proven a frame"…). Result: **0 violation**. We even proved the net
*bit* by deliberately reintroducing the fixed bugs — it fell within a few steps. On that basis, we announced
**"zero ghost window, guaranteed"**.

### The investigation — the real thing, not the simulation
The guarantee was **false**, and only a campaign on the **real** machine showed it, in two log lines: `LAUNCH
hub pid=3420`, then `REGISTER hub pid=3500` — **two different PIDs for the same world**. The fuzz assumed the
process we launched was the game. But the executable of an Unreal package is not the game: it is a **0.2 MB
bootstrapper** that launches the real binary (~324 MB)… **then exits**. The launcher was therefore "killing"
an already-dead process, while the real game survived — the ghost window.

### The revelation
A massive, rigorous test only validates **the model it was given**. Its 200,000 cases do not compensate for a
**false hypothesis about the real world** ("the launched process is the game"): they **disguise it as a
proof**. It is the **inverse** trap of the previous investigations — there, the instrument *refused* to
confirm what we expected, and it was right; here, it confirmed **too well**. The fix was to **own the real
binary** (the launcher keeps the handle of the real process, not the bootstrapper's); the switch is since
judged **smooth** in real play.

### What it taught us
1. **Before drawing a guarantee from a simulation, state out loud the hypothesis about the real world it
   encodes** — and ask who *verified* it. If no one: it is not a guarantee, it is a well-tested conjecture.
2. **A fuzz proves internal consistency; only a measurement on the real system proves correspondence to
   reality.** Both are necessary; neither replaces the other.
3. **Fourth reminder of the same rule**: the reliable judge is never enthusiasm, it is the log of the real
   system.

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Behind the scenes** — a step of the **🔎 Judge fast** and **🧭 Understand everything** paths.

**Continue the thread:**
- 🧭 *Understand everything* → **[The development journal](journal.md)**
- 🔎 *Judge fast* → ✓ you are at the end of this path. And next? ⚙️ [the code](ARCHITECTURE.md) or 🧭 [understand everything](README.md).

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
