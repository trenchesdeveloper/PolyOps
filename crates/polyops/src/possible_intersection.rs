//! Sweep-time intersection dispatcher, port of upstream
//! `src/possible_intersection.ts`.
//!
//! Invoked during the sweep whenever two segments become neighbors on
//! the sweep-line status tree. Asks [`crate::segment_intersection`]
//! for the geometric relationship, then dispatches to one of four
//! branches:
//!
//! - **No intersection:** nothing to do.
//! - **Single-point interior intersection:** split each segment at
//!   the intersection point via [`crate::divide_segment`], unless the
//!   intersection coincides with an endpoint of that segment.
//! - **Single-point endpoint-only touch:** ignore (segments are
//!   already correctly connected at that point).
//! - **Collinear overlap (two intersection points):** mark events
//!   with the appropriate [`crate::edge_type::EdgeType`] and split
//!   the overlapping portion off.
//!
//! Return value is an integer code (0, 1, 2, 3) matching upstream's
//! return for tests; downstream callers ignore it.

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::compare_events::compare_events;
use crate::divide_segment::divide_segment;
use crate::edge_type::EdgeType;
use crate::equals::equals;
use crate::event_queue::EventQueue;
use crate::segment_intersection::{intersection, SegmentIntersection};
use crate::sweep_event::SweepEvent;
//pub mod sweep_event;


/// Test the geometric relationship between two sweep-line neighbors
/// `se1_idx` and `se2_idx` and dispatch any subdivisions or
/// edge-type tagging that the algorithm requires.
///
/// Returns an integer code matching upstream:
/// - `0` — no contributing intersection (none, or endpoint-only).
/// - `1` — single-point interior crossing handled.
/// - `2` — collinear overlap with shared left endpoint.
/// - `3` — collinear overlap, other configurations.
pub(crate) fn possible_intersection(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    se1_idx: usize,
    se2_idx: usize,
) -> i32 {
    /* Snapshot the relevant points up front; divide_segment will mutate the arena. */
    let se1_pt = arena[se1_idx].point;
    let se2_pt = arena[se2_idx].point;
    let se1_other_idx = arena[se1_idx]
        .other_event
        .expect("possible_intersection: se1 has no peer");
    let se2_other_idx = arena[se2_idx]
        .other_event
        .expect("possible_intersection: se2 has no peer");
    let se1_other_pt = arena[se1_other_idx].point;
    let se2_other_pt = arena[se2_other_idx].point;

    let inter = intersection(se1_pt, se1_other_pt, se2_pt, se2_other_pt, false);

    match inter {
        SegmentIntersection::None => 0,
        SegmentIntersection::Point(p) => {
            /*
             * Endpoint-only touch at *both* segments: upstream
             * short-circuits because the sweep already handles those
             * connections via the queue's left/right event ordering.
             */
            if equals(se1_pt, se2_pt) || equals(se1_other_pt, se2_other_pt) {
                return 0;
            }

            /* Subdivide each segment at the intersection if the
             * intersection isn't already one of its endpoints. */
            if !equals(se1_pt, p) && !equals(se1_other_pt, p) {
                let (l, r) = divide_segment(arena, se1_idx, p);
                queue.push(arena, l);
                queue.push(arena, r);
            }
            if !equals(se2_pt, p) && !equals(se2_other_pt, p) {
                let (l, r) = divide_segment(arena, se2_idx, p);
                queue.push(arena, l);
                queue.push(arena, r);
            }
            1
        }
        SegmentIntersection::Overlap(_, _) => {
            /*
             * Two intersection points = collinear overlap. Same-polygon
             * collinear overlap is "real" but harmless for the algorithm
             * (and indicates self-intersection in the input, which
             * upstream tolerates without subdivision).
             */
            if arena[se1_idx].is_subject() == arena[se2_idx].is_subject() {
                return 0;
            }

            handle_overlap(arena, queue, se1_idx, se2_idx)
        }
    }
}

/**********************************************************************
 * Internal — the four sub-cases of the collinear-overlap branch.
 *********************************************************************/

