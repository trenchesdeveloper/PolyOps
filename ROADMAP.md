# PolyOps — Roadmap (post-0.0.6)

> Forward-looking plan. Companion to [`PLAN.md`](PLAN.md) (the original
> build plan, milestones 0–9) and [`PERFORMANCE_PLAN.md`](PERFORMANCE_PLAN.md).
> Priority: **P0** now · **P1** next · **P2** soon · **P3** later.
> Effort: **S** <½ day · **M** ~1–2 days · **L** multi-day.
> **Last updated:** 2026-06-14.

---

## Where we are

Shipped `0.0.6` to crates.io + npm (7 prebuilt platforms): full parity with
`martinez-polygon-clipping@0.8.1`, a typed-array fast path that's the default
(faster than martinez at every substantive size), dual CJS/ESM packaging,
npm README, criterion + head-to-head benchmarks, and a hardened, idempotent,
main-only release pipeline.

What we learned that shapes this roadmap:
- The **pure-Rust** engine is where the big win lives (1.8–2.9×); a Rust
  consumer benefits most. → invest in Rust-ecosystem adoption (Track A).
- The **N-API per-call cost** still loses to in-process JS for *many tiny*
  ops (the process-photo pattern). The flat path fixed per-op marshalling;
  a **batch API** is the remaining lever. → Track B.
- We hit real **release-engineering friction** (manual version bumps, the
  exact-version path-dep gotcha, stacked-PR mis-merges, tag dances). → harden
  it (Track E).

---

## Track A — Rust-ecosystem adoption *(highest leverage; the real perf win is here)*

| ID | Item | P | Effort |
|----|------|---|--------|
| A1 | **`geo-types` interop** behind a feature flag — `From`/`Into` between polyops types and `geo::{Polygon, MultiPolygon}`. Unlocks the `geo`/`geozero`/`geojson` ecosystem; the single biggest crates.io adoption lever. | P1 | M |
| A2 | **`serde` feature** for `Geometry`/types — Rust consumers deserialize GeoJSON directly without going through `polyops-napi` (PLAN §14). Keep it opt-in; core stays serde-free. | P1 | S |
| A3 | **Public `Operation` enum** — some consumers `switch` on it (martinez exposes `{ INTERSECTION, UNION, … }`) (PLAN §14). | P2 | S |
| A4 | **rustdoc + docs.rs** — module/function docs with runnable `# Examples`, doctests in CI, docs.rs metadata. Currently the public API has no examples. | P1 | S |

---

## Track B — JS/npm performance & reach

| ID | Item | P | Effort |
|----|------|---|--------|
| B1 | **Batch API** — `unionBatch(pairs)` etc. that processes N polygon-pairs in **one** N-API call, amortizing the per-call boundary cost. This is the one thing that could make polyops win even for *many small* ops (the workload where it currently loses to in-process JS — see BENCHMARKS.md). High-value, novel. | P1 | M |
| B2 | **WASM build** (`wasm-bindgen`/`wasm-pack`) — browser consumers + the Leaflet-demo crowd (PLAN §13/§14 future work). Big reach; the engine is `no_std`-friendly-ish already. | P2 | L |
| B3 | **Streaming / arena reuse** across calls for thousands of sequential pairs (PLAN §14). B1 (batch) captures most of this benefit more cheaply — do B1 first, revisit B3 only if demand. | P3 | L |

---

## Track C — Core performance backlog *(profile-guided; we now have the benches to gate it)*

From PLAN §11. Diminishing returns for the napi consumer (boundary-bound),
but real for pure-Rust / WASM / batch consumers. Land each with a before/after
delta in `BENCHMARKS.md`.

| ID | Item | P | Effort |
|----|------|---|--------|
| C1 | Pre-size collections (`Vec::with_capacity`) | P2 | S |
| C2 | `smallvec` / inline buffers for short-lived per-event arrays | P2 | S |
| C3 | `robust` predicate `f64` fast-path with adaptive escalation | P3 | M |
| C4 | `splay_tree` sweep-line status (only if profiling fingers it) | P3 | M |
| C5 | SIMD orientation predicates in the inner loop | P3 | L |

---

## Track D — Correctness & robustness

| ID | Item | P | Effort |
|----|------|---|--------|
| D1 | **Property tests** — commutativity (`A∪B == B∪A`), idempotence (`A∩A == A`), identity (`A∖∅ == A`), `xor == (A∪B)∖(A∩B)`. Catches bugs the fixtures miss (PLAN §14). | P1 | S |
| D2 | **Fuzzing** (`cargo-fuzz`) on `segment_intersection`/`connect_edges` — panic-safety on degenerate/adversarial input. | P2 | M |
| D3 | **Track upstream** — refresh goldens against new `martinez` patch releases; decide on `0.9` if it lands; keep parity the gate. | P2 | ongoing |

---

## Track E — Project maturity & release engineering *(we felt this pain)*

| ID | Item | P | Effort |
|----|------|---|--------|
| E0 | **Deprecate broken `0.0.2`/`0.0.3`** on npm (`npm deprecate …`). Trivial, do now. | P0 | S |
| E1 | **`CONTRIBUTING.md`** — parity harness, fixtures, test structure, release flow (project DoD, still missing). | P1 | S |
| E2 | **Bump CI actions off Node-20** — `actions/checkout`, `setup-node`, `upload/download-artifact` to current (the deprecation warnings in every run). | P1 | S |
| E3 | **Release automation** — a version-bump script (workspace + the exact-match path-dep + `napi version` + lockfiles, in one shot — the manual flow bit us repeatedly), a `workflow_dispatch` trigger (no more tag-move dances), and an optional `dry_run`. | P1 | M |
| E4 | **`NPM_TOKEN` rotation runbook** — 90-day cap; document + calendar so releases don't fail auth. | P1 | S |
| E5 | **Cut `0.1.0`** — once Track A lands a stable Rust API, commit to semver and graduate from `0.0.x`. Path to `1.0` after a soak. | P2 | S |

---

## Recommended near-term sequence

A "**0.1.0-readiness**" arc that front-loads quick wins and the
adoption-critical work:

1. **E0** — deprecate the broken versions (minutes).
2. **E1 + E2** — CONTRIBUTING + CI Node bump (hygiene, cheap).
3. **D1** — property tests (cheap robustness insurance).
4. **A4 + A2 + A3 + A1** — docs + serde + `Operation` + `geo-types`: the
   crates.io adoption bundle → **cut `0.1.0`** with a stable, ecosystem-friendly
   Rust API (E5).
5. **B1** — batch API: the high-value JS perf feature for many-small-ops.
6. **B2 / Track C** — WASM and profile-guided perf as demand dictates.

---

## Deferred / non-goals (for now)

- **Other algorithms** (Vatti/Clipper, i_overlay-style). The `polyops` name
  leaves room, but it's a large separate effort — revisit only on demand.
- **Additional ops** (offset/buffer, area, centroid) — outside Martinez's
  scope; would be a different product.
- **Multi-threading** — single-thread matches the target workloads; not worth
  the complexity yet.
