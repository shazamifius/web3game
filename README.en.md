# web3game

**A peer-to-peer engine for massive shared worlds — with no central game server.**

*[Version française](README.md)*

> **You are on the public showcase of web3game.** The project itself — the Rust code — lives in a separate,
> **semi-private** repository: it is not open to everyone, but I open it for reading to anyone who asks.
> This page presents it honestly; **to see the code, just write to me** (right below).

---

A universe of shared worlds where people meet, with no central server, and with an identity you genuinely
own. Hand-written network infrastructure, attacked by ourselves, and documented without rounding anything up.

**This is neither a product nor a promise.** It is a research project — deep and niche — run by one person.
What follows consistently separates what has been measured from what has not.

## Where to start

Two paths, depending on what you are after — and nothing stops you taking both:

- **[Read the documentation, freely](#the-documentation-is-freely-readable)** — the idea, the measurements
  and their limits, the open doubts. Nothing to ask anyone.
- **See the code, on request** — the repository is private, but I gladly open it for reading. Email me at
  **[shazamifius@gmail.com](mailto:shazamifius@gmail.com)** (who you are, and your GitHub handle) and I add
  you — or ask for a [live demonstration ↓](#requesting-access-to-the-code).

## The idea

No central game server holding authority: the players themselves form the network. What that changes:

- **No server cost that explodes with success** — the infrastructure *is* the players.
- **An identity you own** — a cryptographic key, like an SSH key, not an account on someone else's machine.
  *"web3" here means decentralised / self-owned identity — no token, no cryptocurrency involved.*
- **No single point of failure** that takes everyone down at once.

The scale target is openly distant: the largest coordinated battle in gaming history (EVE Online, B-R5RB,
2014, a Guinness world record) gathered thousands of players in one space — carried by exceptional
*centralised* server infrastructure. This project explores the same frontier while deliberately giving up
that server. It is a research direction, not a figure already reached.

## Built with AI — and I say so plainly

I design and build this project **together with Claude** (Anthropic's AI). I don't hide it and I don't dress
it up: it is part of the story, and I would rather tell that story honestly.

The AI is the **lever** that lets me — alone, with no team — write, byte by byte, infrastructure of this
ambition, and learn while building it. But it does not replace judgement: **every line, I read it, I
understand it, I own it.** The decisions, the doubts, the direction held, the refusal to claim victory too
early — that stays me. The AI proposes and moves fast; I decide, I verify, and I sign.

Put simply: this project, in this form and at this pace, **would not exist without that collaboration** — and
saying so plainly is my rule number one: **honesty first.**

## What is built, and verified

Core written in **Rust**, by hand, no black boxes — the only external dependency is the cryptography
library. **363 automated tests, 0 warnings**, every figure reproducible.

- **Identity is your key.** Every message is signed; no central directory decides who you are.
- **Real NAT traversal, down to the hardest case.** Two humans behind their home routers connect directly;
  when the NAT is too closed to punch through (*symmetric*), a relay takes over — proven between two real
  networks over the Internet, not in a lab.
- **A network that measures its own links** and adapts: duplicating data protects a link with random loss
  but makes a congested link worse, so it is only done when it helps. This probe has already disproved one
  of our assumptions — measurement outranks intuition.
- **Crowd perception at bounded cost.** Each node exchanges at full rate only with a small neighbourhood
  (~32) and perceives the distant crowd at low fidelity: ~34 KB/s per node (~0.27 Mbit/s), independent of
  the total population. In simulation, perception is restored to ~87 % at 1,000 nodes, with flat inbound
  bandwidth.
- **Engine-agnostic.** Two different engines (Bevy and Unreal) have been brought into the same shared space
  through a local bridge. That is what makes a multi-engine platform credible.
- **Measured in the wild, not only simulated.** A measurement agent run by volunteers recorded, over real
  remote links (several countries, some behind the hardest NAT), a live presence: p95 freshness
  ~200–335 ms, real packet loss ~0.
- **A first world-to-world transition.** A native launcher and two Unreal worlds (a hub and an island):
  walking through a portal switches you from one to the other, the old world being closed only once the new
  one is on screen. Recent, and judged smooth in real play.

## Security

A peer-to-peer network has no central server to arbitrate: every node must defend itself, against packets
written by anyone.

An internal audit on 22 July 2026 found **16 defects** — including a personal-data leak and a way to turn a
node into an amplification weapon. All 16 are fixed, each with a test that fails if the fix is removed. What
we take from it matters more than the list:

- **No single method is enough.** A shared-object theft slipped past 11 unit tests, 4 property tests,
  ThreadSanitizer and 240 million fuzzed packets — only the real-conditions attack bench caught it.
- **Proving beats trying.** [Kani](https://model-checking.github.io/kani) (AWS) broke, in seconds, an
  anti-amplification function that 240 million fuzzing attempts had deemed sound.
- **Systematic mutation testing.** Every fix is validated by putting the vulnerable code back and checking
  that the test fails. A test that cannot fail proves nothing.

## What is *not* proven

It is a project rule: write down what is proven **and** what is not.

- **Very-large-crowd scale is not directly measured.** Costs are measured up to ~1,000–2,000 nodes in
  simulation; beyond that it is architecture and extrapolation.
- **"Serverless" keeps an honest asterisk.** Bootstrapping still goes through a rendezvous point;
  decentralising that last brick is future work.
- **End-to-end encryption is not there yet.** Positions travel in clear: signatures guarantee authenticity,
  not secrecy. It is planned.
- **The decisive test is started, not passed.** We measure that the substrate carries a live presence
  between real remote machines — not yet how it feels for real players moving and playing together.

---

## The documentation is freely readable

Everything below is open, with nothing to request. It is the project's technical material: what is measured,
by which method, and where the limits are.

> **These documents are written in French** — they are the project's working notes, kept honest rather than
> polished for an audience. Machine translation handles them well, and if anything matters to you, ask and
> I will walk you through it in English.

**The idea, without jargon** — [serverless, in plain words](docs/comprendre-le-p2p.md) ·
[the project's measured state](docs/etat-du-projet.md) · [the register of doubts](docs/doutes.md)

**Architecture and technical choices** — [architecture and code layout](docs/ARCHITECTURE.md) ·
[network](docs/chantier-reseau.md) · [dense crowd](docs/chantier-foule.md) ·
[liveliness](docs/chantier-vivant.md) · [robustness](docs/chantier-robustesse.md) ·
[launcher, world to world](docs/chantier-launcher.md) · [security, and what we broke ourselves](docs/SECURITE.md)

**To judge quickly** — [measured state](docs/etat-du-projet.md) ·
[the register of doubts](docs/doutes.md) · [behind the scenes](docs/coulisses.md), where measurement
repeatedly corrected the plan

The [full table of contents](docs/) covers the rest, and a [glossary](docs/glossaire.md) defines each term in
one sentence. One honest caveat: [docs/TESTS.md](docs/TESTS.md) explains how to replay the measurements
yourself — those commands assume access to the code, which is requested just below.

## Requesting access to the code

The code is private for a simple reason: it is not ready to be published, and I would rather show it while
explaining it than let it be judged on a misunderstanding. Access is granted case by case, and gladly —
studios, companies, researchers, curious people in the field.

**Two levels, depending on what interests you:**

| | What you get |
|---|---|
| **Code reading** | Read access to the private repository: the Rust network core, the tests, the full history. |
| **Demonstration** | A live session: the network running, the world-to-world switch, the measurements replayed in front of you. |

**The simplest way: just email me.**

### → [shazamifius@gmail.com](mailto:shazamifius@gmail.com)

Tell me who you are, what you would like to see — the code, a demonstration, or both — and, if you want to
read the code, your GitHub handle (that is what lets me add you to the private repository). Nothing formal:
a few lines are enough, in English or French. I reply to everyone; if you have no answer within a week, do
follow up — it means it slipped past me.

*Comfortable with GitHub, and a publicly visible request does not bother you? You can also
[open a request through a form](../../issues/new?template=demande-acces.yml) — same treatment.*

---

*Author: [shazamifius](https://github.com/shazamifius). The code is under an all-rights-reserved licence.*
