//! Initial event-queue population, port of upstream `src/fill_queue.ts`.
//!
//! Walks every ring of every polygon in both inputs. For each
//! ring-edge, creates the pair of [`SweepEvent`]s representing its
//! two endpoints, sets the `left` flag correctly via
//! [`crate::compare_events`], cross-links `other_event`, and pushes
//! both onto the priority queue. Also accumulates a per-input
//! bounding box during the walk for later trivial-case shortcuts in
//! [`crate::lib::boolean_op`].
//!
//! Collapsed edges (both endpoints at the same position) are
//! silently skipped — they're degenerate and would break the sweep.
//!
//! **API divergence from upstream.** Upstream uses a module-level
//! mutable `contourId` counter that persists across `fillQueue`
//! invocations. Per-invocation `contour_id` values are only compared
//! to each other within a single sweep, so resetting to 0 at the
//! start of every `fill_queue` is semantically equivalent and
//! avoids global state.

#![allow(dead_code)]

use crate::compare_events::compare_events;
use crate::edge_type::EdgeType;
use crate::event_queue::EventQueue;
use crate::operation::Operation;
use crate::sweep_event::{PolygonType, SweepEvent};
use crate::types::{BBox, MultiPolygon, Ring};
use std::cmp::Ordering;

/// Build the initial sweep-event priority queue from the two input
/// multipolygons. Mutates `arena` (appends events), `queue` (pushes
/// indices), and the bounding-box parameters.
pub(crate) fn fill_queue(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    subject: &MultiPolygon,
    clipping: &MultiPolygon,
    sbbox: &mut BBox,
    cbbox: &mut BBox,
    operation: Operation,
) {
    let mut contour_id: i32 = 0;

    /* Subject polygons. */
    for polygon in subject {
        for (ring_idx, ring) in polygon.iter().enumerate() {
            let is_exterior_ring = ring_idx == 0;
            if is_exterior_ring {
                contour_id += 1;
            }
            process_polygon(
                arena,
                queue,
                ring,
                PolygonType::Subject,
                contour_id,
                sbbox,
                is_exterior_ring,
            );
        }
    }

    /* Clipping polygons. */
    for polygon in clipping {
        for (ring_idx, ring) in polygon.iter().enumerate() {
            let mut is_exterior_ring = ring_idx == 0;
            /*
             * For DIFFERENCE, the clipping polygon is being subtracted:
             * its rings effectively flip role, so the exterior ring is
             * treated as a hole-equivalent for downstream classification.
             */
            if operation == Operation::Difference {
                is_exterior_ring = false;
            }
            if is_exterior_ring {
                contour_id += 1;
            }
            process_polygon(
                arena,
                queue,
                ring,
                PolygonType::Clipping,
                contour_id,
                cbbox,
                is_exterior_ring,
            );
        }
    }
}

/**********************************************************************
 * Internal helpers.
 *********************************************************************/

fn process_polygon(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    ring: &Ring,
    polygon_type: PolygonType,
    contour_id: i32,
    bbox: &mut BBox,
    is_exterior_ring: bool,
) {
    /*
     * Walk consecutive pairs of vertices. By GeoJSON convention rings
     * are closed (last point repeats the first), so iterating up to
     * len - 1 gives every edge exactly once.
     */
    if ring.len() < 2 {
        return;
    }
    for window in ring.windows(2) {
        let s1 = window[0];
        let s2 = window[1];

        /* Skip degenerate edges — same point twice. */
        if s1[0] == s2[0] && s1[1] == s2[1] {
            continue;
        }

        let e1_idx = arena.len();
        arena.push(SweepEvent::new(s1, false, polygon_type, EdgeType::Normal));
        let e2_idx = arena.len();
        arena.push(SweepEvent::new(s2, false, polygon_type, EdgeType::Normal));
        arena[e1_idx].other_event = Some(e2_idx);
        arena[e2_idx].other_event = Some(e1_idx);

        arena[e1_idx].contour_id = Some(contour_id);
        arena[e2_idx].contour_id = Some(contour_id);
        if !is_exterior_ring {
            arena[e1_idx].is_exterior_ring = false;
            arena[e2_idx].is_exterior_ring = false;
        }

        /*
         * Decide which endpoint is the "left" event. Whichever sorts
         * earlier under compare_events is left; the other is right.
         */
        if compare_events(arena, e1_idx, e2_idx) == Ordering::Greater {
            arena[e2_idx].left = true;
        } else {
            arena[e1_idx].left = true;
        }

        /* Bounding box accumulation. */
        let (x, y) = (s1[0], s1[1]);
        bbox[0] = bbox[0].min(x);
        bbox[1] = bbox[1].min(y);
        bbox[2] = bbox[2].max(x);
        bbox[3] = bbox[3].max(y);

        queue.push(arena, e1_idx);
        queue.push(arena, e2_idx);
    }
}

