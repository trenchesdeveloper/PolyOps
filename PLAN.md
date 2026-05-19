# PolyOps — Implementation Plan

> **Status:** Living document. Edit as decisions evolve.
> **Owner:** Samuel Opeyemi.
> **Repo:** https://github.com/trenchesdeveloper/PolyOps
> **Last updated:** 2026-05-19

---

## 1. Mission

Ship a fast, correct, faithfully-behaving Rust port of
[`martinez-polygon-clipping`](https://github.com/w8r/martinez), published
as:

- `polyops` on **crates.io** for Rust consumers.
- `polyops` on **npm** (via `napi-rs`) as a drop-in replacement for
  `martinez-polygon-clipping@0.8.1`.

The first concrete consumer is the **`process-photo`** Lambda service,
which today calls `martinez-polygon-clipping` from `src/utils/svg.ts`
during clip-path flattening and is the workload we're optimizing for.

---

## 2. Background

### Why a port at all

Our `process-photo` spends meaningful CPU time on polygon Boolean operations
during SVG clip-path flattening and vectorization. Profiling indicates
the JS Martinez implementation is one of the hot kernels. A Rust port
removes GC overhead, enables tighter data structures, and unlocks
SIMD-friendly code paths over time.

### Why martinez specifically

Several Rust polygon-clipping crates exist (`i_overlay`, `geo-clipper`,
`clipper2`), but each implements a different algorithm with different
numerical behavior. `process-photo`'s existing test fixtures encode
behavior produced by `martinez-polygon-clipping`. The lowest-risk path
to using Rust in production is to match that behavior exactly. Once we
have parity, swapping algorithms or moving to `i_overlay` later is a
separate, easier conversation.

### What already exists in Rust

`geo-booleanop` (`21re/rust-geo-booleanop`, last commit 2023, last
release 2020, ~3,200 LOC) is a faithful Rust port of `w8r/martinez` as
of ~v0.7. **Unmaintained**, but valuable as a *reference* — we consult
it when stuck on algorithm details rather than forking it. The 2020
freeze means it's missing several upstream bug fixes that landed in
`0.8.x`, which is why we don't fork.

The crate name `martinez` on crates.io is **available**. We deliberately
chose `polyops` instead so the name doesn't claim the upstream's
identity and leaves room for non-Martinez algorithms in the future.

### What does not exist in Rust

No `napi-rs` polygon-clipping package on npm. This means PolyOps is the
first native-Rust polygon-clipping engine published to the JS
ecosystem.

---

## 3. Goals & Non-Goals

### Goals

1. **Behavioral parity** with `martinez-polygon-clipping@0.8.1` over the
   complete upstream test suite (36 generic test cases + 16 feature-type
   cases = ~200 parity goldens once expanded across the four ops).
2. **Drop-in npm replacement.** The four exported functions
   (`intersection`, `union`, `diff`, `xor`) accept the same GeoJSON-shaped
   coordinate arrays and return the same shape.
3. **Real-world performance gain.** When swapped into `process-photo`'s
   SVG pipeline, the polygon-Boolean phase runs measurably faster (target:
   ≥3× on representative SVG fixtures from
   `test/process-photo-v3/assets/`).
4. **Published artifacts.** `polyops` on crates.io and `polyops` on npm,
   with platform prebuilts covering the matrix the team uses (Apple
   Silicon, Apple x86, Linux x64 GNU + musl, Linux ARM64 GNU + musl,
   Windows x64).
5. **CI-gated correctness.** Every PR runs unit tests, clippy, fmt, and
   the parity suite.

### Non-goals (for v1.x)

- Algorithmic improvements beyond what upstream does. We port; we don't
  redesign. (Optimization is in scope **after** parity is locked.)
- Output shapes that upstream doesn't produce (GeoJSON Features, properties
  passthrough, etc.). Coords in, coords out.
- A WebAssembly build. Tracked as future work in §13; not part of v1.
- A Leaflet demo. Tracked as future work; not part of v1.
- Streaming/incremental APIs.
- 3D, lat/lon-specific math, or non-planar geometry.

---

## 4. Constraints

- **MIT license** for the project; upstream is MIT, the algorithm paper
  is freely citable. LICENSE credits Milevski and the Martinez/Rueda/Feito
  paper.