fn handle_overlap(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    se1_idx: usize,
    se2_idx: usize,
) -> i32 {
    let se1_other_idx = arena[se1_idx].other_event.unwrap();
    let se2_other_idx = arena[se2_idx].other_event.unwrap();

    /*
     * Build `events`: an ordered list of the (up to 4) distinct
     * endpoints involved. If two left endpoints coincide we set
     * `left_coincide` and skip pushing them; same for right endpoints
     * and `right_coincide`. The remaining endpoints get pushed in
     * compare_events order.
     */
    let mut events: Vec<usize> = Vec::with_capacity(4);
    let mut left_coincide = false;
    let mut right_coincide = false;

    if equals(arena[se1_idx].point, arena[se2_idx].point) {
        left_coincide = true;
    } else if compare_events(arena, se1_idx, se2_idx) == Ordering::Greater {
        events.push(se2_idx);
        events.push(se1_idx);
    } else {
        events.push(se1_idx);
        events.push(se2_idx);
    }

    if equals(arena[se1_other_idx].point, arena[se2_other_idx].point) {
        right_coincide = true;
    } else if compare_events(arena, se1_other_idx, se2_other_idx) == Ordering::Greater {
        events.push(se2_other_idx);
        events.push(se1_other_idx);
    } else {
        events.push(se1_other_idx);
        events.push(se2_other_idx);
    }

    /*
     * Case A: both endpoints coincide, OR only left coincides.
     * Mark se2 as non-contributing (it duplicates se1 in some sense)
     * and tag se1 as same-or-different transition based on whether
     * the two in_out flags agree.
     */
    if left_coincide {
        arena[se2_idx].edge_type = EdgeType::NonContributing;
        let se1_in_out = arena[se1_idx].in_out;
        let se2_in_out = arena[se2_idx].in_out;
        arena[se1_idx].edge_type = if se1_in_out == se2_in_out {
            EdgeType::SameTransition
        } else {
            EdgeType::DifferentTransition
        };

        if !right_coincide {
            /*
             * Left coincides but right doesn't ⇒ one segment is a
             * prefix of the other along the line. Split the longer
             * segment at the shorter's right endpoint. Upstream
             * notes "honestly no idea, but [0, 1] fixes the
             * overlapping self-intersecting polygons issue" — we
             * mirror its choice byte-for-byte.
             */
            let target_other = arena[events[1]].other_event.unwrap();
            let split_at = arena[events[0]].point;
            let (l, r) = divide_segment(arena, target_other, split_at);
            queue.push(arena, l);
            queue.push(arena, r);
        }
        return 2;
    }

    /*
     * Case B: only right endpoints coincide ⇒ one segment ends where
     * the other ends; split the longer (events[0]) at the shorter's
     * left endpoint (events[1]).
     */
    if right_coincide {
        let split_at = arena[events[1]].point;
        let (l, r) = divide_segment(arena, events[0], split_at);
        queue.push(arena, l);
        queue.push(arena, r);
        return 3;
    }

    /*
     * Case C: no endpoint coincidence. Distinguish "partial overlap"
     * from "one segment contains the other" by checking whether
     * events[0]'s segment is the same as events[3]'s segment (i.e.
     * one segment's left endpoint matches the other segment's right
     * endpoint via the peer link).
     */
    let events3_other = arena[events[3]].other_event.unwrap();
    if events[0] != events3_other {
        /* Partial overlap: split each at the other's nearest endpoint. */
        let p1 = arena[events[1]].point;
        let (l, r) = divide_segment(arena, events[0], p1);
        queue.push(arena, l);
        queue.push(arena, r);

        let p2 = arena[events[2]].point;
        let (l, r) = divide_segment(arena, events[1], p2);
        queue.push(arena, l);
        queue.push(arena, r);
        return 3;
    }

    /*
     * Case D: one segment fully contains the other. Split the outer
     * at the inner's left endpoint, then split the remaining outer-
     * right portion at the inner's right endpoint. Re-read
     * events[3].other_event for the second split because the first
     * one mutated it.
     */
    let p1 = arena[events[1]].point;
    let (l, r) = divide_segment(arena, events[0], p1);
    queue.push(arena, l);
    queue.push(arena, r);

    let events3_other_after = arena[events[3]].other_event.unwrap();
    let p2 = arena[events[2]].point;
    let (l, r) = divide_segment(arena, events3_other_after, p2);
    queue.push(arena, l);
    queue.push(arena, r);
    3
}

