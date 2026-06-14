# Performance Plan — Milestone 7 (Benchmark Baseline)

> **Status:** Proposed. Implements PLAN.md §11 (Performance plan) and
> Milestone 7 (§13). Companion to [`PLAN.md`](PLAN.md) and
> [`RELEASE_PLAN.md`](RELEASE_PLAN.md).
> **Last updated:** 2026-06-12.

---

## 1. Goal & success criteria

Land a reproducible benchmark suite and establish the **baseline** numbers
that justify the project's premise (PLAN.md §1, §3): swapping
`martinez-polygon-clipping@0.8.1` for `polyops` in `process-photo` is
faster.

**Milestone 7 is done when** (PLAN.md §13-M7):
- A `criterion` bench suite runs via `cargo bench` over the four scenarios
  below.
- A Node harness compares `martinez-polygon-clipping@0.8.1` vs the
  `polyops` napi binding on the same inputs.
- **`polyops` (Rust, single-thread) matches or beats `martinez@0.8.1`** on
  all four scenarios, and the napi path is within a known overhead of the
  pure-Rust path.
- Numbers are written up (committed `BENCHMARKS.md` + a README table).

> M7 is **baseline only** — we measure, we don't optimize. The
> optimization backlog (§7) is explicitly *after* the baseline lands.

---

## 2. Scenarios (mirror upstream `bench/martinez.bench.ts`)

Upstream's bench runs three `union` workloads; we mirror them exactly, then
add the PolyOps-specific one. **All three upstream scenarios are `union`
operations** (confirmed from the upstream harness — "States clip" is
named for the data, but calls `union`).

| Scenario | Op | Inputs (from the fixtures in §3) | Stresses |
|----------|-----|----------------------------------|----------|
| `hole_hole` | union | `hole_hole.features[0]` ∪ `hole_hole.features[1]` (1.8 KB) | hot kernel, many degeneracies |
| `asia_union` | union | `asia.features[0]` ∪ `asia_unionPoly.geometry` (1.2 MB subject) | sweep-line scaling, ~tens of thousands of vertices |
| `states_clip` | union | `states_source.features[0]` ∪ `states_source.features[1]` (92 KB) | many polygons / multi-polygon path |
| `clip_path_flatten` | intersection + union | extracted from `process-photo` `test39` (FE-1866), see §4 | the real-world target workload |

The first three give parity with upstream's published numbers (a sanity
check that our measurement is sound); `clip_path_flatten` is the number
that actually predicts the `process-photo` win.

---

## 3. Fixtures — vendor them into the repo

The bench inputs live in the upstream clone at
`~/Desktop/rust-learn/martinez/test/fixtures/`. Benches must be
reproducible without that clone (same principle as the committed goldens),
so we **vendor the four fixtures into the repo**.

- **Location:** `crates/polyops/benches/fixtures/`
  - `hole_hole.geojson` (1.8 KB)
  - `asia.geojson` (1.2 MB) + `asia_unionPoly.geojson` (626 B)
  - `states_source.geojson` (92 KB)
  - `clip_path_flatten.json` (extracted, §4)
- **Access patterns** (verbatim from upstream bench — important, they
  differ per fixture):
  - `hole_hole`: `features[0].geometry.coordinates`, `features[1].geometry.coordinates`
  - `asia`: subject = `asia.features[0].geometry.coordinates`; clip = `asia_unionPoly.geometry.coordinates` *(a bare `geometry`, not a feature)*
  - `states_source`: `features[0].geometry.coordinates`, `features[1].geometry.coordinates`
- **Size note:** `asia.geojson` is 1.2 MB. That's fine to commit once
  (goldens already total ~comparable); it changes ~never. If we'd rather
  not, gate the asia bench behind a `POLYOPS_BENCH_FIXTURES` env path —
  but committing is simpler and keeps `cargo bench` zero-setup.
- **Refresh script:** add `parity/copy-bench-fixtures.ts` (or a Make
  target) that copies the four files from `$MARTINEZ_REPO/test/fixtures/`
  into `crates/polyops/benches/fixtures/`, so they can be refreshed when
  upstream bumps. Mirror the `MARTINEZ_REPO` convention from
  `generate-goldens.ts`.