- **Rust edition 2021**, MSRV **1.80**. (Set above napi-build's 1.77
  requirement with headroom; revisit if a dependency forces it up.)
- **Node ≥ 18** for the npm package.
- **No unsafe** in the core crate. (`unsafe` is fine inside `napi-rs`
  generated code; we don't add our own.)
- **Public Rust API must be `serde`-free.** Boundary conversions live
  in `polyops-napi`. This keeps the core crate light for non-Node
  consumers.

---

## 5. Architecture

The project is a Cargo workspace with two crates and a sibling Node
project for parity testing.

```
PolyOps/
├── crates/
│   ├── polyops/                pure-Rust algorithm + types  → crates.io
│   └── polyops-napi/           napi-rs binding              → npm
├── parity/                     Node harness: runs upstream, emits goldens
├── .github/workflows/          ci.yml (today), release.yml (v0.1)
├── Cargo.toml                  workspace
├── PLAN.md                     this file
├── README.md
└── LICENSE
```

### `crates/polyops` — the core

Pure Rust, no I/O, no JSON. Public surface is four free functions and a
handful of types. Module layout mirrors `w8r/martinez/src/` 1:1 so
cross-referencing during the port is mechanical:

| Module                     | Upstream file                       | Role                                      |
|----------------------------|-------------------------------------|-------------------------------------------|
| `types`                    | `src/types.ts`                      | Position, Ring, Polygon, MultiPolygon, BBox, Geometry |
| `operation`                | `src/operation.ts`                  | `enum Operation { Intersection, Union, Difference, Xor }` |
| `signed_area`              | `src/signed_area.ts`                | Signed area of three points (predicate) |
| `equals`                   | `src/equals.ts`                     | Point equality with tolerance |
| `edge_type`                | `src/edge_type.ts`                  | `enum EdgeType { Normal, NonContributing, SameTransition, DifferentTransition }` |
| `sweep_event`              | `src/sweep_event.ts`                | Sweep-event node (arena-backed) |
| `compare_events`           | `src/compare_events.ts`             | Event-queue ordering |
| `compare_segments`         | `src/compare_segments.ts`           | Sweep-line status ordering |
| `segment_intersection`     | `src/segment_intersection.ts`       | Geometric kernel: segment×segment |
| `divide_segment`           | `src/divide_segment.ts`             | Split a segment at an intersection point |
| `possible_intersection`    | `src/possible_intersection.ts`      | Sweep-time intersection dispatcher |
| `compute_fields`           | `src/compute_fields.ts`             | In/out flags, edge classification |
| `fill_queue`               | `src/fill_queue.ts`                 | Initial event queue from input polygons |
| `subdivide_segments`       | `src/subdivide_segments.ts`         | Main sweep loop |
| `contour`                  | `src/contour.ts`                    | Output contour representation |
| `connect_edges`            | `src/connect_edges.ts`              | Stitch result events into contours |
| `lib.rs::boolean_op`       | `src/index.ts`                      | Top-level driver + trivial-case shortcuts |

### `crates/polyops-napi` — the Node binding

Thin adapter layer. Translates GeoJSON-shaped arrays (`Vec<Vec<Vec<f64>>>`
and `Vec<Vec<Vec<Vec<f64>>>>`) to and from the core crate's types.
Crate type is `["cdylib", "rlib"]` so `cargo test` works on all
platforms; `napi build` still produces the cdylib for npm. No business
logic lives here.

### `parity/` — the correctness gate

Node project. Installs `martinez-polygon-clipping@0.8.1`. Walks the
upstream's own test fixtures and writes one golden JSON file per
(operation, fixture) pair to `crates/polyops/tests/goldens/`. The Rust
test runner at `crates/polyops/tests/parity.rs` reads those files and
asserts coordinate-wise equality within `1e-10`.

The harness is **not** required to run on every developer machine; the
goldens are committed. The harness exists so we can:

1. Refresh goldens when upstream tags a new patch release.
2. Confirm the goldens we committed match what upstream produces today.
3. Bootstrap the project for a new contributor.

---

## 6. Public API

### Rust

```rust
pub fn intersection(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon>;
pub fn union       (subject: Geometry, clipping: Geometry) -> Option<MultiPolygon>;
pub fn difference  (subject: Geometry, clipping: Geometry) -> Option<MultiPolygon>;
pub fn xor         (subject: Geometry, clipping: Geometry) -> Option<MultiPolygon>;

pub enum Geometry { Polygon(Polygon), MultiPolygon(MultiPolygon) }
pub type Position    = [f64; 2];
pub type Ring        = Vec<Position>;
pub type Polygon     = Vec<Ring>;
pub type MultiPolygon = Vec<Polygon>;
pub type BBox        = [f64; 4];   // [min_x, min_y, max_x, max_y]
```

`None` is returned exactly when upstream returns `null` (empty/disjoint
result for some operations).

### TypeScript / npm

```ts
type Position    = [number, number];
type Ring        = Position[];
type Polygon     = Ring[];
type MultiPolygon = Polygon[];

export function intersection(
    subject: Polygon | MultiPolygon,
    clipping: Polygon | MultiPolygon,
): MultiPolygon | null;
// same signature for union, diff, xor
```

The exported name `diff` (not `difference`) matches upstream's npm API
exactly. The Rust crate uses `difference` because it's the unambiguous
Rust convention.

---

## 7. Implementation plan, file by file

Order is bottom-up by dependency: leaves first, drivers last. Each item
notes what the file does, what's tricky about it, and what we expect
the Rust shape to look like.

### 7.1 `signed_area` (45 LOC upstream)

Two-argument cross product implementing the standard "signed area of
triangle (p0, p1, p2)" predicate. Used by `compare_events`,
`compare_segments`, ring-orientation checks, and intersection math.

**Port note.** Upstream calls into `robust-predicates` (Shewchuk's
adaptive arithmetic) for the actual computation. We'll do the same via
the `robust` crate's `orient2d`. A non-robust `f64`-only fallback can
exist behind a `#[cfg(feature = "no-robust")]` for benchmarking, but
robust is the default — every parity failure on the degenerate fixtures
(`collapsed`, `crash_overlap`, `overlapping_segments_complex`) traces back
to this choice.

**Definition of done.** Unit tests at the same coverage as upstream's
`test/signed_area.test.ts`; parity test against a few hand-picked
collinear and near-collinear cases.

### 7.2 `equals` (21 LOC upstream)

Float-tolerant point equality. Upstream uses an exact-equal check on
both coordinates after the algorithm has normalized them through the
sweep. We mirror exactly; no fancy epsilon comparison.

**Port note.** Trivial. The reason it's its own module upstream is for
test isolation and consistency of comparison semantics.

### 7.3 `edge_type` (6 LOC upstream)

Four-variant enum. Direct Rust translation. Already stubbed in the
scaffold. No real porting work.

### 7.4 `sweep_event` (100 LOC upstream)

The JS implementation uses linked nodes with mutable `otherEvent`
references and stores them in a splay tree. In Rust we use an arena:
`Vec<SweepEvent>` owned by the sweep driver, and `usize` indices in
place of pointer references. This sidesteps `Rc<RefCell<_>>` and gives
us cache-friendly storage.

**Fields to carry** (from the scaffold stub): `point`, `left`,
`other_event` (index), `polygon_type`, `edge_type`, `in_out`,
`other_in_out`, `prev_in_result`, `result_transition`,
`output_contour_id`, `is_exterior_ring`.

**Port note.** The arena pattern leaks one detail upward: every helper
that traverses events needs a borrow of the arena. The cleanest shape is
a `SweepCtx` struct that owns the arena plus the queue plus the status
tree, and is passed to every sweep-phase function. We'll converge on
that organically; it shouldn't be designed up front.

### 7.5 `compare_events` (45 LOC upstream) & `compare_segments` (47 LOC upstream)

Two `Ord`/comparison functions, one for the event queue (priority by
point x, then point y, then left-before-right, then signed area
tie-break), one for the sweep-line status (vertical order along the
sweep). Both call `signed_area`.

**Port note.** In Rust, the event queue is a `BinaryHeap` (max-heap, so
we wrap items in `Reverse`) and the status tree is a `BTreeSet` keyed
by a `SegmentKey { event_idx: usize }` that implements `Ord` via the
comparator. Care: `BinaryHeap` does **not** guarantee FIFO order on
equal keys, but the Martinez comparators should be total enough that we
never compare-equal in practice. If a parity fixture exposes a
tie-breaking divergence we'll add an explicit insertion-order tiebreak.

### 7.6 `segment_intersection` (151 LOC upstream)

Geometric kernel: returns one of `None | Point | Overlap` for two
segments. The collinear-overlap branch is where most parity bugs hide.

**Port note.** Direct line-by-line port. Use `robust::orient2d` for the
orientation predicates inside; everything else is `f64`. The upstream
`no_endpoint_touch` flag has to be preserved exactly — it's a subtle
correctness signal for the `possible_intersection` dispatcher.

### 7.7 `divide_segment` (39 LOC upstream)

When two segments intersect, split each at the intersection point by
creating two new sweep events and patching the `otherEvent` links.

**Port note.** Pure arena manipulation. Watch for the case where the
intersection point is exactly the endpoint of one or both segments —
upstream short-circuits this; we must too.

### 7.8 `compute_fields` (110 LOC upstream)

Per-event, decide `inOut`, `otherInOut`, and `edgeType` based on the
previous event on the sweep line. This is the file that encodes the
operation-specific logic: union vs intersection vs difference vs xor
all flow through here.

**Port note.** Straightforward branching. Tests should cover each
operation independently. Upstream's `test/compute_fields.test.ts` is
sparse (8 LOC); we'll write more.

### 7.9 `fill_queue` (111 LOC upstream)

Initial event-queue population. Walks every ring of every polygon of
both inputs, creates two `SweepEvent`s per segment, pushes them into
the queue, and accumulates per-input bounding boxes.

**Port note.** Important to preserve upstream's exact handling of:
ring closure (first point repeated at the end), degenerate two-point
rings, zero-area segments. Each of these has at least one fixture in
`test/genericTestCases/`.

### 7.10 `possible_intersection` (122 LOC upstream)

The dispatcher invoked during the sweep when two segments become
neighbors on the sweep line. Calls `segment_intersection`, then handles
four cases: no intersection, single-point, shared-endpoint overlap,
full overlap. Each case may call `divide_segment` zero or more times
and may mark events as non-contributing.

**Port note.** Algorithmically the densest file. Port it last among the
leaves. The overlap branch in particular has subtle interactions with
`compute_fields` — a wrong flag here produces silent parity failures
far downstream in `connect_edges`. Write the parity test runner so it
prints which fixture failed and which operation, not just a count.

### 7.11 `subdivide_segments` (87 LOC upstream)

The main sweep loop. Pulls events off the queue, maintains the
sweep-line status, calls `possible_intersection` on neighbors,
collects the result events.

**Port note.** This is mostly orchestration. The hardest part is
remembering to remove events from the status tree at the right moment;
upstream uses pointer-equality which we replace with arena-index
equality.

### 7.12 `contour` (25 LOC upstream)

Plain data: `points`, `holeIds`, `holeOf`, `depth`, `isExterior()`.
Already stubbed.

### 7.13 `connect_edges` (187 LOC upstream)

Largest upstream file. Walks the sorted result events, stitches them
into closed contours, then performs a hole-ownership analysis to attach
each hole to its enclosing exterior ring.

**Port note.** The data-structure choices in upstream are very
JS-flavored (sparse arrays, in-place mutation of indexes). Resist
porting them literally; use proper `Vec<usize>` index lookups and a
union-find-ish structure for the hole-of relationships. But port
correctness first — clean up only after the file passes parity.

### 7.14 `lib.rs::boolean_op` (113 LOC upstream)

Top-level driver. Trivial-case shortcuts (empty operand, disjoint
bboxes), then calls `fill_queue` → `subdivide_segments` →
`connect_edges`, then assembles the result polygons.

**Port note.** Largely mechanical once the pieces underneath work.

---

## 8. Data-structure decisions (committed and provisional)

| Concern               | JS upstream                   | PolyOps choice               | Status      | Fallback if parity fails                |
|-----------------------|-------------------------------|------------------------------|-------------|------------------------------------------|
| Event arena           | linked nodes + GC             | `Vec<SweepEvent>` + indices | Committed   | n/a                                       |
| Event queue           | `tinyqueue` (binary heap)     | `BinaryHeap` + `Reverse`     | Committed   | Add insertion-order tiebreak               |
| Sweep-line status     | `splaytree`                   | `BTreeSet` keyed by comparator | Provisional | `splay_tree` crate                         |
| Orientation predicate | `robust-predicates`           | `robust::orient2d`           | Committed   | Plain `f64` (will fail degenerate fixtures) |
| Hole-of detection     | Sparse JS array + DFS         | `Vec<Option<usize>>` + DFS  | Provisional | Reconsider after `connect_edges` lands     |
| Allocations           | Implicit JS                   | Pre-size all Vecs where input size is known | Committed | n/a |

---

## 9. Parity test strategy

### Goldens

`parity/generate-goldens.ts` reads from a local clone of `w8r/martinez`
(default `../martinez`, overridable via `MARTINEZ_REPO`). For each file
in `test/genericTestCases/`, it loads the GeoJSON, extracts
`features[0]` as subject and `features[1]` as clipping, then runs
upstream's four operations and writes one golden per (op, fixture)
pair. It additionally processes the 4×4 `featureTypes` matrix.

Each golden file has the shape:

```json
{ "subject": <coords>, "clipping": <coords>, "expected": <multipolygon|null> }
```

### Rust runner

`crates/polyops/tests/parity.rs` walks
`crates/polyops/tests/goldens/{intersection,union,difference,xor}/`,
loads each golden, calls the corresponding Rust function, and compares
coordinate-wise with `eps = 1e-10`. All four tests are `#[ignore]`d
until the algorithm is implemented; drop the attribute per-operation as
each goes green.

### Comparison tolerance

`1e-10` is intentionally tight. The algorithm is deterministic; the
only legitimate source of divergence is floating-point summation order,
which `robust::orient2d` standardizes. If a parity check fails at this
tolerance, it's a real bug, not a numerical artifact.

### CI gating

The `parity` CI job runs the harness against the upstream clone and
executes `cargo test -- --include-ignored`. While the algorithm is
stubbed it's gated `continue-on-error: true`. The moment `parity_xor`
is the last red test, flip the flag — parity becomes a merge gate from
that PR onward.

### Reproducing failures

When a parity test fails, the runner prints the fixture name, the
expected coords, and the actual coords. Standard debugging loop:

1. Open `~/Desktop/rust-learn/martinez/test/genericTestCases/<name>.geojson`
   to see the input visually.
2. Run upstream's own vitest in the martinez clone with the same
   fixture isolated, with `console.log` instrumentation in the suspected
   upstream function.
3. Add equivalent `eprintln!` in the Rust port, line them up.
4. Find where they diverge.
5. Add a focused Rust unit test that reproduces the divergence in
   isolation, fix it, then re-run parity.

---

## 10. Distribution

### npm package shape (`polyops`)

Standard `napi-rs` 2.x layout. The main package `polyops` has a JS
loader (`index.js`) and types (`index.d.ts`), plus
`optionalDependencies` listing seven platform sub-packages. Consumers
`npm install polyops`; npm resolves the matching prebuilt
automatically.

Platform matrix:

```
@polyops/polyops-darwin-arm64
@polyops/polyops-darwin-x64
@polyops/polyops-linux-x64-gnu
@polyops/polyops-linux-x64-musl
@polyops/polyops-linux-arm64-gnu
@polyops/polyops-linux-arm64-musl
@polyops/polyops-win32-x64-msvc
```

The `linux-x64-musl` and `linux-arm64-musl` builds are critical because
Alpine-based Docker images need them — `process-photo`'s Dockerfile
may want to switch. The `linux-x64-gnu` build is the path of least
resistance for the Lambda image as it stands today.

### crates.io publish

The `polyops` crate publishes the pure-Rust library only. The
`polyops-napi` crate has `publish = false` and never goes to crates.io
— it's an npm-only artifact.

### Versioning

`0.0.x` for pre-alpha while the algorithm is being filled in.
`0.8.x` for the first parity-locked releases (matching upstream's
current major). Bump in lockstep with upstream when reproducing
upstream patches. Independent patch versions are fine for Rust-only
improvements (perf, docs) that don't change behavior.

### Release workflow

Not yet scaffolded; add when v0.1 is in sight. The shape:

1. Tag `vX.Y.Z` on `main`.
2. GitHub Actions matrix builds all seven napi prebuilts.
3. `napi prepublish` rewrites package.jsons for each platform sub-package.
4. `npm publish` publishes the main package and all sub-packages.
5. Same workflow runs `cargo publish` for the `polyops` crate.

NPM_TOKEN and CARGO_REGISTRY_TOKEN as repo secrets.

---

## 11. Performance plan

### What we measure

Three benchmark scenarios, mirroring upstream's `bench/martinez.bench.ts`:

- **`hole_hole`** — small, many degeneracies; tests the hot kernel.
- **`asia_union`** — large polygons (~tens of thousands of vertices);
  tests sweep-line scaling.
- **`states_clip`** — many polygons (US states-like); tests the
  multi-polygon path.

Plus one PolyOps-specific scenario seeded from `process-photo`:

- **`clip_path_flatten`** — a representative SVG clip-path tree
  extracted from `process-photo/test/process-photo-v3/assets/test39/`
  (the 3-level nested clip-path case from FE-1866), reduced to its
  polygon-Boolean inputs.

### What we compare

For each scenario, three numbers:

1. `martinez-polygon-clipping@0.8.1` (Node, single-thread).
2. `polyops` (Rust, single-thread).
3. `polyops` via napi-rs (Node calling Rust).

The (3) - (1) ratio is the user-facing speedup. The (2) - (3) gap is
the napi marshalling overhead — useful to know but not the target.

### When we measure

Not before parity is green on the corresponding fixture. Optimizing
before correctness is locked is how parity regressions ship.

### Optimization order (after parity)

1. **Pre-size collections** — replace `Vec::new()` + `push` with
   `Vec::with_capacity(estimated)`. Cheap, high-impact.
2. **Smallvec / inline buffers** for short-lived per-event arrays.
3. **Replace `BTreeSet` with `splay_tree`** *only* if profiling shows
   the sweep-line status as the bottleneck.
4. **`robust` predicates** are slow by design; consider falling back to
   `f64` for the obviously-non-degenerate case (orientation magnitude
   above a threshold) and only escalating to robust for borderline cases.
5. **SIMD orientation predicates** for the inner loops of
   `possible_intersection`. Only worth pursuing if (1–4) leave performance
   short of the 3× target.

---

## 12. Integration with process-photo

Once `polyops@0.1.0` is published, the integration into
`~/Desktop/works/process-photo` is one PR:

1. `npm install polyops` — pulls the napi binding.
2. In `src/utils/svg.ts`, replace
   ```ts
   import martinez from 'martinez-polygon-clipping';
   ```
   with
   ```ts
   import * as polyops from 'polyops';
   ```
   (call sites use `martinez.intersection`, `martinez.union`, etc., so
   they map 1:1 to `polyops.intersection`, etc.)
3. Run the existing `test/process-photo-v3/` suite to confirm SVG
   output is byte-identical.
4. Update `package.json` to drop `martinez-polygon-clipping` from
   `dependencies`.
5. Confirm the Lambda's Docker image still builds — `polyops` adds
   ~few MB for the native binary; the existing `Dockerfile-process-photo-v3`
   pattern already handles native deps fine.
6. Measure end-to-end Lambda execution time on three representative
   inputs before and after; expected ≥1.5× speedup on photos where the
   clip-path flattening dominates.

If for any reason `process-photo`'s tests fail after the swap, that's a
parity bug in `polyops`, not a `process-photo` bug. The fix lives
upstream in PolyOps.

---

## 13. Milestones & definition of done

### Milestone 0 — Scaffold ✅

- Workspace, crate skeletons, parity harness scaffold, CI workflow,
  license, README, this plan.

### Milestone 1 — Parity harness produces goldens

- `cd parity && npm install && npm run generate` runs end-to-end.
- `crates/polyops/tests/goldens/` has files in all four operation
  directories.
- `cargo test --workspace -- --ignored` runs without compile errors
  (tests will fail — algorithm not implemented yet).

### Milestone 2 — Leaves are real Rust

- `signed_area`, `equals`, `compare_events`, `compare_segments`,
  `sweep_event` no longer contain `todo!()`.
- Unit tests for each at the coverage of upstream's matching
  `test/*.test.ts`.

### Milestone 3 — Kernel passes its own tests

- `segment_intersection`, `divide_segment`, `compute_fields`
  implemented with passing unit tests.
- The parity test runner can call `polyops::intersection` etc. without
  panicking (returns wrong answers — that's fine for now).

### Milestone 4 — First operation greens up

- One of the four ops (start with `intersection` — usually the
  simplest in Martinez) passes parity on the simplest fixtures
  (`disjoint_boxes`, `one_inside`, `two_triangles`).
- `#[ignore]` removed from `parity_intersection`.

### Milestone 5 — All four operations pass simple fixtures

- All four `parity_*` tests pass on the simple subset; `#[ignore]`
  removed everywhere.
- The complex fixtures (`asia`, `canada`, `crash_overlap`,
  `overlapping_segments_complex`) still fail. That's expected.

### Milestone 6 — Full parity

- 100% of generated goldens pass.
- CI parity job's `continue-on-error: true` is removed.

### Milestone 7 — Performance baseline

- Benchmarks land. PolyOps (Rust, single-thread) matches or beats
  martinez-polygon-clipping@0.8.1 on the three upstream scenarios and
  the `clip_path_flatten` scenario.

### Milestone 8 — First public release

- `polyops 0.8.0` publishes to crates.io.
- `polyops 0.8.0` publishes to npm with all seven prebuilts.
- README polished, badges added, CHANGELOG started.

### Milestone 9 — `process-photo` integration

- PR in `process-photo` swaps `martinez-polygon-clipping` for
  `polyops`. Test suite green. Lambda built and deployed.
- Production metrics confirm the speedup or surface a regression to
  chase.

### Definition of done for the project as a whole

- Milestones 0–9 complete.
- Documentation in the README and rustdoc covers the public API.
- A short `CONTRIBUTING.md` explains how to run the parity harness, the
  test structure, and how to add a new fixture.

---

## 14. Risks & open questions

### Known risks

- **Robust-predicate divergence.** The `robust` Rust crate is a port of
  Shewchuk's predicates, same as the JS dep. If their implementations
  drift in obscure ways, we'll see divergence on the most degenerate
  fixtures (`crash_overlap`, `overlapping_segments_complex`). Mitigation:
  port `signed_area` with both `robust::orient2d` and a hand-translation
  of the JS code, and run both against the goldens to identify which
  produces parity. Pick that one.

- **Splay tree vs BTreeSet ordering.** A splay tree exposes "most
  recently accessed" as a side-effect of normal operations; a BTreeSet
  doesn't. Upstream Martinez doesn't *depend* on this for correctness,
  but the order of equal-key resolution in the sweep status could
  differ. Mitigation: try BTreeSet first; fall back to the `splay_tree`
  crate if a fixture insists.

- **`tinyqueue` FIFO vs `BinaryHeap` arbitrary.** Equal-priority events
  pop in insertion order from `tinyqueue` and in unspecified order from
  `BinaryHeap`. Mitigation: extend the event-comparison key with a
  monotonic `insertion_sequence: u64` so equal keys never actually
  compare equal.

- **Upstream changes during port.** If Milevski tags `0.9.0` while
  we're porting, we have to decide whether to chase. Mitigation: lock
  to `0.8.1` for v0.1; track upstream `master` informally; bump after
  v0.1 ships.

### Open questions

- **Should we offer a streaming API?** Some upstream consumers process
  thousands of polygon pairs sequentially. A streaming API that reuses
  the arena across calls could be much faster. Defer to v1.x.

- **Should we ship a `wasm-bindgen` build?** It would let browser
  consumers use PolyOps the same way they use the existing JS
  martinez. Adds CI complexity. Defer; can be added later without
  breaking changes.

- **Should we expose `Operation` as a public type?** Upstream does
  (`{ INTERSECTION: 0, UNION: 1, ... }`). Some consumers `switch` on
  it. Probably yes for symmetry, but it's a v0.2 concern.

- **Optional features for the `polyops` crate.** Candidates: `serde`
  for `Geometry` (so Rust consumers can deserialize GeoJSON without
  going through `polyops-napi`), `geo-types` integration. Probably yes,
  behind feature flags, in v0.2.

### Things to revisit at each milestone

- Is the parity gate still the right correctness model, or should we
  add property tests (e.g., union should be commutative, double
  intersection should be idempotent)? Property tests catch real bugs
  that fixtures miss.

---

## 15. Decision log

Each entry: date, what we decided, why, and what we considered.

### 2026-05-19 — Port from scratch instead of forking `geo-booleanop`

**Decided.** Write the Rust port file-by-file, with `geo-booleanop` as
a consultation reference.

**Why.** `geo-booleanop` is faithful to upstream as of ~v0.7 (2020) and
is unmaintained. The deltas between v0.7 and v0.8.1 include real bug
fixes we need. Forking commits us to backporting those fixes through a
3,000 LOC codebase we didn't write. Writing from scratch with the
parity harness as the gate means parity is correct by construction.

**Considered.** Forking outright (Path A); a hybrid where we port the
data structures but lift the algorithms verbatim. Rejected both for the
same reason — they assume upstream behavior is captured in
`geo-booleanop`, which it isn't, fully.

### 2026-05-19 — Use `BTreeSet` for the sweep-line status, not a splay tree

**Decided.** Start with `BTreeSet` keyed by a comparator that wraps the
event index.

**Why.** `BTreeSet` is `std`, well-understood, and has stable
performance. Splay trees have great amortized bounds on locality-rich
access patterns but unpredictable worst-case latency. For the parity
phase we want simple and deterministic.

**Considered.** `splay_tree` crate, hand-rolled splay tree, AVL tree
crates. Reserved as fallback if parity fails for ordering reasons.

### 2026-05-19 — Crate names

**Decided.** `polyops` on both crates.io and npm. GitHub repo:
`PolyOps` (CamelCase per the user's preference). Internal namespace:
`polyops` (snake_case).

**Why.** `martinez-polygon-clipping` on npm is owned by upstream.
`martinez` on crates.io is available but ties our identity to the
upstream's. `polyops` is short, distinct, doesn't claim upstream's
brand, and leaves room for non-Martinez algorithms later (e.g., if we
also implement Vatti/Clipper in a sibling crate).

### 2026-05-19 — MSRV 1.80

**Decided.** Workspace `rust-version = "1.80"`.

**Why.** `napi-build` 2.x emits the `cargo::` build-script syntax
introduced in 1.77. Bumping to 1.80 gives headroom for other
dependencies (`serde`, `robust`) without immediate friction. 1.80 was
released July 2024 — well within any sensible MSRV policy.

**Considered.** 1.77 (just enough for napi-build), 1.78–1.79.

### 2026-05-19 — `polyops-napi` crate-type `["cdylib", "rlib"]`

**Decided.** Both. Default to cdylib for the npm artifact; include rlib
so `cargo test --workspace` works on Windows without a Node install on
the linker path.

**Why.** Pure cdylib forces every `cargo build` consumer to resolve
Node symbols at link time, which Windows can't do without setup-node.
Adding rlib costs nothing and removes that constraint.

---

## 16. References

- `https://github.com/w8r/martinez` — upstream JS reference
  implementation. Clone at `~/Desktop/rust-learn/martinez/`.
- `https://github.com/21re/rust-geo-booleanop` — Rust port for
  reference only. Unmaintained.
- `https://github.com/iShape-Rust/iOverlay` — alternative Rust
  polygon-clipping crate (different algorithm). Not used.
- `https://napi.rs/` — napi-rs documentation, including the dual-publish
  template that informs our `polyops-napi` package.json.
- F. Martinez, A. J. Rueda, F. R. Feito. *A new algorithm for computing
  Boolean operations on polygons.* Computers & Geosciences 35 (2009)
  1177–1185. The original paper. The 2013 extension is also relevant.
- Shewchuk, J. R. *Adaptive Precision Floating-Point Arithmetic and
  Fast Robust Geometric Predicates.* Discrete & Computational Geometry
  18 (1997) 305–363. The basis for `robust-predicates` (JS) and the
  `robust` crate (Rust).

---

## 17. Glossary

- **Sweep event** — a node in the algorithm's event queue, representing
  one endpoint of one input segment (or a segment subdivided during the
  sweep).
- **Sweep line** — a conceptual vertical line that scans left-to-right
  across the input polygons, encountering events in x-then-y order.
- **Sweep-line status** — the ordered set of segments currently
  intersected by the sweep line.
- **Robust predicate** — a geometric predicate (orientation,
  in-circle, etc.) whose sign is computed exactly using adaptive
  floating-point arithmetic, regardless of input magnitudes.
- **Multipolygon** — GeoJSON-style: a list of polygons, each of which
  is a list of rings (exterior + holes), each ring a list of
  `[x, y]` positions.
- **Hole-of relationship** — which exterior ring of the output a given
  hole belongs to. Computed by `connect_edges` after the sweep.
- **Parity** — behavioral equivalence between PolyOps and
  `martinez-polygon-clipping@0.8.1`, measured by coordinate-wise
  equality of outputs on a fixed test corpus within `1e-10`.
