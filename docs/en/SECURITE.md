# Security — what we broke at home, and how

*[Version française](../SECURITE.md)*

This document tells the story of a security audit run on this project on **22 July 2026**, the defects it
found, and the method used. It is published because a repository that shows what it fixed is more useful —
and more honest — than a repository that claims all is well.

The project is a peer-to-peer network: each participant runs a node that talks directly to the others. There
is no central server to arbitrate, so **each node must defend itself alone**, against packets written by
anyone.

## Result in one line

**16 defects found, 16 fixed**, each with a test that fails if the fix disappears.

| Severity | Defect | What was possible |
|---|---|---|
| 🔴 | Presence log served publicly | Read the IP address, connection times and machine name of every participant |
| 🔴 | Unauthenticated `PUNCH` | Reflect a node's stream toward an arbitrary victim — 34 bytes sent, 728 emitted, indefinitely |
| 🟠 | Replay of a signed redirection | Send the whole fleet toward a dead or reassigned server |
| 🟠 | No version ↔ content binding | Reinstall an old vulnerable executable, **permanently** |
| 🟠 | Unsigned `HELLO` | Register under someone else's identity: their traffic arrives at you, they become unreachable |
| 🟠🟡 | 5 denials of service | Exhaust the server's memory or fill its disk |
| 🟡 | 2 defects on the shared object | Steal it remotely; lock it forever |
| 🟡 | Object theft found by the attack bench | Take a free object without ever approaching it, then make it unreachable |

Two other defects were only visible by querying the **real server**, not the code: a liveness probe aimed at
a nonexistent URL (so the client permanently believed all servers were dead), and a cryptographic
verification placed before its own rate guard — a security fix that opened another one.

## How they were found

No single method was enough. That is the most important point of this document.

### 1. Audit by agents with adversarial verification

82 agents launched in parallel over the sensitive areas, in two stages: search, then **refute**. Each finding
is submitted to independent sceptics whose instruction is to demolish it.

25 raw findings → **14 confirmed, 8 refuted**. A third of the alerts were false. Without the refutation step,
we would have "fixed" code that was not broken.

### 2. Property tests

An example test says *"for this input, here is the output"*. It says nothing about the case no one thought of
— and a security defect is exactly that.

Four properties, stated as universal rules:

- no signed message is accepted after **a single bit** is altered (exhaustive: each byte, each bit);
- no accepted version ever goes back down, on **any** channel;
- we never send back to an unvalidated address more bytes than it sent;
- no table grows unbounded under hostile traffic.

The first one immediately flagged a byte not covered by a signature. On checking, it was deliberate — but the
exception is now **declared and justified in the code**, and the test requires it to stay real.

### 3. Mutation testing — the discipline that matters most

For each fix: put the vulnerable code back, **verify that the test fails**, restore.

That is what distinguishes a useful test from a decorative one. On the shared object, the demonstration is
final: putting the two original behaviours back, the 2 new tests fall and **none of the 8 existing tests
moves**. Those 8 tests had passed forever, right next to the flaw.

Each attack test also carries an **anti-hollow-test guard**: it first requires the normal path to work.
Without it, "no byte went to the victim" would be true simply because nothing was sent at all.

### 4. Confrontation with external tools

| Tool | What it looks for | Result |
|---|---|---|
| **ThreadSanitizer** | races between threads | 0 on 351 instrumented tests |
| **Kani** (AWS) | formal proof of absence of panic | **1 defect found**; 6 proofs out of 8 (2 on text too costly, covered otherwise) |
| **Wycheproof** (Google) | known pitfalls of signature verification | 150 cases, verdict identical to the reference — 62 pitfalls all rejected |
| **CodeQL** (GitHub) | semantic static analysis (SAST) of the Rust | 63 files out of 63 analysed, 0 alert *(repo then public; suspended since — see below)* |
| **Deterministic fuzzing** | hostile packets | 240,000,000 hand decodes, 0 panic — now replayed continuously (nightly campaign with a varying seed) |
| **Clippy** | quality | 0 remark |

**Kani** did better than confirm: it **found a defect that 240 million fuzzing attempts had missed**. An
anti-amplification function clamped an oversized length to the maximum representable value instead of
rejecting it — the invariant "never more bytes emitted than received" was false in an extreme case. A proof
tries *all* values at once; a test, only those we imagined. Fixed, then **proven**. Two proofs bearing on
text analysis remain out of Kani's practical reach (symbolic exploration of strings explodes — a known limit
of the method, not a code defect): they are covered by property tests and fuzzing, and the code says so
without rounding.

**Wycheproof** is the test battery of Google's security team, written to *break* cryptography
implementations. Our signature verification gives the same verdict as it does on the 150 cases, including the
62 pitfalls (malleable signatures, degenerate keys, non-canonical encodings). Verified by mutation: a
verification that accepted everything lets 50 of these pitfalls through.

