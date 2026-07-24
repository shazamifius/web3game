# Launcher work — moving from one world to another

*[Version française](../chantier-launcher.md)*

> A peer-to-peer network cannot be seen. You can prove it transports a live presence between distant machines
> (done) without yet having anything that **looks like a platform**. Yet the project's distant promise — a
> universe of worlds you move between — needs, at some point, a **concrete gesture**: entering a world,
> crossing a portal, finding yourself in another. This page shows that first gesture, how it is orchestrated
> without ever leaving a black screen, **and what it does not prove yet**.
>
> See: [the measured state](etat-du-projet.md) · [behind the scenes](coulisses.md) (investigation #5) · [the register of doubts](doutes.md).

---

## 1. The problem: two worlds, no gap between them

Running two games separately is easy. The hard part is the **transition**: a player crossing a portal must
**never** see the void. No black screen, no dead window that lingers, no "both worlds running at once and
lagging". The switch must be **invisible** — otherwise the illusion of a continuous universe collapses at the
first crossing.

And like everything else in the project, it happens **with no central game engine to arbitrate**: it is a
small local application, the **launcher**, that holds the worlds and orchestrates the crossing.

## 2. Three pieces

| Piece | What it is |
|---|---|
| **The launcher** | A native application (Rust, hand-made). It launches the worlds, draws the interface, and plays the role of **régisseur** (stage manager) of the switch. It is the only conductor. |
| **Two Unreal worlds** | A **hub** (the crossroads, with a portal) and an **island** (the arrival world). Two distinct games, each wired to the network core through the *sidecar* bridge. |
| **A navigation protocol** | A small conversation between each world and the launcher (a local socket): "I'm ready", "show yourself", "go on standby", "close yourself". The launcher decides; the worlds obey. |

## 3. The switch, and its invariant

When the player crosses the portal, the hub tells the launcher. The launcher does not simply kill the hub and
launch the island — that would be the guaranteed black hole. It follows a **state machine** with a golden
rule, an invariant that is **never** violated:

> **We never close the old world until the new one has proven that it displays a frame.**

Concretely: the launcher prepares the island in the background while the hub stays visible; it waits for the
island to announce "I have rendered my first frame"; **only then** does it promote the island and put the hub
on standby. If the island cannot manage it (it crashes, it drags too long), the hub stays — we never end up in
front of nothing. It is the same philosophy as the rest of the project: **never break what works before the
replacement has proven itself.**

## 4. What has been proven

- **Smooth in real play.** The judge here is not a figure: it is the **eye**. Under real conditions, the
  switch was judged smooth — you cross the portal, you are on the island, with no perceived jerk.
- **The state machine holds under pressure.** Its logic was machine-gunned by a simulation: **200,000 steps**
  of random switch sequences, four invariants checked at every step (including "never two active worlds",
  "never close the old before the new one's frame"), **zero violation** — and we proved the net *bit* by
  deliberately reintroducing the fixed bugs.

But this second proof is also a **lesson in humility**, and it deserves to be told in full: those 200,000
green steps once validated a **false** guarantee, because the simulation rested on an erroneous hypothesis
about the real world (the executable of an Unreal package is a mere *bootstrapper* that launches the real
binary then exits — so the launcher was "killing" an already-dead process). Only a campaign on the **real**
machine showed it. The full story is in **[behind the scenes, investigation #5](coulisses.md)** — it tells,
better than anything else, how this project works.

## 5. What remains, honestly

- **The island is a dead-end.** You arrive there, but there is **no return portal yet**: the only exit is to
  close the window. A one-way trip is not a platform — that is the next gesture.
- **The content is sparse.** The worlds are demonstration sets, not places to inhabit. The question of *"do
  you want to stay?"* (doubt D27, cf. [alive work](chantier-vivant.md)) remains whole, and it will not be
  settled on the network side.
- **The switch is proven between two worlds, on one machine.** The crossing with several people (friends who
  meet while changing worlds together) is a notch above, not yet crossed.

In other words: the **mechanism** of the crossing holds and it is clean. What is missing is not the plumbing —
it is the **world at the end**. It is an acknowledged work in progress, not a disguised achievement.

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **launcher work** — the part of the project a player *sees*, where the invisible network
becomes a concrete gesture.

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
