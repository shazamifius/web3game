# Project state — an honest review (with figures)

*[Version française](../etat-du-projet.md)*

> A **review of the current state**: what has been built and verified, **with the measurements**; the battles
> fought; the walls hit; and — above all — the **open doubts**. Project rule: **a proof = a reproducible
> figure.** Each measurement below gives the command to replay it, and what it does **not** prove.
>
> This repository is **neither a product nor a promise**: it is an **exploration**. See also the
> [README](../../README.en.md) and the [code architecture](ARCHITECTURE.md). The commands: [TESTS.md](TESTS.md).
>
> *The figures below were measured on a test PC (desktop tower, 12 cores). They stand as **reproducible orders
> of magnitude**, not universal constants.*

---

## 1. The key figures (reproducible)

| Measurement | Value (test PC) | How to reproduce it |
|---|---|---|
| **Upstream** throughput per node (saturation, 32 neighbours) | **~34 KB/s** (max ~38) | `cargo run -- sim 50 3 15` |
| **Downstream** throughput per node | **~31 KB/s** | same |
| **CPU** per node | **~0.7 %** of one core | same |
| **RAM** (process peak) | **~38 MB** | same |
| **Crowd perception** at N = 1,000 | **~87 %**, at **flat inbound bandwidth** (~46 KB/s) | `cargo run -- coopsim-bus 1000` *(with `POW_BITS=8`)* |
| Perception at N = 2,000 / 5,000 | **52 % / 49 %** (limited by bootstrap, not cost) | `coopsim-bus 2000` / `coopsim-bus 5000` |
| **Latency** of the local bridge (sidecar Rust↔engine) | RTT **median 47 µs**, p95 67 µs | `cargo run -- sidecar` |
| Size of a **signed state packet** | **182 bytes** (118 payload + 64 of Ed25519 seal) | format, see [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Symmetric NAT traversal** via relay | established **both ways** (real networks) | `cargo run -- relay-test 6` *(deterministic bench)* |
| **Freshness** on **real remote links** (several countries, incl. CGNAT) | **p95 ~200–335 ms** (< 500 ms "alive" threshold), **real loss ~0** | measurement instrument (agent) + rendezvous log |
| **Link probe** (NAT type, RTT, jitter) | runs in the agent, reported continuously | `cargo run -- natcheck` |
| **Redundancy gain** on *random* loss (real bench) | 30 % loss → **9 %** (2 copies) / **2.8 %** (3 copies), ≈ `pᴷ` | `./tools/netem-bench.sh 30` |
| Automated **tests** | **363**, 0 warning | `cargo test` |
| Simulation **bench** ceiling | **~1,500 nodes** (1 OS thread / node, 12 cores) | hardware limit (see §4) |

> **What these figures do NOT prove** (method honesty): they are taken in **simulation / localhost** — the
> real network cost (interface card, physical RTT) is not counted; "perception" counts the **known** peers,
> not necessarily those **heard in time**; and **"55,000" is never measured directly**. So we will **never**
> say "55K proven".

## 2. How we extrapolate to 55,000 (the calculation, not a slogan)

A node's throughput is **bounded by its neighbourhood (~32 neighbours), not by the total N**: a player only
emits at full rate to its immediate neighbours, whatever the size of the crowd. At saturation, we **measure
~34 KB/s** upstream, i.e.:

```
34 KB/s ≈ 0.27 Mbit/s  (≈ 0.4 Mbit/s counting the unmeasured IP/UDP headers)
```

This figure **does not change** at 55,000 players: the area of interest bounds emission to the neighbourhood,
not to the total. Scaling therefore happens by **adding machines**, never by loading a single one.
**Acknowledged limit:** it is an **architectural argument**, measured up to ~1–2,000 nodes; beyond that it is
an **extrapolation**, and bootstrap (large-scale peer discovery) remains the ceiling to lift — hence the
52 % / 49 % at 2,000 / 5,000.

## 3. What has been built and verified

- **Identity = a key.** Every message signed (Ed25519); identity is the public key, carried in the packet —
  it self-proves, with no trust directory, and **persists** across sessions. *(unit tests + reload.)*
- **Real NAT traversal, down to the hard case.** Direct hole-punching; and when the NAT is too closed to be
  punched (*symmetric*), a **relay** takes over — established **both ways between two distinct networks on
  the Internet** (not in a lab).
- **The network characterises its links and adapts its redundancy.** Each node **probes** its link: NAT type
  (punchable cone vs symmetric, via STUN, hand-made), latency, jitter, and **nature of the loss** (random vs
  congestion, via a rising-rate burst). It deduces whether it is worth duplicating its sends: on a
  controlled *random*-loss bench, redundancy does **divide** the loss (**30 % → 9 % at 2 copies, 2.8 % at 3,
  ≈ `pᴷ`**); on a *saturated* link (congestion), it would **make it worse** — so it is not enabled.
  *(`natcheck`, `losscheck`, `phase1`, `tools/netem-bench.sh`.)*
- **Resistance to attacks.** Real adversarial programs (Sybil, eclipse, *framing*, gossip flood) are played
  against the network: the swarm holds, stolen states are rejected, cheaters are muted.
  *(`cargo run -- attack sybil-frame` · `attack gossip-flood`.)*
- **Crowd perception at bounded cost.** Sharp neighbourhood + distant crowd as aggregated summaries → **flat
  inbound bandwidth** (~46 KB/s), perception **~87 % at 1,000 nodes**. *(`coopsim-bus`.)*
- **3D-engine independence.** Two engines (Bevy and Unreal) brought together **in the same space** via a
  local bridge (latency **~47 µs**). *(proven on screen.)*
- **"Alive" presence.** Distant avatars smooth and inhabited (interpolation + procedural life), even under
  loss.
- **Measured in the wild, not only simulated.** A **measurement instrument** (an autonomous agent that
  volunteers run) recorded, on **real remote links** (several countries, some behind the hardest NAT), an
  **alive** presence: freshness **p95 ~200–335 ms** (below the 500 ms threshold), **real loss ~0**. *(First
  hard fact against the "empty fortress" — see §6.)*
- **A first world-to-world transition.** A **native launcher** (Rust) makes several Unreal worlds coexist:
  from a **hub**, you cross a portal and **switch** to an **island**. The régisseur never closes the old
  world until the new one has **proven a frame** (an invariant that held even under 200,000 simulated switch
  steps). Judged **smooth** in real play. *(Acknowledged limits, works in progress and not yet acquired: the
  island is still a **dead-end** — no return portal — and its content is sparse.)*

## 4. The battles fought (the method)

Strict loop: **compile → test → prove (a figure) → write.** We only mark "done" what is verified, and every
"done" lists what it does **not** do (a debt register).

- **A core attacked by ourselves**: instead of asserting "it is safe", we write the adversary and we measure.
  A negative result (a wall, a collapse) **counts as progress** if it is reproducible.
- **The judge is neutral**: for the real proofs (NAT, relay), it is the **rendezvous log** that decides, not
  enthusiasm — this is how we found bugs invisible in the lab (on a single machine punching always succeeds;
  only a human outside reveals the real behaviour).
- **Presentation was separated from the core** (the core became pure, agnostic), then reconnected by
  **stages, each proven before the next** (contract → measured latency → real core → real NAT).

## 5. The walls hit (the acknowledged limits)

- **The "55,000" scale is not directly measured**: ~1–2,000 nodes measured; beyond = extrapolation.
  Large-scale **bootstrap** is the remaining ceiling (52 % at 2,000, 49 % at 5,000).
- **"Serverless" keeps an asterisk**: bootstrap goes through a rendezvous (introductions only); fully
  decentralising it is open.
- **No end-to-end encryption**: positions in the clear (the signature guarantees authenticity, not secrecy).
- **The decisive test is *started*, not passed.** We have a **first measurement in the wild**: **real remote
  nodes** (several countries, incl. CGNAT) are **alive** (freshness p95 < 500 ms, real loss ~0). It is a hard
  fact — the infrastructure is no longer *empty*. But it measures the **substrate** (the presence
  transported), not yet the **feel**: humans who **move and play together** and **sense** it alive. That
  test, the most important one, is still ahead.

## 6. The open doubts — the heart of the approach

> Here, **a doubt is not a weakness to hide: it is the object of the work.** The complete, tracked register:
> **[the doubts](doutes.md)**.

- **Inclusivity**: a weak link *receives* too much in a dense crowd (the area of interest bounds emission,
  not reception). How do we guarantee a decent experience from the weakest to the fastest?
- **Perception by relevance, not by proximity**: "the 32 closest" ≠ "the 32 that matter" (those you talk to,
  who handle the object you are looking at). An architecture problem, not a tuning one.
- **Persistence without a server**: where does a player's state live between two sessions, and who prevents
  forging it?
- **Distribution / player arrival**: install, cross the routers, meet up — a wall of real-world usage.
- **The "empty fortress"**: have we built a fine infrastructure in which two humans have never *really* met,
  in motion, over the real Internet? **First measured answer:** real remote nodes are **alive** there (p95
  < 500 ms, real loss ~0) — the infrastructure is no longer *empty*. But the **feel** (humans moving and
  playing together) is not proven yet: the doubt lightens, it does not close.

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **The project's measured state** — a step of the **🌱 Discover**, **🔎 Judge fast** and **🧭
Understand everything** paths.

**Continue the thread:**
- 🌱 *Discover* · 🔎 *Judge fast* · 🧭 *Understand everything* → **[The register of doubts](doutes.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
