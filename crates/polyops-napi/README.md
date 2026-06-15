# polyops

Fast polygon Boolean operations for Node.js — a Rust ([napi-rs](https://napi.rs))
port of [`martinez-polygon-clipping`](https://github.com/w8r/martinez), with
**full behavioral parity** with `martinez-polygon-clipping@0.8.1` and a
typed-array fast path that makes it **faster than martinez at every size**.

Drop-in compatible: the four operations take and return the same
GeoJSON-shaped coordinate arrays as `martinez-polygon-clipping`.

```bash
npm install polyops
```

Prebuilt native binaries ship for macOS (x64/arm64), Linux gnu & musl
(x64/arm64), and Windows (x64) — no build step, no toolchain required.

## Usage

`subject` and `clipping` are GeoJSON-shaped `Polygon` (`number[][][]`) or
`MultiPolygon` (`number[][][][]`) coordinate arrays. Each op returns a
`MultiPolygon` or `null` (empty/disjoint result).

**ESM**

```js
import { union, intersection, diff, xor } from 'polyops';
// or: import polyops from 'polyops';

const subject  = [[[0, 0], [2, 0], [2, 2], [0, 2], [0, 0]]];
const clipping = [[[1, 1], [3, 1], [3, 3], [1, 3], [1, 1]]];

union(subject, clipping);        // => MultiPolygon | null
```

**CommonJS**

```js
const { union, intersection, diff, xor } = require('polyops');
```

### Migrating from `martinez-polygon-clipping`

The exported names match upstream (`intersection`, `union`, `diff`, `xor`),
so it's a one-line swap:

```js
// import martinez from 'martinez-polygon-clipping';
import * as martinez from 'polyops';
```

## API

| Function | Description |
|----------|-------------|
| `intersection(subject, clipping)` | Region in both. |
| `union(subject, clipping)` | Region in either. |
| `diff(subject, clipping)` | `subject` minus `clipping`. |
| `xor(subject, clipping)` | Symmetric difference. |

### Buffer (typed-array) API — for hot loops

The default functions already route through `Float64Array` buffers
internally. If you process many polygons and want to skip re-packing
GeoJSON arrays on every call, work in buffer form directly:

```js
import { pack, unpack, unionFlat } from 'polyops';

const a = pack(subject);          // { coords, ringLengths, polyRingCounts }
const b = pack(clipping);
const result = unpack(unionFlat(a, b));
```

`intersectionFlat`, `unionFlat`, `diffFlat`, `xorFlat` take and return the
`FlatPolys` buffer shape; `pack`/`unpack` convert to/from GeoJSON arrays.

## Performance

On the upstream `union` benchmarks, polyops runs **~1.8–2.9× faster** than
`martinez-polygon-clipping@0.8.1` in pure Rust, and **~1.9× (small) to
~2–2.6× (large)** through the Node binding. Full methodology and numbers:
[BENCHMARKS.md](https://github.com/trenchesdeveloper/PolyOps/blob/main/BENCHMARKS.md).

> Note: for *very small, single* polygon ops, in-process JS can edge out any
> native binding (the N-API call cost dominates trivial work). polyops wins
> on substantive polygon workloads.

## polyops vs `martinez-polygon-clipping`

A drop-in, faster replacement — same GeoJSON-shaped API, same results
(verified to parity), Rust under the hood:

| | **polyops** | martinez-polygon-clipping |
|---|---|---|
| Engine | Rust (native, prebuilt) | pure JavaScript |
| Speed (union benchmarks) | **1.8–2.9× (Rust), 1.9–2.6× (Node)** | 1× (baseline) |
| API | drop-in (identical shape) | — |
| Output parity | matches `0.8.1` exactly | — |
| Modules | ESM + CommonJS | CommonJS |
| Prebuilt platforms | macOS, Linux gnu+musl, Windows | n/a |

### When to use polyops

- You already use `martinez-polygon-clipping` and want it **faster, same API**.
- You want maintained polygon Boolean ops with **prebuilt binaries** (no build toolchain).
- You process **substantive** polygons (clip paths, GIS, vector geometry).

### Other polygon-Boolean libraries

Different algorithms / trade-offs — pick by need:

- **`martinez-polygon-clipping`** — the pure-JS reference polyops ports; use it if you can't ship a native addon.
- **`polygon-clipping`**, **`polybooljs`** — other pure-JS Boolean-operation libraries.
- **`clipper2`**, **`i_overlay`** — fast native/Rust libraries using *different* algorithms (not Martinez-compatible).

## Correctness

Verified against the full `martinez-polygon-clipping@0.8.1` fixture corpus
(coordinate-wise equality within `1e-10`). polyops aims to match upstream
exactly; a divergence is a bug — please
[report it](https://github.com/trenchesdeveloper/PolyOps/issues).

## License

MIT. Credit to Alexander Milevski for the JS reference implementation and to
Martinez/Rueda/Feito for the algorithm. Source & docs:
[github.com/trenchesdeveloper/PolyOps](https://github.com/trenchesdeveloper/PolyOps).
