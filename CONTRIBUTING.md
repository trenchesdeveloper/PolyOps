# Contributing to PolyOps

Thanks for your interest! PolyOps is a Rust port of
[`martinez-polygon-clipping`](https://github.com/w8r/martinez) with a
`napi-rs` Node binding. The guiding rule: **behavioral parity with
`martinez-polygon-clipping@0.8.1`** — a divergence is a bug.

## Layout

```
crates/polyops/        pure-Rust algorithm + types        → crates.io
crates/polyops-napi/   napi-rs binding (+ ESM/flat layer)  → npm
parity/                Node harness: goldens, benches, verifiers
scripts/               spin-polyops-apps.mjs: install + scale/compat tester
```

## Prerequisites

- Rust (MSRV **1.80**), Node ≥ 18 (24 recommended), and your package
  manager of choice (npm/pnpm/bun) for the binding.
- For refreshing parity goldens: a clone of
  [`w8r/martinez`](https://github.com/w8r/martinez) (default `../martinez`,
  or set `MARTINEZ_REPO`).

## Build & test

```bash
cargo test --workspace              # unit + parity tests (the core gate)
cargo fmt --all -- --check          # formatting
cargo clippy --workspace --all-targets -- -D warnings

# the napi binding:
cd crates/polyops-napi
npm ci && npm run build && npm test
```

## Parity — the correctness contract

Goldens live in `crates/polyops/tests/goldens/{intersection,union,difference,xor}/`
(one JSON per `(operation, fixture)` with `{subject, clipping, expected}`).
`crates/polyops/tests/parity.rs` runs every Rust op against them within
`1e-10`. To regenerate from upstream:

```bash
cd parity && npm install
MARTINEZ_REPO=../../martinez npm run generate   # rewrites the goldens
```

**Adding a fixture:** drop a GeoJSON file in the upstream
`test/genericTestCases/` (or add to `featureTypes/`), regenerate, and commit
the new goldens. If a parity test fails, that's a real divergence — fix it in
the Rust port, don't loosen the tolerance.

The flat/typed-array path is verified against the same goldens:
`cd parity && npm run verify-flat`.

## Benchmarks

```bash
cargo bench -p polyops              # pure-Rust (criterion)
cd parity && npm run bench          # head-to-head vs martinez@0.8.1 (+ napi)
```

Bench fixtures are vendored under `crates/polyops/benches/fixtures/`
(refresh with `npm run copy-bench-fixtures`). Land perf changes with a
before/after note in `BENCHMARKS.md`.

## Releasing

Versions move in lockstep across crates.io + npm. The pipeline is
**tag-triggered and main-only**:

1. Bump everything together: `[workspace.package] version`, the `polyops`
   path-dep in `crates/polyops-napi/Cargo.toml` (exact-match under `0.0.x`
   caret rules — easy to miss), `package.json`, the 7 `npm/*` platform
   packages (`napi version`), and both lockfiles. Update `CHANGELOG.md`.
2. Merge to `main`, then tag the merged commit: `git tag -a vX.Y.Z -m … && git push origin vX.Y.Z`.
3. `.github/workflows/release.yml` validates (version-match + fmt/clippy/test
   + pack), publishes the crate, builds 7 platform binaries, publishes npm
   (idempotent — safe to re-run), and smoke-installs on Linux/macOS/Windows.

The workflow refuses to publish a tag that isn't on `main`. `index.js` /
`index.d.ts` are committed and CI fails if they drift from `napi build`.

## Pull requests

- Keep `cargo fmt`, `cargo clippy -D warnings`, and `cargo test --workspace`
  green. The parity suite gates merges.
- One logical change per PR; describe what and why.
- No `unsafe` in `crates/polyops`. The public Rust API stays `serde`-free
  (boundary conversions live in `polyops-napi` / behind feature flags).

See [`PLAN.md`](PLAN.md) for architecture and [`ROADMAP.md`](ROADMAP.md) for
where things are headed.
