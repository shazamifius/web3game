# ARCHITECTURE — the code layout & the long-term target

*[Version française](../ARCHITECTURE.md)*

> File layout (`src/`), the common packet header, and the target "Own + Shields" architecture (authority per
> object, BFT, relays).
> *The idea in plain words, before the code: [serverless, in plain words](comprendre-le-p2p.md).*

## The whole project — six parts

Before diving into `src/`, the overall shape. The project is not one block: it is **~53,000 lines of
hand-written code** (the rest of the figures one sometimes sees — over a million lines — is **generated 3D
geometry** written by a script, not code), spread across six repositories with a clear role.

| Part | Language | ~Lines | Role |
|---|---|---|---|
| **The network core** | Rust | ~29,000 | The real R&D work: identity, NAT, perceived crowd, measurement. This is what the rest of this page describes. |
| **The launcher** | Rust | ~13,000 | The native application: launches the worlds, draws the interface, orchestrates the [switch](chantier-launcher.md). |
| **The hub** | C++ | ~2,900 | The crossroads world in Unreal, with the portal to the island. |
| **The island** | C++ (+Python) | ~2,800 | The Unreal game world; the Python scripts **generate** the terrain (hence the million lines of `.obj`). |
| **The showcase** | Markdown | — | The public presentation and this documentation. The only part open to all. |
| **The notes** | Markdown | — | The private logbook (session recaps, audits). Never shared. |

The **core** and the **launcher** talk through a local socket (the *sidecar* bridge); the two Unreal worlds
are clients of that core. The rest of this page zooms into the **core** (`src/`).

## Code layout (`src/`)

Principle: **one file = one responsibility** (many small files rather than one big one).

