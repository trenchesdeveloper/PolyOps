# PolyOps parity harness

Generates the test goldens that lock our Rust output to
`martinez-polygon-clipping@0.8.1`. Run from this directory:

```bash
npm install
npm run generate
```

By default this expects a sibling clone of upstream at
`../../martinez` (relative to the repo root). Override with:

```bash
MARTINEZ_REPO=/path/to/w8r-martinez-clone npm run generate
```

The script writes one JSON file per case to
`../crates/polyops/tests/goldens/{intersection,union,difference,xor}/`.

Each golden has the shape:

```json
{
  "subject":   [/* GeoJSON Polygon or MultiPolygon coords */],
  "clipping":  [/* GeoJSON Polygon or MultiPolygon coords */],
  "expected":  [/* MultiPolygon coords */] | null
}
```

These are loaded by `cargo test`'s `tests/parity.rs` runner. As the
algorithm comes online, drop the `#[ignore]` attributes in
`tests/parity.rs` to gate CI on parity.
