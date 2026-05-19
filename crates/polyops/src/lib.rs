//! # PolyOps
//!
//! Boolean operations on polygons via the Martinez-Rueda-Feito sweep-line
//! algorithm. Faithful Rust port of
//! [`martinez-polygon-clipping`](https://github.com/w8r/martinez).
//!
//! ## Operations
//!
//! - [`intersection`] — the area covered by **both** subject and clipping.
//! - [`union`] — the area covered by **either** subject or clipping.
//! - [`difference`] — the area in subject **but not** in clipping.
//! - [`xor`] — the area in subject **xor** clipping (symmetric difference).
//!
//! All four accept GeoJSON-shaped coordinate arrays: either a `Polygon`
//! (`Vec<Ring>`) or a `MultiPolygon` (`Vec<Polygon>`), via the [`Geometry`]
//! enum.
//!
//! ## Status
//!
//! Pre-alpha. The public surface is stable; the algorithm itself is not yet
//! implemented. See the crate's `tests/parity.rs` for the correctness bar.

#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

/**********************************************************************
 * Module layout mirrors the upstream w8r/martinez `src/` directory so
 * cross-referencing during the port is mechanical. Order matches the
 * upstream import graph (leaves first).
 **********************************************************************/
pub mod types;
pub mod operation;

mod signed_area;
mod equals;
mod edge_type;
mod sweep_event;
mod compare_events;
mod compare_segments;
mod segment_intersection;
mod divide_segment;
mod possible_intersection;
mod compute_fields;
mod fill_queue;
mod subdivide_segments;
mod contour;
mod connect_edges;

use crate::operation::Operation;
pub use crate::types::{BBox, Geometry, MultiPolygon, Polygon, Position, Ring};

/**********************************************************************
 * Public API — matches the four entrypoints exposed by upstream
 * `src/index.ts`. Signatures take owned `Geometry` for clarity at the
 * boundary; internally everything flows through `boolean_op`.
 **********************************************************************/

/// Intersection of `subject` and `clipping`.
pub fn intersection(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Intersection)
}

/// Union of `subject` and `clipping`.
pub fn union(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Union)
}

/// `subject` minus `clipping`.
pub fn difference(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Difference)
}

/// Symmetric difference of `subject` and `clipping`.
pub fn xor(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Xor)
}

/**********************************************************************
 * Top-level driver — mirrors upstream `src/index.ts::boolean`.
 * Currently a stub; will be filled in as the port progresses.
 **********************************************************************/
fn boolean_op(
    _subject: Geometry,
    _clipping: Geometry,
    _operation: Operation,
) -> Option<MultiPolygon> {
    /**
     * TODO: trivial-case short-circuits (empty operand, disjoint bboxes),
     * fill_queue, subdivide_segments, connect_edges, contour → polygons.
     * See the upstream `src/index.ts` reference for the structure.
     */
    todo!("polyops::boolean_op not yet implemented — see tests/parity.rs for the parity bar")
}
