# Benchmark fixtures

Vendored from upstream [`w8r/martinez`](https://github.com/w8r/martinez)
(`test/fixtures/`, MIT-licensed) so `cargo bench` is reproducible without
a local clone. They are the exact inputs upstream's
`bench/martinez.bench.ts` uses, so PolyOps numbers are comparable to
upstream's published ones.

| File | Shape | Used as |
|------|-------|---------|
| `hole_hole.geojson` | FeatureCollection (2 × MultiPolygon) | `hole_hole` — `features[0]` ∪ `features[1]` |
| `asia.geojson` | FeatureCollection (1 × MultiPolygon, ~1.2 MB) | `asia_union` subject — `features[0]` |
| `asia_unionPoly.geojson` | Feature (Polygon) | `asia_union` clip — `geometry.coordinates` |
| `states_source.geojson` | FeatureCollection (MultiPolygon + Polygon) | `states_clip` — `features[0]` ∪ `features[1]` |

## Refreshing

When upstream bumps, re-copy from a `w8r/martinez` clone:

```bash
cd parity && MARTINEZ_REPO=../../martinez npm run copy-bench-fixtures
```

(or copy `$MARTINEZ_REPO/test/fixtures/{hole_hole,asia,asia_unionPoly,states_source}.geojson`
here by hand). See [`../../../../PERFORMANCE_PLAN.md`](../../../../PERFORMANCE_PLAN.md) §3.
