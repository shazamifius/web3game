# 📖 Glossary — every term, in one sentence

*[Version française](../glossaire.md)*

> A rescue page, to keep at hand. **There is no silly question**: if a word on another page stops you, it is
> probably here, explained simply. Terms are grouped by theme, from the most general to the most precise.

---

## The general idea

- **Peer-to-peer (P2P)** — the players' computers talk **directly** to each other, without going through a
  central computer that would hold the truth.
- **Central server** — in a classic game, the single computer in the middle that sees everything, settles
  everything, and that this project specifically seeks to **remove**.
- **"web3" (here)** — taken to mean **decentralised + an identity you own**; **no** cryptocurrency, no
  blockchain, no token.
- **Latency** — the delay between the moment something happens and the moment you see it. The project aims
  for **≤ 500 ms** so that a presence feels "alive".

## The connections (network)

- **NAT** — your router's mechanism that shares a single public address between your devices; it **blocks
  incoming connections by default** (useful for security, awkward for P2P).
- **Hole punching** ("punching through the NAT") — the trick by which two routers open a passage **at the
  same moment** to let a direct connection through.
- **"Cone" vs "symmetric" NAT** — a **cone** NAT keeps the same public port whatever the destination →
  **punchable**; a **symmetric** NAT changes it every time → **not punchable**, a relay is needed.
- **CGNAT** — a NAT operated by the carrier (typical of mobile), where **several subscribers share** one
  public address. A CGNAT can be punchable (cone) or not (symmetric) — **we measure it, we do not assume
  it**.
- **STUN** — a small question asked of a public server ("what address do you see for me?") that lets you
  **deduce the NAT type** without installing anything.
- **Relay** — when two players cannot punch through to each other, a third one **copies** their packets
  across; it **cannot alter them** (they stay signed).
- **Rendezvous** — the minimal bootstrap point that **introduces** players at the start, then can step
  aside (the only piece that is still slightly "central", acknowledged).
- **Jitter** — the irregularity of the delay: packets do not arrive at a steady rhythm. High jitter = jerky
  presence.
- **Freshness** — the age of the last piece of information received from a neighbour; it is often summarised
  by its **p95** (see below). Under 500 ms = "alive".
- **Random loss vs congestion** — losing packets **at random** (noise) is not the same as losing them
  because the link is **saturated** (congestion); the remedy differs, hence the value of **measuring** the
  nature of the loss.
- **Bufferbloat** — when a saturated link swells its queues: latency explodes **before** loss appears. A
  sign of congestion.
- **Redundancy / `p^K`** — sending a piece of information in **K copies**; if loss is *random* with
  probability `p`, the residual loss falls as `p^K` (useful). On a **saturated** link, duplicating **makes
  things worse** — hence an **adaptive** redundancy (we only duplicate where it helps).
- **netem** — the Linux kernel tool that **injects real loss/latency** onto a connection, to test a protocol
  under controlled network conditions.

## Identity and trust

- **Signature / Ed25519** — every message is sealed with your **private key**; anyone verifies the seal, no
  one can forge it. (Like an SSH key.)
- **Public key / PeerId** — your identity **is** your public key, carried in every packet: it proves itself,
  with no central directory.
- **Proof of work** (*PoW*) — a small computation to provide in order to create an identity, to make mass
  identity manufacturing **expensive** (anti-Sybil).
- **Sybil attack** — manufacturing a crowd of **fake identities** to weigh more than one should.
- **Eclipse attack** — **isolating** a victim by surrounding them with accomplice nodes.
- **Framing** (false accusation) — getting an **innocent banned**; the counter: only counting **credible and
  distinct** accusers, not heads.
- **Quorum / BFT** — requiring that a **sufficient number** of distinct participants agree before acting, to
  tolerate a minority of cheaters (*Byzantine Fault Tolerance*).
- **Own + Shields** — for a shared object, **a single** player holds authority (the **Own**); the others
  (**Shields**) verify and can depose it if it cheats.

## The crowd and perception

- **Area of interest (AoI)** — the principle that **bounds** your cost: you only talk at full rate to a
  small neighbourhood, whatever the total size of the crowd.
- **Focus vs awareness** — **focus** = the few peers tracked at full rate (detailed); **awareness** = all
  the others, perceived at **low fidelity** (silhouettes). Focus is bounded, awareness is not.
- **Cell / summary** — for a distant crowd, we replace N individual streams with **a few region summaries**
  (a sample of signed positions).
- **p95** — an honest way to summarise a measurement: the value below which **95 %** of cases fall (so we
  look at the "almost worst", not the flattering average).

## The bridges and the tools

- **Sidecar** — the **local bridge** between the network core (in Rust) and the 3D engine (Unreal), via a
  simple socket: this is what makes the core **engine-independent**.
- **Measurement agent** — a small **autonomous instrument** a volunteer runs at home: it joins the network,
  probes its link, and **reports honest figures** about the liveliness of the distant links.
- **Headless** — "with no graphical interface": the network core runs as pure computation/text, the 3D lives
  elsewhere.

---

*🗺️ [Back to the showcase](../../README.en.md) · 🧭 [The documentation, your way](README.md)*
