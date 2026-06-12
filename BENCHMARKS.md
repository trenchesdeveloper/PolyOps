# Benchmarks

Baseline for Milestone 7 (PLAN.md §11). Three `union` workloads mirroring
upstream `bench/martinez.bench.ts`, run three ways:

1. **martinez** — `martinez-polygon-clipping@0.8.1` (Node, in-process JS)
2. **polyops (Rust)** — the pure-Rust crate via `criterion` (`cargo bench`)
3. **polyops-napi** — the same Rust engine called from Node through the
   N-API binding (`parity/bench.ts`)

> Numbers are a snapshot from one machine, single run — comparable **within
> this table**, not across machines. Re-measure on your own hardware before
> quoting.

**Environment:** Apple M1 Pro · macOS 26.5.1 · rustc 1.95.0 · Node v26.0.0 ·
single-thread · release builds. Measured 2026-06-12.

## Results (ops/sec — higher is faster)

| Scenario | Input | martinez@0.8.1 | polyops (Rust) | polyops-napi |
|----------|-------|---------------:|---------------:|-------------:|
| `hole_hole`   | tiny, degenerate    | 68,150 | **125,219** (1.84×) | 45,887 (0.67×) |
| `states_clip` | ~92 KB, multi-poly  | 419    | **940** (2.24×)     | 567 (1.35×)    |
| `asia_union`  | ~1.2 MB subject     | 20.6   | **60** (2.91×)      | 33.1 (1.61×)   |

Ratios in parentheses are **vs martinez** (>1 = faster than martinez).

## Reading the numbers

- **The Rust engine is solidly faster than martinez** — 1.8×–2.9× across
  the three workloads, widening with size. Milestone 7's bar ("PolyOps,
  Rust single-thread, matches or beats martinez@0.8.1") is **met on all
  three.**
- **The N-API boundary is the tax.** Marshalling GeoJSON arrays in and out
  across N-API costs real time:
  - `hole_hole`: Rust does the compute in ~8 µs, but the round-trip
    marshalling adds ~14 µs, so `polyops-napi` (45.9k ops/s) lands **below**
    in-process martinez (68.2k ops/s) → **0.67×**. For tiny inputs, the
    boundary dominates and JS-in-process wins.
  - `asia_union`: copying the ~1.2 MB array across the boundary roughly
    doubles wall-time (Rust 16.7 ms → napi 30.2 ms), yet still **1.61×**
    faster than martinez.
  - `states_clip`: **1.35×** via napi (Rust 1.06 ms → napi 1.76 ms).
- **Takeaway:** the win is real for medium/large polygon work; for very
  small ops called in a tight JS loop, the boundary cost can erase it. The
  biggest optimization lever is **reducing marshalling** (see below), not
  the algorithm.

## What this means for process-photo

The decisive workload — `clip_path_flatten`, the real SVG clip-path
flattening from `process-photo` — is **not yet measured** (PERFORMANCE_PLAN
§4; needs the `process-photo` repo to extract the input sequence). Whether
the napi win holds there depends on the polygon sizes and how many ops run
per call: large clip paths favor polyops; many tiny ops in a tight loop are
where the boundary cost bites. Measure before integrating (PLAN.md §12).

## Optimization backlog (not yet done)

Ordered by expected impact for the napi consumer (PERFORMANCE_PLAN §7):

1. **Cut N-API marshalling** — the dominant cost for the binding. Accept
   `Float64Array`/typed arrays or a flat coordinate buffer instead of
   nested `number[][][]`, to avoid the deep per-coordinate copy.
2. Pre-size collections (`Vec::with_capacity`).
3. `smallvec` for short-lived per-event arrays.
4. `splay_tree` sweep status (only if profiling fingers it).
5. `robust` predicate `f64` fast-path with adaptive escalation.
6. SIMD orientation predicates in the inner loop.

Each lands as its own PR with a before/after delta in this file.

## Reproduce

```bash
# (2) pure-Rust:
cargo bench -p polyops

# (1) + (3) head-to-head (build the binding first):
cd crates/polyops-napi && npm ci && npm run build && cd -
cd parity && npm install && npm run bench
```

Fixtures live in `crates/polyops/benches/fixtures/` (vendored from upstream;
refresh with `cd parity && npm run copy-bench-fixtures`).
