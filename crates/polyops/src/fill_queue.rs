//! Initial event-queue population from the input polygons,
//! port of upstream `src/fill_queue.ts`.

#![allow(dead_code)]

use crate::operation::Operation;
use crate::types::{BBox, MultiPolygon};

/// Build the initial sweep-event priority queue from the input
/// polygons, accumulating bounding boxes along the way.
///
/// TODO: port from upstream `src/fill_queue.ts`.
pub(crate) fn fill_queue(
    _subject: &MultiPolygon,
    _clipping: &MultiPolygon,
    _sbbox: &mut BBox,
    _cbbox: &mut BBox,
    _operation: Operation,
) {
    todo!("port src/fill_queue.ts")
}
