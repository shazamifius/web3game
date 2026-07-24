# The measurements, in equations — how we quantify "alive" and a link's quality

*[Version française](../les-mesures.md)*

> The project's principle: *a proof = a reproducible figure.* We do not say "it looks smooth" or "the link is
> bad" — we **measure**, with simple, deterministic, replayable formulas. This page gathers **all the
> calculations** behind the verdicts: the fidelity of a perceived motion, the quality of a network link, the
> nature of a loss, and redundancy. Each formula is followed by what it captures **and what it does not
> prove**.
>
> See also: [the "alive" work](chantier-vivant.md) (the story) · [the network work](chantier-reseau.md) ·
> [the glossary](glossaire.md) · [how to replay the measurements](TESTS.md).

---

## 1. Is the perceived motion "alive"? — three separate measures

We play a **true** trajectory $p^*(t)$ (analytic, so known exactly), send it through a realistic network
channel, and reconstruct the **perceived** position $\hat p(t)$ on the receiver's side. We compare the two
curves by **three** distinct quantities (we refuse a single score: "alive" has three dimensions tuned
separately).

### 1.1 Shape fidelity $F$ (and freshness $d_{\text{eff}}$)

The tracing error **once the delay is compensated** — "is it the right gesture, just late?". We look for the
shift $d$ that best aligns the two curves, and keep the residual error:

$$F = \min_{0 \le d \le d_{\max}} \sqrt{\frac{1}{N}\sum_{t} \lVert \hat p(t) - p^*(t-d) \rVert^2}$$

- The sweep of $d$ is done on a **2 ms** grid. The $d$ that achieves this minimum is the **freshness**
  $d_{\text{eff}}$: the delay actually perceived. $F \approx 0$ means "what I see IS what was played, just late
  by $d_{\text{eff}}$".
- $F$ is in metres (shown in cm); $d_{\text{eff}}$ in seconds (target $\le 500$ ms, ambition $\approx 150$ ms).

### 1.2 Smoothness $J$ (the "jerk") and the jumps

The **jerk** = the norm of the 3rd discrete derivative of the perceived position (a badly filled hole
spikes):

$$J = \sqrt{\frac{1}{M}\sum_i \left\lVert \frac{\hat p_i - 3\hat p_{i-1} + 3\hat p_{i-2} - \hat p_{i-3}}{\Delta t^3} \right\rVert^2}$$

where $\Delta t = 1/f_{rx}$ is the display step. We also count the **jumps** (visible teleports):
$n_{\text{jumps}} = \#\{\, i : \lVert \hat p_i - \hat p_{i-1}\rVert > \text{threshold}\,\}$ (threshold 0.5 m
between two frames). The reference is the **natural jerk** of the trajectory itself: a perfect reconstruction
reaches it, never less.

### 1.3 The verdict

$$\textbf{alive} \iff F \le \varepsilon \ \text{ AND }\ d_{\text{eff}} \le 500\,\text{ms} \ \text{ AND }\ J \le \tau \ \text{ AND }\ n_{\text{jumps}} = 0$$

Thresholds calibrated on the first runs: $\varepsilon = 2$ cm (fidelity) and $\tau = 2 \times J_{\text{natural}}$
(jerk). Otherwise the verdict states the cause: *stuttering* ($J$ or jumps), *blurry* ($F$), *late*
($d_{\text{eff}}$).

### 1.4 How we reconstruct $\hat p(t)$ (interpolation / prediction)

The receiver displays the instant $t - d_{\text{interp}}$. Between two received states $(p, v)$ bracketing $a$
and $b$, we interpolate with a **cubic Hermite spline** (which respects the velocities at the ends → reproduces
*exactly* a straight line). With $s = (t - t_a)/\Delta t$ and $\Delta t = t_b - t_a$:

$$\hat p = h_{00}\,p_a + h_{10}\,\Delta t\, v_a + h_{01}\,p_b + h_{11}\,\Delta t\, v_b$$
$$h_{00}=2s^3-3s^2+1,\quad h_{10}=s^3-2s^2+s,\quad h_{01}=-2s^3+3s^2,\quad h_{11}=s^3-s^2$$

