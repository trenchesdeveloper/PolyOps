//! Node.js bindings for `polyops`.
//!
//! Exposes the four Boolean operations over GeoJSON-shaped coordinate
//! arrays — drop-in compatible with the public API of
//! [`martinez-polygon-clipping`](https://www.npmjs.com/package/martinez-polygon-clipping).

#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use polyops::{Geometry, MultiPolygon, Polygon, Position};

/*
 * GeoJSON-shaped boundary types — these are what TypeScript will see.
 * Positions are `Vec<f64>` (rather than `[f64; 2]`) to absorb 3D inputs
 * gracefully; we ignore anything beyond x/y at the boundary.
 */
type JsPosition = Vec<f64>;
type JsRing = Vec<JsPosition>;
type JsPolygon = Vec<JsRing>;
type JsMultiPolygon = Vec<JsPolygon>;

/*
 * Internal helpers — alphabetical.
 */

fn into_geometry(value: Either<JsPolygon, JsMultiPolygon>) -> Result<Geometry> {
    match value {
        Either::A(p) => Ok(Geometry::Polygon(to_polygon(p)?)),
        Either::B(mp) => {
            let polygons = mp
                .into_iter()
                .map(to_polygon)
                .collect::<Result<Vec<Polygon>>>()?;
            Ok(Geometry::MultiPolygon(polygons))
        }
    }
}

fn run(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
    op: fn(Geometry, Geometry) -> Option<MultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    let s = into_geometry(subject)?;
    let c = into_geometry(clipping)?;
    Ok(op(s, c).map(to_js_multipolygon))
}

fn to_js_multipolygon(mp: MultiPolygon) -> JsMultiPolygon {
    mp.into_iter()
        .map(|polygon| {
            polygon
                .into_iter()
                .map(|ring| ring.into_iter().map(|pos| pos.to_vec()).collect())
                .collect()
        })
        .collect()
}

fn to_polygon(p: JsPolygon) -> Result<Polygon> {
    p.into_iter()
        .map(|ring| {
            ring.into_iter()
                .map(to_position)
                .collect::<Result<Vec<Position>>>()
        })
        .collect()
}

fn to_position(p: JsPosition) -> Result<Position> {
    if p.len() < 2 {
        return Err(Error::from_reason(format!(
            "position must have at least 2 components, got {}",
            p.len()
        )));
    }
    Ok([p[0], p[1]])
}

/*
 * Public napi entry points — match the upstream JS API names.
 */

/// Intersection of `subject` and `clipping`.
#[napi(
    ts_args_type = "subject: number[][][] | number[][][][], clipping: number[][][] | number[][][][]",
    ts_return_type = "number[][][][] | null"
)]
pub fn intersection(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::intersection)
}

/// Union of `subject` and `clipping`.
#[napi(
    ts_args_type = "subject: number[][][] | number[][][][], clipping: number[][][] | number[][][][]",
    ts_return_type = "number[][][][] | null"
)]
pub fn union(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::union)
}

/// `subject` minus `clipping`.
#[napi(
    ts_args_type = "subject: number[][][] | number[][][][], clipping: number[][][] | number[][][][]",
    ts_return_type = "number[][][][] | null"
)]
pub fn diff(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::difference)
}

/// Symmetric difference of `subject` and `clipping`.
#[napi(
    ts_args_type = "subject: number[][][] | number[][][][], clipping: number[][][] | number[][][][]",
    ts_return_type = "number[][][][] | null"
)]
pub fn xor(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::xor)
}

