//! Sweep event nodes used by the Martinez algorithm.
//!
//! Port target: upstream `src/sweep_event.ts`. The JS implementation
//! uses doubly-linked node objects: each `SweepEvent` carries an
//! `otherEvent` reference, and instances are stored in a splay tree
//! and a priority queue.
//!
//! We replace the pointer pattern with an **arena**: events live in a
//! `Vec<SweepEvent>` owned by the sweep driver, and `other_event`
//! holds a `usize` index into that vector. This sidesteps
//! `Rc<RefCell<_>>`, gives us cache-friendly storage, and makes
//! `Clone` / equality semantics trivial.
//!
//! The arena indirection means a few helper methods that upstream
//! defines on `SweepEvent` (specifically `isBelow` / `isAbove`, which
//! need `otherEvent.point`) need either the arena passed in, or the
//! resolved `other_point` passed in as a parameter. We chose the
//! latter — the caller resolves the index once and the predicate stays
//! a pure function of two points + a query point. This keeps the
//! geometric kernels arena-free and trivially testable.

#![allow(dead_code)]

use crate::edge_type::EdgeType;
use crate::types::Position;

/// One endpoint of an input or subdivided segment.
///
/// Fields mirror upstream `SweepEvent` 1:1 with two structural
/// differences:
///
/// 1. `other_event` is `Option<usize>` (arena index) rather than a
///    pointer reference. Sentinels (no peer yet) use `None`.
/// 2. `prev_in_result` is `Option<usize>` for the same reason.
#[derive(Debug, Clone)]
pub(crate) struct SweepEvent {
    /// Endpoint of the segment this event represents.
    pub point: Position,
    /// `true` if this is the left endpoint of its segment.
    pub left: bool,
    /// Index of the peer event (the other endpoint of the same segment).
    pub other_event: Option<usize>,
    /// Which input polygon this event came from.
    pub polygon_type: PolygonType,
    /// Edge classification, populated during `compute_fields`.
    pub edge_type: EdgeType,
    /// Whether this event represents an inside-outside transition for
    /// its own polygon.
    pub in_out: bool,
    /// Same flag for the *other* polygon.
    pub other_in_out: bool,
    /// Previous event on the sweep line that lies in the result.
    pub prev_in_result: Option<usize>,
    /// `connect_edges` transition flag: `+1`, `-1`, or `0`.
    pub result_transition: i32,
    /// Pre-sorted index used by `connect_edges`; `-1` until set.
    pub other_pos: i32,
    /// Output contour index this event was assigned to.
    pub output_contour_id: i32,
    /// Whether the resulting contour is an exterior ring (vs a hole).
    pub is_exterior_ring: bool,
    /// Contour-of-origin identifier used in `compare_segments`
    /// tie-breaking. `None` until `fill_queue` sets it.
    pub contour_id: Option<i32>,
}

/// Whether a sweep event originated from the subject or clipping
/// polygon. Mirrors upstream's `isSubject: boolean` field but as a
/// typed enum so misuse is a compile error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PolygonType {
    Subject,
    Clipping,
}

/**********************************************************************
 * Construction.
 **********************************************************************/
impl SweepEvent {
    /// Build a fresh sweep event. All transient flags default to the
    /// same values upstream's constructor initializes
    /// (`inOut=false`, `otherInOut=false`, `prevInResult=null`,
    /// `resultTransition=0`, `otherPos=-1`, `outputContourId=-1`,
    /// `isExteriorRing=true`).
    pub(crate) fn new(
        point: Position,
        left: bool,
        polygon_type: PolygonType,
        edge_type: EdgeType,
    ) -> Self {
        Self {
            point,
            left,
            other_event: None,
            polygon_type,
            edge_type,
            in_out: false,
            other_in_out: false,
            prev_in_result: None,
            result_transition: 0,
            other_pos: -1,
            output_contour_id: -1,
            is_exterior_ring: true,
            contour_id: None,
        }
    }
}

/**********************************************************************
 * Pure geometric predicates — none of these need the arena. Callers
 * resolve `other_event -> other_point` first and pass it in.
 **********************************************************************/
