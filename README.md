# PolyOps

Fast polygon Boolean operations in Rust, with first-class Node.js bindings.

PolyOps is a faithful port of [`martinez-polygon-clipping`](https://github.com/w8r/martinez)
(Alex Milevski's JavaScript implementation of the Martinez-Rueda-Feito algorithm)
into idiomatic Rust, with a [`napi-rs`](https://napi.rs) wrapper so the same
engine can be consumed from Node.js without giving up native performance.

> **Status:** pre-alpha. The scaffold is in place; the algorithm is not yet
> implemented. See [`PLAN.md`](PLAN.md) for the full implementation plan
> (milestones, file-by-file porting order, decision log, open questions)
> and [`crates/polyops/src/lib.rs`](crates/polyops/src/lib.rs) for the
> public API surface.

## Operations

```
intersection(subject, clipping)
union       (subject, clipping)
difference  (subject, clipping)
xor         (subject, clipping)
```

All four operate on GeoJSON-shaped `Polygon` or `MultiPolygon` coordinate
arrays, matching the upstream JS API.

## Packages

| Registry   | Name              | Source                  |
|------------|-------------------|-------------------------|
| crates.io  | `polyops`         | `crates/polyops`        |
| npm        | `polyops`         | `crates/polyops-napi`   |

## Layout

```
PolyOps/
├── crates/
│   ├── polyops/          pure-Rust crate (algorithm, types, tests)
│   └── polyops-napi/     napi-rs binding, published to npm
├── parity/               Node harness: runs martinez-polygon-clipping@0.8.1
│                         against all upstream fixtures, emits goldens
└── .github/workflows/    CI: cargo test + parity tests
```

## Development

```bash
cargo test          # runs unit + parity tests
cargo bench         # runs benchmarks
cargo clippy        # lints
cargo fmt --check
```

To regenerate the parity goldens against the latest upstream:

```bash
cd parity
npm install
npm run generate
```

## License

MIT. See [`LICENSE`](LICENSE). Credit to Alexander Milevski for the JS
reference implementation and to Martinez/Rueda/Feito for the algorithm.
