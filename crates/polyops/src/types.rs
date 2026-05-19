//! Coordinate types matching GeoJSON shape, mirroring upstream `src/types.ts`.

/// `[x, y]` coordinate.
pub type Position = [f64; 2];

/// Closed ring of positions. By GeoJSON convention the first and last
/// positions are equal.
pub type Ring = Vec<Position>;

/// Polygon: exterior ring followed by zero or more hole rings.
pub type Polygon = Vec<Ring>;

/// Collection of polygons.
pub type MultiPolygon = Vec<Polygon>;

/// Axis-aligned bounding box: `[min_x, min_y, max_x, max_y]`.
pub type BBox = [f64; 4];

/// GeoJSON-shaped input to the four Boolean operations. Mirrors the
/// upstream `Geometry = Polygon | MultiPolygon` union by discriminating
/// explicitly at the Rust API boundary.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    /// A single polygon (exterior + optional holes).
    Polygon(Polygon),
    /// A collection of polygons.
    MultiPolygon(MultiPolygon),
}

impl Geometry {
    /// Normalize either variant to a `MultiPolygon` for internal processing.
    ///
    /// Will be wired up by `boolean_op` once the algorithm is in place;
    /// allow(dead_code) until then.
    #[allow(dead_code)]
    pub(crate) fn into_multi(self) -> MultiPolygon {
        match self {
            Geometry::Polygon(p) => vec![p],
            Geometry::MultiPolygon(mp) => mp,
        }
    }
}

impl From<Polygon> for Geometry {
    fn from(p: Polygon) -> Self {
        Geometry::Polygon(p)
    }
}

impl From<MultiPolygon> for Geometry {
    fn from(mp: MultiPolygon) -> Self {
        Geometry::MultiPolygon(mp)
    }
}
