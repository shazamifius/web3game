# Serverless, in plain words

*[Version française](../comprendre-le-p2p.md)*

> The project's central idea, **in plain words**: start from a classic online game, remove the central
> server, and look — problem by problem — at what replaces it. No jargon.
>
> For the code, see [the architecture](ARCHITECTURE.md); for the measured figures, the [state review](etat-du-projet.md).

---

## The starting point: the "normal" model

Almost every online game works the same way: a **central server** in the middle. Your game talks to it, it
talks to the other players, it holds the truth (your position, your score), it settles conflicts. It is
simple, and it works very well.

But that server has a price:
- **it is expensive** — the more players there are, the more powerful machines you need;
- **it belongs to someone** — who can see everything, cut everything off, shut the service down overnight;
- **it is a single point of failure** — if it goes down, everyone goes down with it.

This project's question is simple: **what if we removed it?**

## What breaks when you remove the server (and how we repair it)

Without a server, players have to manage **directly among themselves**: that is "**peer-to-peer**" (P2P).
Six problems then appear. Here they are, one by one, with their solution.

### 1. "Who are you?" — with no account on a server

Normally, the server is what knows that this account is you. Without it, how do you prove your identity?

**The solution: your identity is a key.** Like a house key, but digital (exactly the principle of an **SSH
key**). You **sign** every message with it; anyone can verify the signature, but no one can forge it. So
**no one can impersonate you**, and **no central directory "decides" who you are**: your identity proves
itself.

### 2. "How do you find each other?" — with no central directory

Two players who don't know each other still have to manage to talk. Without a directory, how?

**The solution: a simple "rendezvous point" that makes the introductions** — like someone at a party who
introduces two guests, then steps aside: they talk directly afterwards. Once the introductions are done, you
could **turn off the rendezvous**, and the game goes on. *(This is the only piece that is still slightly
"central" — the honest asterisk of "serverless". More on it at the end.)*

### 3. "The routers block everything" — NAT

Your home router **hides** your computer and **blocks** connections coming from outside (that is "NAT", and
it is normal: it protects you). The problem: for peer-to-peer, we specifically need others to be able to
reach you.

**The solution: "hole punching"** — a small trick where both routers open a passage **at the same moment**,
by surprise. It works in most cases. And when it is impossible (some mobile connections), another player
acts as a **relay**: it copies the packets across, without being able to alter them (they stay signed — see
point 1).

### 4. "You can't talk to 55,000 people" — the area of interest

If everyone talked to everyone, traffic would explode (at 55,000 players, it is unmanageable).

**The solution: you only talk at full rate to your immediate neighbours (~32).** The distant crowd you
perceive at **low fidelity** (a summary, silhouettes), not in detail. As a result, your network cost stays
**bounded** — it does **not** depend on the total number of players, only on your small neighbourhood. *(The
figures: [state review](etat-du-projet.md) — ~34 KB/s per node, stable whatever N is.)*

### 5. "Who is right about a shared object?" — with no referee

Two players grab the same object "at the same time". With no server-referee, who wins?

**The solution: "Own + Shields".** For each contested object, **a single** player holds authority at a given
moment (the "Own"). The others ("Shields") **verify** what it announces and can **depose** it if it cheats.
If the Own leaves or goes silent, authority **migrates** to another. No central server: just rules that
everyone applies the same way.

### 6. "Who prevents cheating?" — with no moderator

No central moderator. So who stops the cheaters?

**The solution: several layers.** **Signatures** (point 1) already prevent forging other people's messages.
Added to that: **plausibility rules** (you don't teleport across the map in one step), a **shared
reputation** (a spotted cheater is muted for everyone), and the principle of **corroboration** (you don't
take a player at their word when they speak about a *group*: you cross-check against what others report).

## A clarification: here, "web3" does not mean crypto

The word "web3" is often associated with cryptocurrencies. **In this project, it has nothing to do with
that.** "web3" is taken to mean **decentralised** + **an identity you genuinely own** — and that is all. **No
token, no blockchain, no speculation.** The only "crypto" present is the **cryptographic signatures** (like
an SSH key) to prove identity — the same thing that already secures your connection to a website. Nothing to
do with money.

## So what exactly does "serverless" mean?

The honest meaning: **no central server that holds the truth and referees the game.** The logic lives with
the players. **The asterisk** (point 2): a small rendezvous point still helps with the **introductions** at
startup — it can be turned off once players are connected, and fully decentralising it remains an open
project. So the exact phrasing is "**no central server holding authority**", not "zero servers". *(This
honesty about limits is a project rule — see the [acknowledged walls](etat-du-projet.md).)*

---

### 🧭 Finding your way — wherever you start, you are in the right place

You are reading **Serverless, in plain words** — the first step of the **🌱 Discover** and **🧭 Understand
everything** paths.

**Continue the thread:**
- 🌱 *Discover* · 🧭 *Understand everything* → **[The project's measured state](etat-du-projet.md)**

**The doors** (jump, come back, switch at any time):
🌱 [Discover](comprendre-le-p2p.md) · ⚙️ [The code](ARCHITECTURE.md) · 🔎 [Judge fast](etat-du-projet.md) · 🧭 [Understand everything](README.md) · 📖 [Glossary](glossaire.md) · 🗺️ [The showcase](../../README.en.md)