**CodeQL** (GitHub's static analyser) builds a semantic model of the code — not a mere pattern search — and
looks for known vulnerability schemes in it. It understands Rust natively since late 2025. Run over the
network core, it analysed **63 Rust files out of 63** and raised **no alert**. That "zero" only counts
because the run logs prove the Rust extraction actually happened (database built, queries executed) — the
same requirement as further down: *a "0 problems" must prove that something ran.*

> **Update (July 2026) — CodeQL is suspended, honestly.** The analysis was replayed on every push while the
> repository was **public**. The code repository has since gone **private**, and GitHub only offers CodeQL
> for free on public repositories (on a private repo you need *GitHub Advanced Security*, paid): the analysis
> is therefore **no longer refreshed**. The result above stays true for the version analysed — it is simply
> no longer re-run on every commit. **Deterministic fuzzing**, on the other hand, keeps running in continuous
> integration.

### 5. The attack bench — what found what nothing else saw

The project embeds its own attacking program: 11 real attacks, real sockets, real forged packets, against
real nodes.

**One attack succeeded.** The shared object was stolen from both victims. At the same moment: the 11 unit
tests passed, the 4 properties passed, ThreadSanitizer saw nothing, and neither did 240 million fuzzing
packets.

Only real execution revealed it.

## What formal proof brought that 240 million attempts did not

The fuzzer had submitted 240 million packets to the anti-amplification credit function without finding
anything. [Kani](https://model-checking.github.io/kani) broke it in seconds.

The function clamped a too-large size to the maximum representable value, instead of rejecting it. With a
maximal credit, a send of several billion bytes therefore went through while being charged only four billion
— the invariant "never more bytes emitted than received" was **false**.

Not exploitable as such (a datagram caps at 64 KB), but the guard would have silently broken the day this
function was used elsewhere. The tests had not seen it because they only tried realistic sizes. **A proof, in
contrast, tries everything.**

The general rule comes out reinforced: *on a quantity to be paid, saturating upward always under-charges; the
safe direction is to refuse.*

## The lessons, worth more than the fixes

**A guard placed on ONE path is not a guard.** Four defects out of sixteen are variants of this: the
protection existed on one channel and was missing on another. A single point of trust is only worth something
if every barrier there is complete.

**Signing proves WHO, never WHEN.** Any signed data stays replayable for life. You need a version inside the
signed content, and a floor that **survives deletion of the file** — deleting a file is within the attack's
reach.

**Budgets first, cryptography second.** Verifying a signature is expensive. Placing that verification before
the rate guard turns a protection into a denial of service.

**Measure before setting a threshold.** A jerk detector tuned by intuition (150 ms) detected nothing, even
under 48 concurrent processes. Measurement done — 0 ms at rest, 189 ms under load — the threshold moved to
50 ms. A poorly calibrated instrument is worse than no instrument: it reassures.

**A "0 problems" must prove that something ran.** ThreadSanitizer first announced "no race detected"… while
nothing had compiled. The proof that the analysis actually happened is part of the result.

**A hard-coded message is not a measurement.** The attacking program printed "the object is NOT stolen" at
the exact moment it had just stolen it. The verdict is read on the victim's side.

**These are not backdoors.** A backdoor is deliberate; these are mistakes. The distinction is not a
vocabulary detail.

## Acknowledged choices, and their costs

Security has a price. What is paid here, in black and white:

- **An identity never moves while it is alive.** Closes impersonation; costs a slower reconnection to anyone
  who actually changes address mid-session.
- **A peer's address is never learned from received traffic**, only from a corroborated source. A signed
  state proves a key emitted it once, never that it is at this address now.
- **The list of publicly served files is a whitelist, not a blacklist.** A blacklist always forgets
  something; a whitelist forgets on the safe side. That is what closed the presence-log leak.
- **Zero dependencies, except cryptography.** We never write our own cryptography; all the rest of the
  protocol is hand-made and readable.

## Limits — what this document does not claim

- The presence log was exposed for an unknown duration. You had to know the URL, which makes a lookup
  unlikely. "Unlikely" is not "no one".
- ThreadSanitizer only sees the paths the tests execute: "no race" means "no race where the tests pass".
- Fuzzing is not coverage-guided: 240 million packets are not a proof.
- Formal proofs bear on the absence of decoder panics, not on the correctness of the whole protocol.
- **Sixteen defects found does not mean zero remaining.** This document says what was searched and how, not
  that the search is finished.

## Reporting a problem

Open an *issue* on this repository. If the subject is sensitive, say so without detailing the flaw in the
public thread, and a private channel will be offered.

---

*🗺️ [Back to the showcase](../../README.en.md) · 📚 [Documentation index](README.md) · 📖 [Glossary](glossaire.md)*
