# Changelog

Notable changes to `polyops` (crates.io) and `polyops` (npm). Versions move
in lockstep across both registries. Loosely follows
[Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

### Added
- **Public `Operation` enum** at the crate root (`polyops::Operation`) — was
  previously only reachable via `polyops::operation::Operation`.
- **Optional `serde` feature** (off by default) deriving `Serialize` /
  `Deserialize` on `Geometry` and `Operation`, so Rust consumers can
  (de)serialize the public types without going through `polyops-napi`. The
  core stays serde-free by default.
- **Optional `geo-types` feature** (off by default) — `From<geo_types::Polygon
  / MultiPolygon> for Geometry` (input) and a `ToGeo` extension trait
  (`.to_geo()`) on `Geometry`/`MultiPolygon` (output), for interop with the
  Rust geo ecosystem.

### Documentation
- rustdoc with runnable examples on the crate and the four operations;
  docs.rs configured to build with all features (so optional-feature items
  render). Retired the stale "pre-alpha" status note.

## [0.0.7] — 2026-06-15

### Changed
- **Discoverability / docs only — no code change.** Expanded npm keywords
  (11 → 20) and tuned the crates.io keywords; tightened the package
  descriptions; added a `vs martinez-polygon-clipping` comparison table, a
  "when to use", and an "other libraries" section to both READMEs. Republished
  so the updated keywords + README reach the registries.
- Added `ROADMAP.md` (post-0.0.6 plan).

## [0.0.6] — 2026-06-15

### Added
- **Proper ESM support (dual CJS/ESM package).** An `exports` map plus ESM
  wrappers (`polyops.mjs`, `flat.mjs`) so `import { union } from 'polyops'`
  (named), `import polyops from 'polyops'` (default), and the `polyops/flat`
  subpath all work from ESM — matching the type declarations. CommonJS
  `require('polyops')` is unchanged.
- npm package README (`crates/polyops-napi/README.md`) — fixes the
  "no README" notice on npmjs.com.

### Notes
- Non-breaking, additive. Default/named/subpath imports verified in both
  ESM and CJS against an installed tarball.

## [0.0.5] — 2026-06-14

### Added
- **Typed-array fast path, on by default.** `intersection`/`union`/`diff`/`xor`
  now route coordinates through `Float64Array`/`Uint32Array` buffers, so only
  flat data crosses the N-API boundary instead of nested `number[][][]`
  arrays. The binding is now **faster than `martinez-polygon-clipping@0.8.1`
  at every size** (~1.9× on a small clip-path intersection, ~2–2.6× on large
  unions — see [`BENCHMARKS.md`](BENCHMARKS.md)). Same drop-in, GeoJSON-shaped
  API.
- `pack` / `unpack` helpers and raw buffer ops (`intersectionFlat`,
  `unionFlat`, `diffFlat`, `xorFlat`) for pipelines that keep geometry in
  flat form across calls (skips repacking).
- `polyops/flat` subpath, kept as an alias of the default entry.
- Benchmark suite: `criterion` benches + a Node head-to-head harness vs
  martinez (Milestone 7).

### Notes
- **Non-breaking:** identical signatures and results — verified against all
  79 parity goldens. The pure-Rust crate's behavior is unchanged.

## [0.0.4] — 2026-06-12

### Fixed
- First fully-working multi-platform release. The main npm package ships its
  `index.js` loader plus all seven prebuilt platform binaries (macOS x64/arm64,
  Linux gnu/musl x64/arm64, Windows x64).

## [0.0.3] — 2026-06-12 — **broken, deprecated**
- The main npm package was published without its `index.js` loader, so
  `require('polyops')` threw. Use `>= 0.0.4`.

## [0.0.2] — 2026-06-12 — **broken, deprecated**
- The per-platform binary packages were never published, so installs found no
  native addon. Use `>= 0.0.4`.

## [0.0.1] — 2026-06-11
- First published release. Full behavioral parity with
  `martinez-polygon-clipping@0.8.1` over the upstream fixture corpus
  (intersection, union, difference, xor).