impl SweepEvent {
    /// Whether the segment `(self.point -> other_point)` lies strictly
    /// below the query point `p`.
    ///
    /// Direct translation of upstream's `isBelow` (which inlines the
    /// cross product). The `self.left` branch flips operand order so
    /// the predicate's geometric meaning is consistent regardless of
    /// which endpoint is "this" event.
    pub(crate) fn is_below(&self, other_point: Position, p: Position) -> bool {
        let p0 = self.point;
        let p1 = other_point;
        if self.left {
            (p0[0] - p[0]) * (p1[1] - p[1]) - (p1[0] - p[0]) * (p0[1] - p[1]) > 0.0
        } else {
            (p1[0] - p[0]) * (p0[1] - p[1]) - (p0[0] - p[0]) * (p1[1] - p[1]) > 0.0
        }
    }

    /// Negation of [`Self::is_below`].
    pub(crate) fn is_above(&self, other_point: Position, p: Position) -> bool {
        !self.is_below(other_point, p)
    }

    /// Whether the segment is vertical (same x for both endpoints).
    pub(crate) fn is_vertical(&self, other_point: Position) -> bool {
        self.point[0] == other_point[0]
    }

    /// Whether this event contributes to the output. Mirrors upstream's
    /// `inResult` getter.
    pub(crate) fn in_result(&self) -> bool {
        self.result_transition != 0
    }

    /// Convenience: whether this event came from the subject polygon.
    pub(crate) fn is_subject(&self) -> bool {
        self.polygon_type == PolygonType::Subject
    }
}

/**********************************************************************
 * Tests.
 **********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_event_initializes_to_upstream_defaults() {
        let e = SweepEvent::new([3.0, 4.0], true, PolygonType::Subject, EdgeType::Normal);
        assert_eq!(e.point, [3.0, 4.0]);
        assert!(e.left);
        assert_eq!(e.other_event, None);
        assert!(matches!(e.polygon_type, PolygonType::Subject));
        assert!(!e.in_out);
        assert!(!e.other_in_out);
        assert_eq!(e.prev_in_result, None);
        assert_eq!(e.result_transition, 0);
        assert_eq!(e.other_pos, -1);
        assert_eq!(e.output_contour_id, -1);
        assert!(e.is_exterior_ring);
        assert_eq!(e.contour_id, None);
    }

    #[test]
    fn is_below_when_query_is_above_segment() {
        /**
         * Segment from (0,0) to (4,4); the point (1,3) lies above the
         * segment. So `is_below(p)` is true — the segment is below the
         * point.
         */
        let e = SweepEvent::new([0.0, 0.0], true, PolygonType::Subject, EdgeType::Normal);
        assert!(e.is_below([4.0, 4.0], [1.0, 3.0]));
        assert!(!e.is_above([4.0, 4.0], [1.0, 3.0]));
    }

    #[test]
    fn is_below_false_when_query_is_below_segment() {
        let e = SweepEvent::new([0.0, 0.0], true, PolygonType::Subject, EdgeType::Normal);
        /** Point (3,1) lies below the (0,0)→(4,4) line. */
        assert!(!e.is_below([4.0, 4.0], [3.0, 1.0]));
        assert!(e.is_above([4.0, 4.0], [3.0, 1.0]));
    }

    #[test]
    fn is_below_flips_when_event_is_right_endpoint() {
        /**
         * The right-endpoint branch uses the opposite cross-product
         * orientation, but the geometric meaning is the same: a query
         * point above the segment line yields `is_below = true`.
         */
        let e = SweepEvent::new([4.0, 4.0], false, PolygonType::Subject, EdgeType::Normal);
        assert!(e.is_below([0.0, 0.0], [1.0, 3.0]));
        assert!(!e.is_below([0.0, 0.0], [3.0, 1.0]));
    }

    #[test]
    fn is_vertical() {
        let e = SweepEvent::new([5.0, 0.0], true, PolygonType::Subject, EdgeType::Normal);
        assert!(e.is_vertical([5.0, 10.0]));
        assert!(!e.is_vertical([5.1, 10.0]));
    }

    #[test]
    fn in_result_tracks_result_transition() {
        let mut e = SweepEvent::new([0.0, 0.0], true, PolygonType::Subject, EdgeType::Normal);
        assert!(!e.in_result());
        e.result_transition = 1;
        assert!(e.in_result());
        e.result_transition = -1;
        assert!(e.in_result());
        e.result_transition = 0;
        assert!(!e.in_result());
    }
}