```
src/
├── main.rs              entry point, routing of the headless modes (rendezvous/sidecar/bot/agent/sim/…)
├── math.rs              in-house Vec3 (no 3D engine) — the core's maths brick
├── dsp/                 THE VOICE: signal-processing benches, hand-made, std-only (work paused)
│   ├── fft.rs           hand-made radix-2 FFT/STFT — the common spectral base
│   ├── psycho.rs        psychoacoustic model (Bark bands + masking) — perceived quality
│   ├── codec.rs         transform-domain codec + perceptual allocation (keeps singing/whisper/beatbox)
│   ├── denoise.rs       "mic study": denoising by spectral subtraction, user-controlled
│   ├── separate.rs      source separation: enumerate/isolate/remove each noise, as chosen
│   ├── stoi.rs          STOI intelligibility (white-box) — "is the voice understood?"
│   ├── chain.rs         the end-to-end chain (denoise → codec → transport → verdict)
│   ├── optim.rs         STOI-guided codec optimisation (find the bitrate/quality "knee")
│   ├── adaptive.rs      adaptive controller: re-tunes bitrate/buffer/denoise per observed link
│   └── spectro.rs       hand-made PNG spectrogram — "eyes" to judge a sound without ears
└── net/                 THE NETWORK, hand-made (engine-agnostic, no 3D engine)
    ├── mod.rs           assembles the module and exposes the public API
    ├── wire.rs          a packet's TYPE (1st byte) + protocol version + rendezvous port
    ├── message.rs       a packet's format (PlayerState, encode/decode + signed, state batches)
    ├── control.rs       the directory messages (HELLO / WELCOME)
    ├── crypto.rs        Ed25519 + PeerId (identity = key) + proof of work — the only "black box"
    ├── rendezvous.rs    the RENDEZVOUS POINT: introduces players then steps aside (+ relay for hard NATs)
    ├── transport.rs     the raw UDP socket — the "connection"
    ├── punch.rs         hole punching: wire boundary (encode/decode/abandon of the punch)
    ├── linkprobe.rs     the LINK PROBE: NAT type (STUN), latency, jitter, nature of loss (Phase 2)
    ├── linkstats.rs     an observed link's stats: REAL vs apparent loss, jitter, freshness
    ├── gossip.rs        decentralised discovery: "business cards" between peers, no directory (D22)
    ├── cell.rs          cell SUMMARY: perceive a distant crowd without N streams (D22)
    ├── aoi.rs           Area of Interest (water-filling: who receives what rate)
    ├── aoi_bench.rs     "social relevance vs proximity" bench (cargo run -- aoi)
    ├── aoi_e2e.rs       the same, end-to-end over the real transport (cargo run -- aoi-live)
    ├── anticheat.rs     the "local Shield": physical plausibility rules (ch. 6.3+)
    ├── accuse.rs        signed accusations + quorum: shared reputation (ch. 6.7)
    ├── orb.rs           the shared orb: PURE authority logic (ORB+OWN, signed encode/decode, migration)
    ├── stars.rs         the shared star field (demonstration world)
    ├── skin.rs          the random skin colour (carried in the state packet)
    ├── link.rs          NetLink: a node's network state (peer table, reputation, cells…)
    ├── bot.rs           the HEADLESS CLIENT (cargo run -- bot …) + reusable `Bot` brick
    ├── sidecar.rs       THE BRIDGE to Unreal (cargo run -- sidecar): local socket to the engine client
    ├── metrics.rs       the distributed MEASUREMENT AGENT (cargo run -- agent/stats): measures links in the wild and reports the figures
    ├── session_window.rs the session's visible window: the volunteer SEES what the agent measures
    ├── probe.rs         system probe (node CPU/RAM, via /proc) — quantify a node's real cost
    ├── liveness.rs      "is it alive?" bench: fidelity / freshness / jerk (cargo run -- vivant)
    ├── voice_bench.rs   voice TRANSPORT bench (mouth-to-ear latency, jitter buffer)
    ├── sim.rs           the MASSIVE SIMULATION (cargo run -- sim N M T): N nodes + M attackers
    ├── coopsim.rs       crowd benches in a cooperative thread / memory bus (coopsim, coopsim-bus)
    ├── lossbench.rs     DETERMINISTIC redundancy bench under known loss (cargo run -- phase1)
    ├── netembench.rs    REAL redundancy bench on a degraded link (cargo run -- netem-bench)
    ├── attack.rs        the ATTACKING PROGRAM (cargo run -- attack …) — ch. 5 & 6
    ├── natdemo.rs       the nat-test text mode (hole punching without 3D, for netns)
    └── demo.rs          the net-demo text mode (observe the packets)
```

> Two **executable components** deserve a word, because they are at the heart of the "in the wild" proofs: the
> **rendezvous** (`rendezvous.rs`) — the minimal directory that introduces players, then relays for NATs too
> closed — and the **measurement agent** (`metrics.rs`) — an autonomous instrument a volunteer runs at home,
> which joins the network, **probes its link** (`linkprobe.rs`) and **reports honest figures** on the
> liveliness of the distant links.

> The **latency catch-up** (interpolation, prediction, damped spring) that lived in `net/netcode/` on the
> Bevy client side has been **removed**: it is now Unreal that interpolates the distant avatars from the
> received velocity (via the *sidecar* bridge).

**Header common to ALL packets**: byte 0 = `type` (KIND), byte 1 = `protocol version` (`PROTO_VERSION`). A
receiver of another version rejects the packet **and signals it** instead of reading it wrong — no more
"invisible person" of two desynchronised binaries. See `net/wire.rs`.

Since chapter 5, every state packet is **signed**: we emit the body followed by a **64-byte Ed25519 seal**.
Since **chapter 6.1**, the identity (`id`) is no longer a `u8` number but the sender's **public key** (32
bytes), **carried in the packet**: the receiver verifies the seal AGAINST that embedded key — identity
self-proves, **with no trust directory at all**. The rendezvous can therefore no longer lie about "who is
who".

