//! Performance baseline (PLAN.md §11, Milestone 7).
//!
//! Mirrors upstream `bench/martinez.bench.ts`: three `union` workloads
//! over the same fixtures the upstream benchmarks use —
//!
//!   - `hole_hole`   small, many degeneracies (the hot kernel)
//!   - `asia_union`  large subject (~tens of thousands of vertices)
//!   - `states_clip` many polygons (the multi-polygon path)
//!
//! Fixtures are vendored under `benches/fixtures/` (origin + refresh
//! noted in `benches/fixtures/README.md`) so `cargo bench` is zero-setup.
//!
//! Inputs are parsed once, up front. The measured closure uses
//! `iter_batched` with the clone in *setup* (untimed), so we measure the
//! algorithm, not the `Geometry` clone that `union`'s by-value signature
//! forces. The companion Node harness (`parity/bench.ts`) compares these
//! same inputs against `martinez-polygon-clipping@0.8.1` and the napi
//! binding; see PERFORMANCE_PLAN.md §5.2.

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion, SamplingMode};
use polyops::{union, Geometry, MultiPolygon};
use serde_json::Value;

/*
 * Fixture loading. `coords_to_geometry` / `depth` mirror the helpers in
 * `tests/parity.rs` — GeoJSON discriminates Polygon vs MultiPolygon by
 * coordinate nesting depth (Polygon is [[[x,y],...]]; MultiPolygon adds a
 * level).
 */

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("benches")
        .join("fixtures")
}

fn read_fixture(name: &str) -> Value {
    let path = fixtures_dir().join(name);
    let text =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

fn depth(value: &Value) -> usize {
    let mut d = 0;
    let mut cur = value;
    while let Value::Array(arr) = cur {
        d += 1;
        match arr.first() {
            Some(next) => cur = next,
            None => break,
        }
    }
    d
}

fn coords_to_geometry(value: Value) -> Geometry {
    if depth(&value) >= 4 {
        let mp: MultiPolygon = serde_json::from_value(value).expect("multipolygon coords");
        Geometry::MultiPolygon(mp)
    } else {
        let p: Vec<Vec<[f64; 2]>> = serde_json::from_value(value).expect("polygon coords");
        Geometry::Polygon(p)
    }
}

/// `features[i].geometry.coordinates` from a FeatureCollection.
fn feature_coords(v: &Value, i: usize) -> Value {
    v["features"][i]["geometry"]["coordinates"].clone()
}

/// `geometry.coordinates` from a single Feature.
fn geometry_coords(v: &Value) -> Value {
    v["geometry"]["coordinates"].clone()
}

struct Scenario {
    name: &'static str,
    subject: Geometry,
    clipping: Geometry,
    /// criterion sample size — small for the slow, large workloads.
    sample_size: usize,
}

fn load_scenarios() -> Vec<Scenario> {
    let hole_hole = read_fixture("hole_hole.geojson");
    let asia = read_fixture("asia.geojson");
    let asia_clip = read_fixture("asia_unionPoly.geojson");
    let states = read_fixture("states_source.geojson");

    vec![
        Scenario {
            name: "hole_hole",
            subject: coords_to_geometry(feature_coords(&hole_hole, 0)),
            clipping: coords_to_geometry(feature_coords(&hole_hole, 1)),
            sample_size: 100,
        },
        Scenario {
            // subject is a FeatureCollection feature; clip is a bare Feature.
            name: "asia_union",
            subject: coords_to_geometry(feature_coords(&asia, 0)),
            clipping: coords_to_geometry(geometry_coords(&asia_clip)),
            sample_size: 10,
        },
        Scenario {
            name: "states_clip",
            subject: coords_to_geometry(feature_coords(&states, 0)),
            clipping: coords_to_geometry(feature_coords(&states, 1)),
            sample_size: 20,
        },
    ]
}

fn bench_union(c: &mut Criterion) {
    let scenarios = load_scenarios();
    let mut group = c.benchmark_group("union");
    // Long-running workloads (asia) want flat sampling, not the default
    // linear ramp.
    group.sampling_mode(SamplingMode::Flat);

    for scenario in &scenarios {
        group.sample_size(scenario.sample_size);
        group.bench_function(scenario.name, |b| {
            b.iter_batched(
                || (scenario.subject.clone(), scenario.clipping.clone()),
                |(subject, clipping)| union(black_box(subject), black_box(clipping)),
                BatchSize::SmallInput,
            )
        });
    }
    group.finish();
}

criterion_group!(benches, bench_union);
criterion_main!(benches);