/*
 * Flat / typed-array API.
 *
 * Polygon Boolean ops over a buffer-based MultiPolygon encoding that skips
 * the per-coordinate JS<->Rust conversion the nested-array API pays at the
 * N-API boundary (which the benchmarks show is >half the cost on large
 * inputs and dominates on small ones). A MultiPolygon is three parallel
 * buffers:
 *   - `coords`         Float64Array of [x,y,x,y,...] across every ring
 *   - `ringLengths`    Uint32Array, #positions per ring (ring order)
 *   - `polyRingCounts` Uint32Array, #rings per polygon (polygon order)
 * A Polygon is encoded as a 1-polygon multipolygon. Pack/unpack helpers
 * for GeoJSON-shaped arrays ship alongside as `flat.js` / `flat.d.ts`.
 */

#[napi(object)]
pub struct FlatPolys {
    pub coords: Float64Array,
    pub ring_lengths: Uint32Array,
    pub poly_ring_counts: Uint32Array,
}

fn flat_to_geometry(p: &FlatPolys) -> Geometry {
    let coords: &[f64] = &p.coords;
    let ring_lengths: &[u32] = &p.ring_lengths;
    let poly_ring_counts: &[u32] = &p.poly_ring_counts;

    let mut polygons: Vec<Polygon> = Vec::with_capacity(poly_ring_counts.len());
    let mut ring_idx = 0usize;
    let mut coord_idx = 0usize;
    for &nrings in poly_ring_counts {
        let mut rings: Polygon = Vec::with_capacity(nrings as usize);
        for _ in 0..nrings {
            let n = ring_lengths[ring_idx] as usize;
            ring_idx += 1;
            let mut ring = Vec::with_capacity(n);
            for _ in 0..n {
                ring.push([coords[coord_idx], coords[coord_idx + 1]]);
                coord_idx += 2;
            }
            rings.push(ring);
        }
        polygons.push(rings);
    }
    Geometry::MultiPolygon(polygons)
}

fn geometry_to_flat(mp: MultiPolygon) -> FlatPolys {
    let total_coords: usize = mp.iter().flatten().map(|r| r.len() * 2).sum();
    let total_rings: usize = mp.iter().map(|p| p.len()).sum();
    let mut coords: Vec<f64> = Vec::with_capacity(total_coords);
    let mut ring_lengths: Vec<u32> = Vec::with_capacity(total_rings);
    let mut poly_ring_counts: Vec<u32> = Vec::with_capacity(mp.len());
    for polygon in &mp {
        poly_ring_counts.push(polygon.len() as u32);
        for ring in polygon {
            ring_lengths.push(ring.len() as u32);
            for pos in ring {
                coords.push(pos[0]);
                coords.push(pos[1]);
            }
        }
    }
    FlatPolys {
        coords: Float64Array::new(coords),
        ring_lengths: Uint32Array::new(ring_lengths),
        poly_ring_counts: Uint32Array::new(poly_ring_counts),
    }
}

fn run_flat(
    subject: FlatPolys,
    clipping: FlatPolys,
    op: fn(Geometry, Geometry) -> Option<MultiPolygon>,
) -> Option<FlatPolys> {
    let s = flat_to_geometry(&subject);
    let c = flat_to_geometry(&clipping);
    op(s, c).map(geometry_to_flat)
}

/// Intersection over the flat/typed-array representation.
#[napi]
pub fn intersection_flat(subject: FlatPolys, clipping: FlatPolys) -> Option<FlatPolys> {
    run_flat(subject, clipping, polyops::intersection)
}

/// Union over the flat/typed-array representation.
#[napi]
pub fn union_flat(subject: FlatPolys, clipping: FlatPolys) -> Option<FlatPolys> {
    run_flat(subject, clipping, polyops::union)
}

/// `subject` minus `clipping` over the flat/typed-array representation.
#[napi]
pub fn diff_flat(subject: FlatPolys, clipping: FlatPolys) -> Option<FlatPolys> {
    run_flat(subject, clipping, polyops::difference)
}

/// Symmetric difference over the flat/typed-array representation.
#[napi]
pub fn xor_flat(subject: FlatPolys, clipping: FlatPolys) -> Option<FlatPolys> {
    run_flat(subject, clipping, polyops::xor)
}
