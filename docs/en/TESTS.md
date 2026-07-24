# HOW TO RUN & TEST

*[Version française](../TESTS.md)*

> Run the headless core (rendezvous/sidecar/bot/sim), test a real bad network (`tc netem`), test NATs.

## Test under real network conditions, on a single machine

**A single machine is enough** to confront the network with reality: Linux can simulate a bad connection.

- **`tc netem`** (on the loopback interface `lo`) adds **latency, jitter, loss, reordering** to ALL localhost
  traffic. We run `sim` behind it, and the hundreds of nodes suddenly talk "as if over the Internet".
  (`tools/sim-netem.sh` applies it then removes it cleanly.)
- **`tc tbf`** limits the throughput (to simulate the "a few KB/s").
- **`ip netns`** (network namespaces) creates isolated "fake machines" behind "fake NATs" on the same PC —
  that is already what `tools/test-nat.sh` does.

So: **no need for 2 machines** to confront reality. One + netem = a full network lab, which turns "localhost"
tests into real proof.

---

## How to run the core (headless)

The `jeu` binary no longer embeds a 3D window: it is the **headless network core**. The 3D presentation lives
in Unreal, which connects through the `sidecar` mode. The project builds in a reproducible environment
(`nix-shell`) — go **into the project folder first**:

```fish
cd web3game
```

**The bridge to Unreal** (the normal case). Run the **rendezvous** (the directory), then the **sidecar** that
Unreal connects to (local socket `127.0.0.1:47800`):

```fish
nix-shell --run "cargo run -- rendezvous"   # terminal 1  (the directory — start first)
nix-shell --run "cargo run -- sidecar"      # terminal 2  (the bridge; launch Unreal after)
```

**Test the network without Unreal** (headless clients + measurement benches):

```fish
nix-shell --run "cargo run -- bot alice"        # a headless client (the real protocol, no 3D)
nix-shell --run "cargo run -- sim 50 3 15"      # 50 nodes + 3 attackers, 15 s, aggregated report
nix-shell --run "cargo run -- relay-test 6"     # deterministic NAT relay bench (both ways)
nix-shell --run "cargo run -- crowd 200"        # dense crowd (perception coverage)
```

**Characterise a link, and prove redundancy** (the network benches, without Unreal):

```fish
nix-shell --run "cargo run -- natcheck"     # link probe: NAT type (STUN), latency, jitter
nix-shell --run "cargo run -- losscheck"    # nature of loss: random vs congestion (rising-rate burst)
nix-shell --run "cargo run -- phase1"       # DETERMINISTIC bench: redundancy K vs loss (fixed-seed model)
./tools/netem-bench.sh 30                    # REAL bench: redundancy against a real 30 % loss (tc netem)
```

> `netem-bench.sh` is notable: it injects a **real** network loss (`tc netem`) **with no sudo** and **without
> touching the machine**, thanks to a **disposable network namespace** (`unshare -rn`). It displays, for
> K = 1…4 copies, the measured residual loss — to compare against the `pᴷ` prediction.

**Measure "alive" and relevance** (deterministic benches, fixed seed — see the [alive
work](chantier-vivant.md) and doubt D29):

```fish
nix-shell --run "cargo run -- vivant"       # is the perceived motion alive? (fidelity / freshness / jerk)
nix-shell --run "cargo run -- aoi"          # social relevance vs proximity (selection + allocation)
nix-shell --run "cargo run -- aoi-live"     # the same, end-to-end over the real transport (signed message)
```

**The voice benches** (work paused — the code stays replayable, everything is deterministic):

```fish
nix-shell --run "cargo run -- voix"         # voice transport: mouth-to-ear latency, jitter buffer
nix-shell --run "cargo run -- codec"        # hand-made codec (bitrate/quality curve); "codec p" = perceptual
nix-shell --run "cargo run -- micro"        # denoising, "mic study"
nix-shell --run "cargo run -- separe"       # source separation (each noise isolated, removal on demand)
```

**See the network alone, in text** (without the 3D, to observe the packets):

```fish
nix-shell --run "cargo run -- net-demo a"
nix-shell --run "cargo run -- net-demo b"
```

**Test hole punching through real NATs** (on a single PC, two routers simulated in network namespaces):

```fish
nix-shell --run "cargo build"     # compile first (outside sudo)
sudo ./tools/test-nat.sh          # sets up 2 NATs + 2 clients, watch the holes open
sudo ./tools/test-nat.sh --clean  # clean up an interrupted run
```

> The `nat-test` mode (launched by the script) replays the network scenario in text, because 3D windows
> cannot run in a namespace with no display.

**Develop with automatic reload** (the core recompiles and restarts on every save — dev comfort, via
`cargo-watch`):

```fish
nix-shell --run "cargo watch -x 'run -- rendezvous'"  # terminal 1 (directory)
nix-shell --run "cargo watch -x 'run -- sidecar'"     # terminal 2 (the Unreal bridge)
```

> *(The player's **controls** — WASD, mouse, jump — live on the Unreal side, the presentation client. The
> `jeu` binary no longer has keyboard input.)*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **How to run & test** — the **last** step of the **⚙️ The code** and **🧭 Understand
everything** paths.

**Continue the thread:**
- ✓ You are at the end of these two paths — well done, you have done the tour. Go back to the 🗺️
  [showcase](../../README.en.md), or explore 🔎 [Judge fast](etat-du-projet.md).

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)

---