- **License:** upstream is MIT; vendoring its test fixtures is fine. Note
  their origin in a `benches/fixtures/README.md`.

---

## 4. `clip_path_flatten` extraction (the PolyOps-specific scenario)

PLAN.md §11 seeds this from `process-photo/test/process-photo-v3/assets/test39/`
(the 3-level nested clip-path case from FE-1866), reduced to its
polygon-Boolean inputs.

Plan:
1. In `process-photo`, instrument `src/utils/svg.ts` to dump the
   `(subject, clipping, op)` triples it passes to martinez during the
   `test39` clip-path flatten (a temporary `JSON.stringify` to a file).
2. Capture the representative sequence (likely an `intersection` chain for
   nested clips, possibly a `union` to merge). Save the coordinate arrays
   to `crates/polyops/benches/fixtures/clip_path_flatten.json` as
   `{ steps: [{ op, subject, clipping }, ...] }`.
3. The bench replays the recorded steps, so it measures the *actual*
   operation mix the Lambda performs, not a synthetic one.

> If `process-photo` isn't accessible when implementing, ship M7 with the
> three upstream scenarios first and add `clip_path_flatten` when the
> repo is in hand — `log()`/note the gap rather than silently dropping it.

---

## 5. Implementation

### 5.1 Rust `criterion` suite — `crates/polyops/benches/benchmarks.rs`

Replace the stub. The `[[bench]]` entry already has `harness = false`
(criterion-ready).

- **Dep:** add to `crates/polyops/Cargo.toml`:
  ```toml
  [dev-dependencies]
  criterion = { version = "0.5", features = ["html_reports"] }
  serde = { workspace = true }      # already present
  serde_json = { workspace = true } # already present
  ```
