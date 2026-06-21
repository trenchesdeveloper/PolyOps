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
//! ## Example
//!
//! ```
//! use polyops::{union, Geometry};
//!
//! let subject = Geometry::Polygon(vec![vec![
//!     [0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0],
//! ]]);
//! let clipping = Geometry::Polygon(vec![vec![
//!     [1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0], [1.0, 1.0],
//! ]]);
//!
//! let merged = union(subject, clipping).expect("overlapping squares");
//! assert_eq!(merged.len(), 1); // one connected polygon
//! ```
//!
//! ## Optional features
//!
//! - `serde` — derive `Serialize`/`Deserialize` on [`Geometry`] and [`Operation`].
//! - `geo-types` — `From`/`ToGeo` conversions to and from the `geo-types` crate.
//!
//! Both are off by default; the core stays dependency-light and serde-free.
//!
//! ## Parity
//!
//! Behavioral parity with `martinez-polygon-clipping@0.8.1` is verified by
//! `tests/parity.rs` against committed goldens. Published on
//! [crates.io](https://crates.io/crates/polyops) and
//! [npm](https://www.npmjs.com/package/polyops).

#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![deny(rust_2018_idioms)]
#![warn(missing_docs)]

/*
 * Module layout mirrors the upstream w8r/martinez `src/` directory so
 * cross-referencing during the port is mechanical. Order matches the
 * upstream import graph (leaves first).
 */
pub mod operation;
pub mod types;

mod compare_events;
mod compare_segments;
mod compute_fields;
mod connect_edges;
mod contour;
mod divide_segment;
mod edge_type;
mod equals;
mod event_queue;
mod fill_queue;
mod possible_intersection;
mod segment_intersection;
mod signed_area;
mod subdivide_segments;
mod sweep_event;
mod sweep_line;

#[cfg(feature = "geo-types")]
mod geo_compat;

use crate::connect_edges::connect_edges;
use crate::event_queue::EventQueue;
use crate::fill_queue::fill_queue;
pub use crate::operation::Operation;
use crate::subdivide_segments::subdivide_segments;
use crate::sweep_event::SweepEvent;
pub use crate::types::{BBox, Geometry, MultiPolygon, Polygon, Position, Ring};

/// `geo-types` interop (feature `geo-types`): the [`ToGeo`] output trait.
/// The `From<geo_types::…>` input impls live in `geo_compat`.
#[cfg(feature = "geo-types")]
pub use crate::geo_compat::ToGeo;

/*
 * Public API — matches the four entrypoints exposed by upstream
 * `src/index.ts`. Signatures take owned `Geometry` for clarity at the
 * boundary; internally everything flows through `boolean_op`.
 */

/// Intersection of `subject` and `clipping` — the area covered by both.
///
/// Returns `None` when the result is empty (e.g. disjoint inputs).
///
/// # Examples
/// ```
/// use polyops::{intersection, Geometry};
/// let a = Geometry::Polygon(vec![vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]);
/// let b = Geometry::Polygon(vec![vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0], [1.0, 1.0]]]);
/// assert!(!intersection(a, b).unwrap().is_empty());
/// ```
pub fn intersection(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Intersection)
}

/// Union of `subject` and `clipping` — the area covered by either.
///
/// # Examples
/// ```
/// use polyops::{union, Geometry};
/// let a = Geometry::Polygon(vec![vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]);
/// let b = Geometry::Polygon(vec![vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0], [1.0, 1.0]]]);
/// assert_eq!(union(a, b).unwrap().len(), 1); // one connected polygon
/// ```
pub fn union(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Union)
}

/// `subject` minus `clipping` — the area in `subject` but not `clipping`.
///
/// # Examples
/// ```
/// use polyops::{difference, Geometry};
/// let a = Geometry::Polygon(vec![vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]);
/// let b = Geometry::Polygon(vec![vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0], [1.0, 1.0]]]);
/// assert!(!difference(a, b).unwrap().is_empty());
/// ```
pub fn difference(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Difference)
}

