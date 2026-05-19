//! Node.js bindings for `polyops`.
//!
//! Exposes the four Boolean operations over GeoJSON-shaped coordinate
//! arrays — drop-in compatible with the public API of
//! [`martinez-polygon-clipping`](https://www.npmjs.com/package/martinez-polygon-clipping).

#![deny(clippy::all)]

use napi::bindgen_prelude::*;
use napi_derive::napi;
use polyops::{Geometry, MultiPolygon, Polygon, Position};

/**********************************************************************
 * GeoJSON-shaped boundary types — these are what TypeScript will see.
 * Positions are `Vec<f64>` (rather than `[f64; 2]`) to absorb 3D inputs
 * gracefully; we ignore anything beyond x/y at the boundary.
 **********************************************************************/
type JsPosition = Vec<f64>;
type JsRing = Vec<JsPosition>;
type JsPolygon = Vec<JsRing>;
type JsMultiPolygon = Vec<JsPolygon>;

/**********************************************************************
 * Internal helpers — alphabetical.
 **********************************************************************/

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

/**********************************************************************
 * Public napi entry points — match the upstream JS API names.
 **********************************************************************/

/// Intersection of `subject` and `clipping`.
#[napi]
pub fn intersection(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::intersection)
}

/// Union of `subject` and `clipping`.
#[napi]
pub fn union(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::union)
}

/// `subject` minus `clipping`.
#[napi]
pub fn diff(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::difference)
}

/// Symmetric difference of `subject` and `clipping`.
#[napi]
pub fn xor(
    subject: Either<JsPolygon, JsMultiPolygon>,
    clipping: Either<JsPolygon, JsMultiPolygon>,
) -> Result<Option<JsMultiPolygon>> {
    run(subject, clipping, polyops::xor)
}