- **Fixture loading:** reuse the `coords → Geometry` logic already proven
  in [`tests/parity.rs`](crates/polyops/tests/parity.rs) (the `depth()` /
  `coords_to_geometry()` helpers — lift them into a shared
  `benches/common.rs` or duplicate; they're ~20 lines). Parse fixtures
  **once, outside** the `b.iter()` loop so parsing isn't measured.
- **Shape:**
  ```rust
  fn bench_scenarios(c: &mut Criterion) {
      let mut g = c.benchmark_group("union");
      for (name, subj, clip) in load_scenarios() {        // hole_hole, asia, states
          g.bench_function(name, |b| {
              b.iter(|| union(black_box(subj.clone()), black_box(clip.clone())))
          });
      }
      g.finish();
  }
  ```
  - `clone()` the inputs inside the closure only if `union` consumes them
    (it takes `Geometry` by value — yes). Clone cost is incurred by both
    Rust and the JS side equivalently (JS passes fresh arrays too), but to
    isolate the algorithm, prefer an API that borrows, or subtract a
    clone-only baseline. Document the choice.
  - `asia_union` is large/slow → set `sample_size(10..20)` for that group
    so the run finishes in reasonable wall-clock.
  - `clip_path_flatten`: a separate group replaying the recorded steps.
- **Output:** `cargo bench` writes criterion reports to
  `target/criterion/`; the HTML is gitignored (already covered by
  `/target`).

### 5.2 Node comparison harness — `martinez@0.8.1` vs `polyops` napi

Goal: the three numbers PLAN.md §11 calls for, per scenario:
1. `martinez-polygon-clipping@0.8.1` (Node, single-thread)
2. `polyops` pure-Rust (from 5.1)
3. `polyops` via napi (Node calling Rust)

- **Where:** extend the existing `parity/` Node project (it already has
  `martinez@0.8.1` and `tsx`). Add `parity/bench.ts` + a `"bench"` script.
- **Runner:** `tinybench` (tiny, no vitest harness needed) or vitest
  `bench`. tinybench keeps it dependency-light.
- **napi binding:** import the built loader at
  `../crates/polyops-napi/index.js` (same path `verify-via-napi.ts`
  uses); requires `cd crates/polyops-napi && npm ci && npm run build`
  first. Document this prereq.
- **Same fixtures, same access patterns** as §3, so the JS and Rust
  numbers are comparable apples-to-apples.
- **Report:** print an `ops/sec` table; the **(3) ÷ (1)** ratio is the
  user-facing speedup, **(2) vs (3)** gap is napi marshalling overhead.

### 5.3 Methodology guardrails
- **Single-thread** everywhere (no rayon); matches the Lambda model.
- **Release build** for the Rust crate (`cargo bench` is release by
  default) and `napi build --release` for the binding.
- Parse/IO **outside** the measured region on both sides.
- Warm up; report median + MAD (criterion does this; configure tinybench
  warmup iterations to match roughly).
- Pin the machine state in the writeup (CPU, OS, Node/rustc versions) —
  numbers are only comparable within a run, not across machines.

---

## 6. Reporting & CI

- **`BENCHMARKS.md`** at the repo root: the scenario table, the three
  numbers per scenario, the speedup ratios, machine/versions, and the
  date. Re-generate when the algorithm changes materially.
- **README**: a short "Performance" section with the headline ratios +
  link to `BENCHMARKS.md` (and resolves the stale "pre-alpha" status line
  while we're there).
- **CI:** do **not** gate merges on benchmark numbers — microbench timing
  is too noisy on shared runners. Optionally add a *non-gating*
  `cargo bench -- --test` smoke (criterion's `--test` mode just runs each
  bench once to catch compile/panic regressions). Keep the real numbers a
  manual, documented run.

---

## 7. Optimization backlog (AFTER the baseline — not part of M7)

From PLAN.md §11, the order to pursue *only if* the baseline shows
`polyops` short of target (it likely already beats JS before any of this):
1. Pre-size collections (`Vec::with_capacity`).
2. `smallvec` / inline buffers for short-lived per-event arrays.
3. Swap `BTreeSet`/sorted-`Vec` sweep status for `splay_tree` *only* if
   profiling fingers it.
4. `robust` predicate `f64` fast-path with escalation to adaptive only
   for borderline orientations.
5. SIMD orientation predicates in the `possible_intersection` inner loop.

Each is its own PR with a before/after `BENCHMARKS.md` delta.

---

## 8. Risks

- **napi marshalling dominates small inputs.** For `hole_hole` the
  array-copy across the N-API boundary may exceed the compute, making
  path (3) look bad vs (1). Expected; call it out — the win shows on the
  large/real scenarios (`asia`, `clip_path_flatten`).
- **`clip_path_flatten` requires `process-photo`.** If unavailable, ship
  the three upstream scenarios and backfill (note the gap, don't hide it).
- **`asia.geojson` (1.2 MB) in git.** One-time, ~never changes; acceptable.
- **Bench variance on CI.** Mitigated by keeping numbers a manual run and
  CI doing only a `--test` smoke.
- **Clone cost skew.** `union` takes inputs by value; cloning inside the
  loop adds allocation both sides — document, and optionally measure a
  clone-only baseline to subtract.

---

## 9. Step-by-step task list

1. **Vendor fixtures** → `crates/polyops/benches/fixtures/` (3 upstream
   files) + `benches/fixtures/README.md` (origin/license) + a refresh
   script in `parity/`. *DoD: files committed; refresh script runs.*
2. **Rust criterion suite** → rewrite `benches/benchmarks.rs`; add
   `criterion` dev-dep; shared `coords→Geometry` loader. *DoD:
   `cargo bench` runs all three upstream scenarios and prints numbers.*
3. **Node comparison harness** → `parity/bench.ts` + `"bench"` script
   (tinybench; martinez vs napi). *DoD: `npm run bench` prints the
   three-number table.*
4. **`clip_path_flatten`** → instrument `process-photo` `test39`, capture
   the step sequence, vendor it, add the replay bench (Rust + Node).
   *DoD: fourth scenario runs on both sides.*
5. **Write up** → `BENCHMARKS.md` + README "Performance" section (+ fix
   the stale status line). *DoD: committed with real numbers + machine
   info.*
6. **(Optional) CI smoke** → non-gating `cargo bench -- --test` job.
   *DoD: catches bench compile/panic regressions without timing gates.*

Milestone 7 is complete after steps 1–5 (step 4 may slip to a follow-up if
`process-photo` is unavailable; note it explicitly if so).
