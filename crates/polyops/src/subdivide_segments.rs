//! Main sweep loop that subdivides segments at every intersection,
//! port of upstream `src/subdivide_segments.ts`.

#![allow(dead_code)]

use crate::operation::Operation;
use crate::types::{BBox, MultiPolygon};

/// Sweep over the event queue, producing a sorted list of subdivided
/// sweep events ready for [`crate::connect_edges`].
///
/// TODO: port from upstream `src/subdivide_segments.ts`.
pub(crate) fn subdivide_segments(
    _subject: &MultiPolygon,
    _clipping: &MultiPolygon,
    _sbbox: &BBox,
    _cbbox: &BBox,
    _operation: Operation,
) {
    todo!("port src/subdivide_segments.ts")
}
