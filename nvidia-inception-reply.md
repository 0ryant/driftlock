# NVIDIA Inception/Connect: reply to Blaine

Draft reply to Blaine Lewis-Shallow (NVIDIA UKI Account Manager) re: preferred
pricing request. Cheeky-but-credible register; receipts are real and, where
cited for verification, public.

---

**Subject: Re: NVIDIA Inception/Connect Preferred Pricing Request (yes, and I have a list)**

Hi Blaine,

Thanks for the quick turnaround. I'll be honest with you: "tell me what you'd like
to buy" is a dangerous sentence to send a founder who's been quietly building a case
for a Blackwell box to his co-founder for months. You've just handed me the excuse.
So, in writing, for the record: yes please.

Now the part that makes this a real request and not a wishlist. Algol builds
infrastructure for *agentic* AI systems, not chat apps. It's a coherent, Rust-native
suite, and it exists today, not as a deck but as code:

- **Cortex:** longitudinal supervisory memory for agents. The thing that remembers
  so the agents don't have to keep being reminded.
- **AXIOM:** governed orchestration. Cryptographically-signed run-contracts and
  invocation-time identity attestation on every agent action. Nothing runs that can't
  prove who it is and what it's allowed to do.
- **CellOS:** narrow-authority execution cells with a kubectl-style CLI. Agents get
  exactly the blast radius you grant them and not one syscall more.

Those three are the headline acts, but they sit on top of twenty-odd other apps I've
deliberately engineered against the specific failure modes I keep watching AI walk
into. I'll happily name every one and what it does in a separate email if you're
interested; I'm just trying to keep this one focused on the bit with a price tag.

All of it on a shared Rust substrate. And here's the bit most early teams quietly
skip: we hold it to **pre-registered empirical evaluations**. Frozen protocol,
hypotheses and thresholds locked *before* the results exist, the whole open-science
ritual. Here's one, public, no login required: https://osf.io/gesyh/. It is genuinely
titled *"Typed Strategic Families Produce Distinct Failure Morphologies"*, which tells
you roughly how much fun we have on a Friday, but it's the most honest receipt I can
offer that we measure things here rather than vibe them.

And here's why any of this matters beyond my own entertainment. Every quarter more of
the enterprise actually puts AI into production, and every quarter the risk and
compliance teams get a little more nervous, because "the model decided" is not an
answer that survives an audit. Capability stopped being the bottleneck a while ago;
the bottleneck now is being able to *prove* what an agent is, what it was allowed to
touch, what it remembered, and why it did the thing it did. That's the whole bet
behind Algol: governed AI, built so the compliance team's hardest question has a
boring, documented answer. And, conveniently for this email, "provable and governed"
is exactly the kind of workload you want running somewhere you actually control,
rather than rented by the hour on someone else's box.

Here's the bet I actually want to put a GPU behind. The interesting frontier right
now isn't "bigger model." It's that a *smaller* model, wrapped in the right
scaffolding (durable memory, governed orchestration, structured test-time reasoning)
can punch well above its weight. Our entire suite is, conveniently, that scaffolding.
So the experiment writes itself:

> How far can we push an open NVIDIA model, Nemotron-class, served through NIM, toward
> frontier-grade reasoning when it runs inside the Algol stack, and prove it with a
> fresh pre-registered eval?

If that lands, the headline is yours as much as mine: NVIDIA's own open models, on
NVIDIA silicon, reasoning above their weight class at local-inference cost. I'd quite
like to be the case study you didn't have to write yourself.

To do it properly I need the workload on the desk. Memory, planning, retrieval,
evaluation, and multi-agent loops are a nasty mix of latency-sensitive inference and
heavy overnight batch eval, and renting that by the GPU-hour is both slow and a great
way to set money on fire. Hence: Blackwell, locally.

So, concretely, here's what I'd love quoted under Inception/Connect pricing:

1. **RTX PRO 6000 Blackwell workstation GPU:** the 96GB is the whole point; it's what
   lets a Nemotron-class model actually live on the card with room to think. One to
   start, with a very deliberate view to a second for multi-agent parallelism. (Take
   the wheel on exact SKU/config; I suspect you'll have opinions, and I want them.)
2. **DGX Spark:** a dedicated developer box for the local-inference and Nemotron/NIM
   work; the unified memory is lovely for the larger memory and retrieval models.
3. **NVIDIA AI Enterprise / NIM:** whatever software entitlements pair sensibly with
   the above for a small team, plus any Inception credits we've earned the right to.
4. **Your honest read on the platform:** if we're under- or over-speccing for that
   workload, tell me. I'd genuinely rather buy the right thing once than the wrong
   thing twice.

We're early but moving fast, freshly into Inception, and shipping across the suite
weekly. I'd love to get on a call with whoever owns this and turn the above into real
numbers, and if there's a version of events where the Blackwell pays for itself on
paper, please put it in writing before my co-founder talks me back down to a cloud
instance.

Looking forward to it,

Ryan Tilcock
Co-Founder, Algol

---

## Receipts & verification notes (not for the email)

**Public (Blaine can click these):**
- `cma-m2-eval`: pre-registered empirical eval of the Controller Mind Architecture
  (CMA) / M2 policy layer vs a Latent State Model (LSM) baseline; three batteries
  (topology, counterfactual, social-signal). OSF DOI 10.17605/OSF.IO/GESYH, protocol
  frozen 2026-03-11. This is the one to send if he wants to verify "pre-registered."
- `driftlock`: ADR-derived work orders + safe lanes for multi-agent delivery (Rust).

**Private (do NOT invite him to browse the source):**
- `cortex`, `CellOS`, `algol-substrate`, `axiom-composer`, `aegress` are private. The
  email describes them as products (fine) but never links them, to avoid a dead link.

**Open decisions before sending:**
- **OSF link:** CONFIRMED PUBLIC. Verified to render in a logged-out private-browsing
  window (title "Typed Strategic Families Produce Distinct Failure Morphologies",
  contributor Ryan Tilcock, "Public registration"). Live link https://osf.io/gesyh/ is
  now embedded directly in the email. No login wall for Blaine.
- **Card variant:** named the family ("RTX PRO 6000 Blackwell", 96GB GDDR7) and handed
  Blaine the SKU call. If the two-card plan is real, lean **Max-Q Workstation Edition**
  (300W, blower, built for up to 4 GPUs/box) over the 600W Workstation Edition; two
  600W cards in one chassis is a power/thermal headache.
- **Quantity anchor:** "one, view to a second" kept (signals growth + gives the rep an
  upsell path).
- Platform-team-memory angle (Cortex/CellOS at org scale) intentionally held back for
  the call, where it reads as expansion rather than scope-creep.
