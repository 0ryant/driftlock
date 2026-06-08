# NVIDIA Inception/Connect — reply to Blaine

Draft reply to Blaine Lewis-Shallow (NVIDIA UKI Account Manager) re: preferred
pricing request. Cheeky-but-credible register; receipts are real and, where
cited for verification, public.

---

**Subject: Re: NVIDIA Inception/Connect Preferred Pricing Request**

Hi Blaine,

Thanks for the quick turnaround — and for handing me a legitimate, on-the-record
excuse to spec a Blackwell box. My co-founder and I have been after one for a while.

Quick context so the quote lands in the right place. Algol builds infrastructure
for *agentic* AI systems — not chat apps. It's a coherent, Rust-native suite that's
real and shipping today:

- **Cortex** — a longitudinal supervisory memory substrate for agents.
- **AXIOM** — governed orchestration: cryptographically-signed run-contracts plus
  invocation-time identity attestation for every agent action.
- **CellOS** — narrow-authority execution cells with a kubectl-style CLI.

All of it sits on a shared substrate core and is held to a standard most early teams
skip: we measure with **pre-registered empirical evaluations**, not vibes. One of
those is public if you want to kick the tyres — a frozen, OSF-registered protocol
(DOI 10.17605/OSF.IO/GESYH) with the hypotheses and thresholds committed *before* any
results. It's the most honest receipt I can offer that we do real science here, not
demo-ware.

Here's the bet we want to put hardware behind. There's growing evidence that a
*smaller* model, given the right scaffolding — durable memory, governed
orchestration, structured test-time reasoning — can approach the reasoning quality
of a much larger frontier model. Our suite is purpose-built to *be* that scaffolding.
So the experiment we want to run is concrete:

> How far can we push an open NVIDIA model — Nemotron-class, served through NIM —
> toward frontier-grade reasoning when it runs inside the Algol stack, measured by a
> fresh pre-registered eval in the same tradition as the one above?

If it works, the headline is yours as much as ours: NVIDIA's open models, on NVIDIA
hardware, reasoning above their weight class at local-inference cost. That's a
memory / planning / retrieval / evaluation *and* multi-agent workload — a blend of
latency-sensitive inference and heavy overnight batch eval that gets expensive and
slow to rent by the GPU-hour. Hence Blackwell on the desk.

Concretely, here's what we'd like quoted under Inception/Connect pricing:

1. **RTX PRO 6000 Blackwell workstation GPU** — primary local inference + eval
   workhorse. One to start, with a clear view to a second for multi-agent
   parallelism. (Happy to take your steer on the exact PRO SKU and config.)
2. **DGX Spark** — a dedicated developer box for the local-inference and Nemotron/NIM
   work; the unified-memory footprint suits our larger memory and retrieval models.
3. **NVIDIA AI Enterprise / NIM** — whatever entitlements pair sensibly with the above
   for a small team, plus any Inception software credits we're entitled to.
4. **Your read on the right platform** — if we're under- or over-speccing for
   memory / planning / retrieval / eval plus multi-agent, tell us. We'd rather buy the
   right thing once.

We're early but moving fast, recently into Inception, and shipping across the suite
weekly. Happy to get on a call with whoever owns this — and if there's a version of
this where the Blackwell justifies itself on paper, I'd love it in writing before my
co-founder talks me back down to a cloud instance.

Cheers,
Ryan Tilcock
Co-Founder, Algol

---

## Receipts & verification notes (not for the email)

**Public — Blaine can click these:**
- `cma-m2-eval` — pre-registered empirical eval of the Controller Mind Architecture
  (CMA) / M2 policy layer vs a Latent State Model (LSM) baseline; three batteries
  (topology, counterfactual, social-signal). OSF DOI 10.17605/OSF.IO/GESYH, protocol
  frozen 2026-03-11. This is the one to send if he wants to verify "pre-registered."
- `driftlock` — ADR-derived work orders + safe lanes for multi-agent delivery (Rust).

**Private — do NOT invite him to browse the source:**
- `cortex`, `CellOS`, `algol-substrate`, `axiom-composer`, `aegress` are private. The
  email describes them as products (fine) but never links them, to avoid a dead link.

**Open decisions before sending:**
- Confirm **RTX PRO 6000 Blackwell** is the exact workstation SKU to name (vs. leaving
  it as "RTX PRO Blackwell" and letting Blaine pick the config).
- Confirm the quantity anchor: "one, view to a second."
- Platform-team-memory angle (Cortex/CellOS at org scale) intentionally held back for
  the call, where it reads as expansion rather than scope-creep.
