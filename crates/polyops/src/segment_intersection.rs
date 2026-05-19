//! Segment-segment intersection, port of upstream `src/segment_intersection.ts`.
//!
//! This is one of the hot kernels (151 LOC upstream) and one of the most
//! parity-sensitive — collinear overlaps and degenerate intersections
//! drive most of the failing fixtures during a naive port.

#![allow(dead_code)]

use crate::types::Position;

/// Result of intersecting two segments.
#[derive(Debug, Clone)]
pub(crate) enum SegmentIntersection {
    None,
    Point(Position),
    Overlap(Position, Position),
}

/// Compute the intersection of two line segments.
///
/// TODO: port from upstream `src/segment_intersection.ts`.
pub(crate) fn intersection(
    _a1: Position,
    _a2: Position,
    _b1: Position,
    _b2: Position,
    _no_endpoint_touch: bool,
) -> SegmentIntersection {
    todo!("port src/segment_intersection.ts")
}