/// Symmetric difference of `subject` and `clipping` — area in exactly one.
///
/// # Examples
/// ```
/// use polyops::{xor, Geometry};
/// let a = Geometry::Polygon(vec![vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0], [0.0, 2.0], [0.0, 0.0]]]);
/// let b = Geometry::Polygon(vec![vec![[1.0, 1.0], [3.0, 1.0], [3.0, 3.0], [1.0, 3.0], [1.0, 1.0]]]);
/// assert!(!xor(a, b).unwrap().is_empty());
/// ```
pub fn xor(subject: Geometry, clipping: Geometry) -> Option<MultiPolygon> {
    boolean_op(subject, clipping, Operation::Xor)
}

/*
 * Top-level driver — mirrors upstream `src/index.ts::boolean`.
 *
 * Pipeline:
 *   1. Normalize inputs to MultiPolygon.
 *   2. Trivial-case shortcut for empty operands.
 *   3. Build the event queue and per-input bounding boxes via fill_queue.
 *   4. Bbox-disjoint shortcut.
 *   5. Run the sweep via subdivide_segments.
 *   6. Stitch result events into contours via connect_edges.
 *   7. Assemble exterior rings + their holes into output polygons.
 */
fn boolean_op(subject: Geometry, clipping: Geometry, operation: Operation) -> Option<MultiPolygon> {
    let subject_mp = subject.into_multi();
    let clipping_mp = clipping.into_multi();

    /*
     * Trivial-operation shortcut. If either operand is empty, the
     * result is known without running the sweep. Upstream distinguishes
     * two flavors of "empty": the EMPTY sentinel for intersection
     * (returns null) and an actual empty MultiPolygon for difference
     * (returns []) — both encoded here as None vs Some([]) respectively.
     */
    if subject_mp.is_empty() || clipping_mp.is_empty() {
        return match operation {
            Operation::Intersection => None,
            Operation::Difference => Some(subject_mp),
            Operation::Union | Operation::Xor => Some(if subject_mp.is_empty() {
                clipping_mp
            } else {
                subject_mp
            }),
        };
    }

    /*
     * Initialize bboxes to "empty" (mins at +inf, maxes at -inf) so
     * the per-vertex min/max accumulation in fill_queue starts from
     * a neutral state.
     */
    let mut sbbox: BBox = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut cbbox: BBox = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];

    let mut arena: Vec<SweepEvent> = Vec::new();
    let mut queue = EventQueue::new();
    fill_queue(
        &mut arena,
        &mut queue,
        &subject_mp,
        &clipping_mp,
        &mut sbbox,
        &mut cbbox,
        operation,
    );

    /*
     * Bbox-disjoint shortcut. Same null-vs-[] distinction as above:
     * disjoint intersection returns null, disjoint difference returns
     * the subject, disjoint union/xor returns the concatenation.
     */
    let disjoint =
        sbbox[0] > cbbox[2] || cbbox[0] > sbbox[2] || sbbox[1] > cbbox[3] || cbbox[1] > sbbox[3];
    if disjoint {
        return match operation {
            Operation::Intersection => None,
            Operation::Difference => Some(subject_mp),
            Operation::Union | Operation::Xor => {
                let mut combined = subject_mp;
                combined.extend(clipping_mp);
                Some(combined)
            }
        };
    }

    let sorted_events = subdivide_segments(&mut arena, &mut queue, sbbox, cbbox, operation);
    let contours = connect_edges(&mut arena, &sorted_events);

    /*
     * Assemble: for each exterior contour, emit a polygon with the
     * exterior as its first ring followed by its holes. Sweep-produced
     * empty result returns Some([]) — only the trivial/disjoint
     * intersection shortcuts return None.
     */
    let mut polygons: MultiPolygon = Vec::new();
    for contour in &contours {
        if !contour.is_exterior() {
            continue;
        }
        let mut rings: Polygon = Vec::with_capacity(1 + contour.hole_ids.len());
        rings.push(contour.points.clone());
        for &hole_id in &contour.hole_ids {
            rings.push(contours[hole_id].points.clone());
        }
        polygons.push(rings);
    }

    Some(polygons)
}