When the "after" state has not yet arrived, we **extrapolate** — that is where the **prediction order**
matters. With $\delta = t - t_a$:

| order | what we know | extrapolation |
|---|---|---|
| 0 | position | $\hat p = p_a$ (hold the position) |
| 1 | position + velocity | $\hat p = p_a + v_a\,\delta$ (tangent) |
| 2 | + acceleration | $\hat p = p_a + v_a\,\delta + \tfrac{1}{2}\,a\,\min(\delta, H)^2$ |

The acceleration $a = (v_a - v_{\text{prev}})/(t_a - t_{\text{prev}})$ is **estimated locally** by finite
difference of the last two received velocities — *so nothing to add to the network format.* The horizon
$H = 0.15$ s caps the quadratic term so a long hole does not make it explode. *(Measured result: order 2
divides $F$ and $J$ by ~6× at low delay — see [the "alive" work](chantier-vivant.md).)*

Finally, a **damped reconciliation** (critically damped spring, *SmoothDamp*-style) can smooth the jump of a
late correction: the displayed position follows the target with no overshoot, in $\approx \text{smooth\_time}$
seconds.

---

## 2. What is a link's quality? — what "the eye would say", quantified

Each network state carries a **monotonic** sequence number $\text{seq}$ (the anti-replay). From an observer's
point of view, the series of received $(\text{recv\_ms}, \text{seq})$ pairs is enough to deduce everything,
with no 3D and no human.

Over the observed range, $\text{expected} = \text{seq}_{\max} - \text{seq}_{\min} + 1$.

### 2.1 APPARENT loss vs REAL loss (the distinction that avoids lying to yourself)

$$\text{loss\_pct (apparent)} = \max\!\left(0,\ 1 - \frac{\text{received}}{\text{expected}}\right)$$

⚠️ **This apparent loss is misleading**: a *distant* peer is only refreshed at low cadence (by the
field-of-view mechanism), while the sender increments its seq at full rate for everyone. That peer therefore
sees seq 1, 11, 21… → apparent loss calls it "90 % lost" while **nothing** is: the sender simply did not
*send* those seqs. We correct by **inferring the cadence**:

- $\text{base} = \text{median of the consecutive seq gaps}$ (robust: a real loss makes a gap $\approx$
  double);
- for each gap $g$: $k = \max(1, \text{round}(g/\text{base}))$ expected emission slots; $\text{slots}
  \mathrel{+}= k$, $\text{missing} \mathrel{+}= k-1$;

$$\text{real\_loss\_pct} = \frac{\text{missing}}{\text{slots}}, \qquad \text{cadence\_step} = \text{base}$$

Gap $\approx 1$ step = normal; $\approx 2$ steps = a *genuinely* lost send. **It is `real_loss_pct` (not the
apparent loss) that tells a link's truth.**

### 2.2 Reordering, jitter, freshness