/**********************************************************************
 * Tests — upstream's fill_queue isn't tested in isolation; it's
 * exercised through subdivide_segments. These tests verify the
 * surface invariants: event count, link integrity, bbox, exterior
 * flag for holes, and DIFFERENCE's clipping-flip.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    fn empty_bbox() -> BBox {
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ]
    }

    /// One triangle polygon as a MultiPolygon.
    fn unit_triangle() -> MultiPolygon {
        vec![vec![vec![[0.0, 0.0], [4.0, 0.0], [2.0, 3.0], [0.0, 0.0]]]]
    }

    #[test]
    fn empty_inputs_produce_no_events() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &vec![],
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );
        assert!(arena.is_empty());
        assert!(q.is_empty());
    }

    #[test]
    fn single_subject_triangle_produces_six_events_and_correct_bbox() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &unit_triangle(),
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );
        /* 3 edges × 2 events per edge = 6 events. */
        assert_eq!(arena.len(), 6);
        assert_eq!(q.len(), 6);

        /* Triangle vertices (0,0), (4,0), (2,3) ⇒ bbox [0,0,4,3]. */
        assert_eq!(sbb, [0.0, 0.0, 4.0, 3.0]);

        /* Clipping was empty ⇒ cbbox untouched. */
        assert_eq!(cbb, empty_bbox());
    }

    #[test]
    fn collapsed_edges_are_skipped() {
        /* Ring with a repeated vertex producing a zero-length edge. */
        let ring = vec![[0.0, 0.0], [1.0, 1.0], [1.0, 1.0], [2.0, 0.0], [0.0, 0.0]];
        let poly: MultiPolygon = vec![vec![ring]];

        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &poly,
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );
        /* 4 vertex-pairs in the windows, 1 collapsed ⇒ 3 valid edges ⇒ 6 events. */
        assert_eq!(arena.len(), 6);
    }

    #[test]
    fn each_edge_yields_one_left_and_one_right_event() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &unit_triangle(),
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );

        /* Events come in pairs (i, i+1) per edge. Each pair has exactly one left. */
        for pair in (0..arena.len()).step_by(2) {
            let left_count = (arena[pair].left as u8) + (arena[pair + 1].left as u8);
            assert_eq!(
                left_count,
                1,
                "edge events at indices {pair}, {} should be one left + one right",
                pair + 1
            );
            /* And other_event cross-links. */
            assert_eq!(arena[pair].other_event, Some(pair + 1));
            assert_eq!(arena[pair + 1].other_event, Some(pair));
        }
    }

    #[test]
    fn holes_have_is_exterior_ring_false() {
        /* Polygon with an exterior ring and one hole. */
        let exterior = vec![
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
            [0.0, 0.0],
        ];
        let hole = vec![[2.0, 2.0], [4.0, 2.0], [4.0, 4.0], [2.0, 4.0], [2.0, 2.0]];
        let poly: MultiPolygon = vec![vec![exterior, hole]];

        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &poly,
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );

        /* First 8 events belong to exterior (4 edges × 2). Last 8 to the hole. */
        for ev in arena.iter().take(8) {
            assert!(ev.is_exterior_ring);
        }
        for ev in arena.iter().skip(8) {
            assert!(!ev.is_exterior_ring);
        }
    }

    #[test]
    fn difference_flips_clipping_exterior_to_non_exterior() {
        let subject = unit_triangle();
        let clipping = unit_triangle();

        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &subject,
            &clipping,
            &mut sbb,
            &mut cbb,
            Operation::Difference,
        );

        /* Subject events: indices 0..6, all is_exterior_ring=true. */
        for ev in arena.iter().take(6) {
            assert!(
                ev.is_exterior_ring,
                "subject events should keep exterior flag"
            );
        }
        /* Clipping events: indices 6..12, all is_exterior_ring=false under DIFFERENCE. */
        for ev in arena.iter().skip(6) {
            assert!(
                !ev.is_exterior_ring,
                "clipping events should be flipped under DIFFERENCE"
            );
        }
    }

    #[test]
    fn each_polygon_gets_distinct_contour_id() {
        /* Two separate subject polygons in a MultiPolygon. */
        let mp: MultiPolygon = vec![
            vec![vec![[0.0, 0.0], [1.0, 0.0], [0.5, 1.0], [0.0, 0.0]]],
            vec![vec![[2.0, 0.0], [3.0, 0.0], [2.5, 1.0], [2.0, 0.0]]],
        ];
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &mp,
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );
        /* First polygon's events should share contour_id == 1, second == 2. */
        for ev in arena.iter().take(6) {
            assert_eq!(ev.contour_id, Some(1));
        }
        for ev in arena.iter().skip(6) {
            assert_eq!(ev.contour_id, Some(2));
        }
    }
}
