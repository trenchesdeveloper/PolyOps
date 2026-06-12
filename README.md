# PolyOps

Fast polygon Boolean operations in Rust, with first-class Node.js bindings.

PolyOps is a faithful port of [`martinez-polygon-clipping`](https://github.com/w8r/martinez)
(Alex Milevski's JavaScript implementation of the Martinez-Rueda-Feito algorithm)
into idiomatic Rust, with a [`napi-rs`](https://napi.rs) wrapper so the same
engine can be consumed from Node.js without giving up native performance.

> **Status:** published. `polyops` 0.0.4 is on
> [crates.io](https://crates.io/crates/polyops) and
> [npm](https://www.npmjs.com/package/polyops) (prebuilt binaries for macOS,
> Linux gnu/musl, and Windows), with full behavioral parity against
> `martinez-polygon-clipping@0.8.1` over the upstream fixture corpus. The API
> is still `0.0.x` and may change before `0.1`.

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

## Performance

On the upstream `union` benchmarks, `polyops` (Rust, single-thread) runs
**~1.8×–2.9× faster** than `martinez-polygon-clipping@0.8.1`. Through the
Node binding the win is **~1.35×–1.6×** on medium/large inputs; the N-API
marshalling adds overhead that can erase it for very small ops. Full numbers,
methodology, and the optimization backlog are in [`BENCHMARKS.md`](BENCHMARKS.md).

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