$$\text{reorder\_pct} = \frac{\#\{\text{seqs going backwards}\}}{\text{received} - 1}$$

$$\text{jitter\_ms} = \frac{1}{n}\sum_i \big| g_i - \bar g \big| \quad (g_i = \text{inter-arrival intervals})$$

**Freshness** (the queen quantity of "alive"): we sweep time in steps of `tick_ms` and note the age of the
last received state — a sawtooth, 0 just after an arrival, rising until the next. From it we get the
percentiles $\text{fresh\_p50}$, $\text{fresh\_p95}$ and the worst case $\text{fresh\_max}$ (percentile =
nearest rank on the sorted series).

### 2.3 The link verdict

$$
\textbf{alive} \iff \text{fresh\_p95} \le 500\,\text{ms}
$$

Otherwise: *DEAD(silent)* if zero received; *distant (low-fidelity)* if $\text{cadence\_step} \ge 4$ **or**
$\text{real\_loss\_pct} \le 20\%$ (a slow but clean link is not dead, just little-refreshed); *DEAD(>500 ms)*
otherwise. The 20 % threshold comes from an observation: healthy links ($\approx 0\%$) and lossy CGNAT links
(50–80 %) form **two clean populations** → any threshold in $[10, 40]\%$ separates them; we take 20 %
(comfortable margin).

---

## 3. What is a link's NATURE? — the probe (no external dependency)

### 3.1 NAT type (punchable or not) by STUN

We query **two** public STUN servers from a **single** socket and compare the public address seen:

$$
\text{NAT} =
\begin{cases}
\textbf{cone (punchable)} & \text{if same IP:port seen from both servers}\\
\textbf{symmetric (CGNAT)} & \text{if the public port differs}\\
\text{undetermined} & \text{if fewer than two observations}
\end{cases}
$$

A symmetric NAT redoes a mapping per destination → direct punching fails → **relay mandatory**. The median RTT
and the jitter are measured "for free" on the same STUN round-trips (no extra server).

### 3.2 Nature of the loss: congestion vs random

A short rising-rate burst produces a step curve $(\text{Mbps}, \text{loss}, \text{RTT})$. Per step:

$$\text{loss} = 100\cdot\frac{\text{sent} - \text{received}}{\text{sent}}, \qquad \text{Mbps} = \frac{\text{pps}_{\text{eff}} \cdot \text{size} \cdot 8}{10^6}$$

We compare the **base** (lowest step) to the **peak** across all steps (bufferbloat often peaks *before* the
max rate):

- **congestion** if loss climbs ($\Delta_{\text{loss}} \ge 5\%$) **or** the RTT climbs ($\text{RTT}_{\max} \ge 1.5\times\text{RTT}_{\text{base}}$ and $+30$ ms at least);
- **random** if loss is high but *flat* ($\text{base} > 5\%$ and $\Delta < 5\%$);
- **healthy** otherwise.

This distinction decides what follows: we do not treat congestion like random noise.

---

## 4. Redundancy, and why it is ADAPTIVE

On **independent** losses of probability $p$, sending $K$ copies (via the relay) only loses a packet if all
its $K$ copies are lost:

$$\text{residual loss} = p^K \qquad\Longrightarrow\qquad K = \left\lceil \frac{\ln(\text{target})}{\ln(p)} \right\rceil \ \text{(bounded)}$$

But this $p^K$ gain only holds for **random** loss. On a **congested** link, duplicating *aggravates* the
saturation (a lesson proven for real). Hence the **adaptive** decision, wired to the §3.2 probe:

| link nature | redundancy $K$ |
|---|---|
| healthy | 1 (useless) |
| random (with headroom) | $K = \lceil \ln(\text{target})/\ln(p)\rceil$ → gain $p^K$ |
| congestion | **1** (do NOT duplicate) |
| undetermined | 1 (cautious) |

*(The $p^K$ gain on random loss is proven in the lab on a real `netem` link; see [network work](chantier-reseau.md).)*

Orientation note: the coarse connection type is also **deduced from the public IP** seen on the server side
(100.64.0.0/10 = carrier CGNAT; 10/8, 172.16/12, 192.168/16 = local network; otherwise public) — a heuristic,
which the STUN probe then refines.

---

## 5. What these figures do NOT prove (honesty)

- **A bench can INVALIDATE a hypothesis, never by itself VALIDATE reality.** The "alive" bench is a
  deterministic *instrument*: it traces trade-offs and refutes false ideas, but the real judge of "is it
  alive?" remains a human moving with another via the real Internet (doubt D27 stays open).
- The bench's link profiles are *inspired* by real measurements, **not yet wired live** to the probe.
- `real_loss_pct` infers the cadence from a median: on very few arrivals, it abstains rather than invent.
- The freshness verdict is a practical threshold (500 ms), not a physical boundary; it separates two observed
  populations well, which is all we ask of it.

---

*🗺️ [Back to the showcase](../../README.en.md) · 📚 [Documentation index](README.md) · 📖 [Glossary](glossaire.md)*