A player packet is **118 bytes**: `type` (1) + `version` (1) + `id` (**key, 32**) + `x,y,z` + `vx,vy,vz` +
`yaw,pitch` + `r,g,b` (11 × 4 bytes) + `parent` (**key, 32**; zeros = autonomous) + `seq` (8, anti-replay).
Signed = 118 + 64 = **182 bytes**. See `net/message.rs`. A **signed** orb packet is **136 bytes** (body 72 +
seal 64). The **72-byte** body: `type` + `version` + `owner` (**key, 32**) + `orb version` + position,
velocity, colour. See `net/orb.rs`. (`PeerId` = the key, in `net/crypto.rs`; shown in short hex.)

**"Inactive file" convention**: a file no longer in use is prefixed with a `_` (e.g. `_demo.rs`) and its
`mod` line is removed. It rises to the top of the list and signals at a glance that it is no longer used —
without tucking it into a subfolder. (The Rust compiler confirms the opposite: if a file *is* wired in with
no `unused` warning, then it is used.)

---


## The target architecture (long-term vision)

The end goal, formalised in our words:

**"Own + Shields"** — no central server, every player is a node.
- **Own** (authority): **arbitrates** the state of a contested object or zone. If the Own cheats or crashes,
  we replace it (migration, already done on the orb).
- **Shields**: recompute/verify the Own and ban it if it cheats. Quorum **BFT 3f+1** (1 Own + 3 Shields
  tolerates 1 traitor), like PBFT.

> **Major refinement (settled while coding): authority is PER OBJECT, not global.**
> A single Own relaying *everything* for the whole instance would become the upload bottleneck we want to
> avoid (one PC cannot hold thousands of streams). So:
> - **What is yours** (your position, your voice) → **no Own**: you are your own authority, you broadcast
>   **directly** to your ~10 neighbours (no conflict possible).
> - **What is shared/contested** (the orb, a door, a score) → **one Own per object/zone**, at **low rate**
>   (an event now and then). Thousands of small Owns, never a single one. 55,000 players = thousands of
>   zones of ~10.
>
> **Own ≠ Relay.** The **Own** *decides* (authority, conflicts). The **Relay/parent** *copies* bytes for a
> player with low upload (transport, zero decision). One good PC can wear both hats, but they are two
> separate roles — and the relay must never be able to **alter** what it transports (the weak player will
> sign their messages: sealed envelope, chapter 5).

**Choice of Own**: best hardware + best reputation + at the players' geographic centre (minimal latency).

**Sponsorship / supernodes**: a player with low upstream sends their data once to a close, reliable relay,
which redistributes it.

**Fallback**: if no reliable node is available, switch to a personal server. Same for the *signaling*
(STUN/TURN) that helps punch through NATs.

### Truths to keep in mind (corrections already settled)
- **"More players = more stable" → false.** In naive P2P, everyone talks to everyone: O(N²). The solution is
  **AoI** (chapter 3), not brute force.
- **"Blockchain solves latency" → false.** A consensus is *slow* (seconds). We would reserve it for
  **reputation**, never for real-time sync (which requires < 50 ms).
- **"You can remove every server" → almost.** The routers' **NAT** blocks incoming connections; you need a
  small *signaling* server to bootstrap the direct connections (hole-punching). The game itself stays 100 %
  P2P: once the introductions are done, you can **kill the rendezvous**, the game goes on.
- **"Cutting to 4 sends per player solves scaling" → false.** The O(N²) work does not disappear, it **moves**
  onto the node that redistributes (which would explode at 22 Gbps for 55,000 players). The bottleneck is
  **always** the upload of whoever re-broadcasts. The real answer: **AoI** (you only talk to your ~10–100
  neighbours, **regardless of N**) + **local** Owns/relays, never a hub.

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Architecture & code** — a step of the **⚙️ The code** and **🧭 Understand everything** paths.

**Continue the thread:**
- ⚙️ *The code* · 🧭 *Understand everything* → **[Network work](chantier-reseau.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)

---
