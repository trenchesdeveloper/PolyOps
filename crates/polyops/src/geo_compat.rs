//! `geo-types` interop (feature `geo-types`).
//!
//! polyops's `Polygon`/`MultiPolygon` are bare `Vec` aliases, so the orphan
//! rule forbids `From` impls *to* foreign `geo-types` on them. So the
//! conversions are split:
//!
//! - **into polyops** — ergonomic `From`: `geo_types::Polygon` /
//!   `geo_types::MultiPolygon` → [`Geometry`].
//! - **out of polyops** — the [`ToGeo`] extension trait: call `.to_geo()`
//!   on a [`Geometry`] or directly on an op result ([`MultiPolygon`]).
//!
//! ```ignore
//! use polyops::{union, Geometry, ToGeo};
//! let subject: Geometry = some_geo_polygon.into();
//! let clipping: Geometry = some_geo_multipolygon.into();
//! let out = union(subject, clipping).unwrap_or_default().to_geo();
//! ```

use crate::{Geometry, MultiPolygon, Polygon, Ring};
use geo_types as gt;

fn ring_to_ls(ring: &Ring) -> gt::LineString<f64> {
    gt::LineString(ring.iter().map(|&[x, y]| gt::Coord { x, y }).collect())
}

fn ls_to_ring(ls: &gt::LineString<f64>) -> Ring {
    ls.0.iter().map(|c| [c.x, c.y]).collect()
}

fn poly_to_gt(p: &Polygon) -> gt::Polygon<f64> {
    let mut rings = p.iter();
    let exterior = rings
        .next()
        .map(ring_to_ls)
        .unwrap_or_else(|| gt::LineString(Vec::new()));
    gt::Polygon::new(exterior, rings.map(ring_to_ls).collect())
}

fn gt_to_poly(p: &gt::Polygon<f64>) -> Polygon {
    let mut out = Vec::with_capacity(1 + p.interiors().len());
    out.push(ls_to_ring(p.exterior()));
    out.extend(p.interiors().iter().map(ls_to_ring));
    out
}

/// `geo_types::Polygon` → [`Geometry::Polygon`].
impl From<gt::Polygon<f64>> for Geometry {
    fn from(p: gt::Polygon<f64>) -> Self {
        Geometry::Polygon(gt_to_poly(&p))
    }
}

/// `geo_types::MultiPolygon` → [`Geometry::MultiPolygon`].
impl From<gt::MultiPolygon<f64>> for Geometry {
    fn from(mp: gt::MultiPolygon<f64>) -> Self {
        Geometry::MultiPolygon(mp.0.iter().map(gt_to_poly).collect())
    }
}

/// Convert polyops geometry into a [`geo_types::MultiPolygon`].
///
/// Implemented for [`Geometry`] and for op results ([`MultiPolygon`]). A
/// single [`Geometry::Polygon`] becomes a one-polygon multipolygon. (Output
/// can't be a `From` impl — the orphan rule forbids `impl From<Geometry> for
/// geo_types::MultiPolygon`, since the target is foreign.)
pub trait ToGeo {
    /// Convert into a `geo_types::MultiPolygon<f64>`.
    fn to_geo(&self) -> gt::MultiPolygon<f64>;
}

impl ToGeo for MultiPolygon {
    fn to_geo(&self) -> gt::MultiPolygon<f64> {
        gt::MultiPolygon(self.iter().map(poly_to_gt).collect())
    }
}

impl ToGeo for Geometry {
    fn to_geo(&self) -> gt::MultiPolygon<f64> {
        match self {
            Geometry::Polygon(p) => gt::MultiPolygon(vec![poly_to_gt(p)]),
            Geometry::MultiPolygon(mp) => mp.to_geo(),
        }
    }
}