/**********************************************************************
 * Tests — direct unit coverage; upstream's tests for this module are
 * folded into divide_segment.test.ts as end-to-end fixtures we'll
 * eventually re-exercise via the parity harness.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sweep_event::PolygonType;

    fn add_segment(
        arena: &mut Vec<SweepEvent>,
        left_pt: [f64; 2],
        right_pt: [f64; 2],
        polygon_type: PolygonType,
    ) -> usize {
        let left_idx = arena.len();
        arena.push(SweepEvent::new(
            left_pt,
            true,
            polygon_type,
            EdgeType::Normal,
        ));
        let right_idx = arena.len();
        arena.push(SweepEvent::new(
            right_pt,
            false,
            polygon_type,
            EdgeType::Normal,
        ));
        arena[left_idx].other_event = Some(right_idx);
        arena[right_idx].other_event = Some(left_idx);
        left_idx
    }

    #[test]
    fn non_intersecting_segments_return_zero() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [1.0, 0.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [0.0, 1.0], [1.0, 1.0], PolygonType::Clipping);
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 0);
        assert!(q.is_empty());
    }

    #[test]
    fn interior_crossing_returns_one_and_creates_four_new_events() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [4.0, 4.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [0.0, 4.0], [4.0, 0.0], PolygonType::Clipping);
        let before = arena.len();
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 1);
        /* Each segment split ⇒ 2 new events per segment ⇒ 4 new events total. */
        assert_eq!(arena.len(), before + 4);
        assert_eq!(q.len(), 4);
    }

    #[test]
    fn shared_left_endpoint_only_returns_zero() {
        /* Both segments start at (0,0); intersection is exactly that endpoint. */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [4.0, 4.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [0.0, 0.0], [4.0, -4.0], PolygonType::Clipping);
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 0);
        assert!(q.is_empty());
    }

    #[test]
    fn same_polygon_overlap_returns_zero() {
        /*
         * Two overlapping segments from the same polygon. Upstream
         * treats this as harmless (signals a self-intersecting input
         * which Martinez tolerates).
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 0.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [2.0, 0.0], [7.0, 0.0], PolygonType::Subject);
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 0);
        assert!(q.is_empty());
    }

    #[test]
    fn collinear_overlap_different_polygons_marks_edge_types_and_splits() {
        /* Two collinear segments from different polygons, partial overlap. */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 0.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [2.0, 0.0], [7.0, 0.0], PolygonType::Clipping);
        let ret = possible_intersection(&mut arena, &mut q, a, b);
        /* No coincidence ⇒ case C (partial overlap) ⇒ returns 3. */
        assert_eq!(ret, 3);
        assert!(!q.is_empty());
    }

    #[test]
    fn collinear_left_coincident_marks_non_contributing() {
        /*
         * Segments share their left endpoint, different polygons.
         * Returns 2. se2 should be marked NonContributing.
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 0.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [0.0, 0.0], [3.0, 0.0], PolygonType::Clipping);
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 2);
        assert_eq!(arena[b].edge_type, EdgeType::NonContributing);
        assert!(matches!(
            arena[a].edge_type,
            EdgeType::SameTransition | EdgeType::DifferentTransition
        ));
    }

    #[test]
    fn collinear_right_coincident_only_splits_once() {
        /*
         * Segments share their right endpoint, different polygons.
         * Case B ⇒ returns 3, splits events[0] at events[1].point.
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 0.0], PolygonType::Subject);
        let b = add_segment(&mut arena, [2.0, 0.0], [5.0, 0.0], PolygonType::Clipping);
        let before = arena.len();
        assert_eq!(possible_intersection(&mut arena, &mut q, a, b), 3);
        /* One divide_segment ⇒ 2 new events. */
        assert_eq!(arena.len(), before + 2);
    }
}
