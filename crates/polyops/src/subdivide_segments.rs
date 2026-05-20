//! Main sweep loop, port of upstream `src/subdivide_segments.ts`.
//!
//! Drives the algorithm: pops events from the priority queue, keeps
//! the [`crate::sweep_line::SweepLine`] status structure in sync,
//! invokes [`crate::possible_intersection`] on neighbors, calls
//! [`crate::compute_fields`] to classify events, and returns the
//! complete sorted list of (subdivided) event indices for
//! [`crate::connect_edges`] to stitch into output contours.
//!
//! **API divergence from upstream.** The upstream signature takes
//! `subject` and `clipping` MultiPolygons but never reads them inside
//! the function body; we drop them. Otherwise the structure mirrors
//! upstream branch-for-branch.

#![allow(dead_code)]

use crate::compute_fields::compute_fields;
use crate::event_queue::EventQueue;
use crate::operation::Operation;
use crate::possible_intersection::possible_intersection;
use crate::sweep_event::SweepEvent;
use crate::sweep_line::SweepLine;
use crate::types::BBox;

/// Run the sweep. Returns the full list of event indices in the
/// order they were popped — the input to `connect_edges`.
pub(crate) fn subdivide_segments(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    sbbox: BBox,
    cbbox: BBox,
    operation: Operation,
) -> Vec<usize> {
    let mut sweep_line = SweepLine::new();
    let mut sorted_events: Vec<usize> = Vec::new();

    /*
     * Bbox optimization: once the sweep x passes the right bound of
     * the *smaller* input bbox (for intersection) or the subject's
     * right bound (for difference), no more output is possible.
     */
    let rightbound = sbbox[2].min(cbbox[2]);

    while !queue.is_empty() {
        let event_idx = queue.pop(arena).expect("queue.pop on non-empty queue");
        sorted_events.push(event_idx);

        let event_x = arena[event_idx].point[0];
        if (operation == Operation::Intersection && event_x > rightbound)
            || (operation == Operation::Difference && event_x > sbbox[2])
        {
            break;
        }

        if arena[event_idx].left {
            sweep_line_insert_path(arena, queue, &mut sweep_line, event_idx, operation);
        } else {
            sweep_line_remove_path(arena, queue, &mut sweep_line, event_idx);
        }
    }

    sorted_events
}

/**********************************************************************
 * Internal helpers — alphabetical.
 *********************************************************************/

/// Process a left event: insert it onto the sweep line, classify it
/// against its predecessor, and check for intersections with the
/// neighbors that just came into contact.
fn sweep_line_insert_path(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    sweep_line: &mut SweepLine,
    event_idx: usize,
    operation: Operation,
) {
    let position = sweep_line.insert(arena, event_idx);
    let prev_event = sweep_line.prev(position);
    let next_event = sweep_line.next(position);

    compute_fields(arena, event_idx, prev_event, operation);

    /*
     * Test against the segment immediately above on the sweep line.
     * Return code 2 from possible_intersection means a collinear
     * overlap was processed; the new edge_types invalidate our
     * just-computed fields, so we recompute for both events.
     */
    if let Some(next) = next_event {
        if possible_intersection(arena, queue, event_idx, next) == 2 {
            compute_fields(arena, event_idx, prev_event, operation);
            compute_fields(arena, next, Some(event_idx), operation);
        }
    }

    /*
     * Same check against the segment immediately below. The
     * recompute targets are different (prev's own predecessor for
     * prev, and prev for event_idx), matching upstream's structure.
     */
    if let Some(prev) = prev_event {
        if possible_intersection(arena, queue, prev, event_idx) == 2 {
            /*
             * "prevprev" is the segment below prev. None if prev is
             * the bottom of the sweep line (position 1 ⇒ prev at 0).
             */
            let prevprev_event = if position >= 2 {
                sweep_line.at(position - 2)
            } else {
                None
            };
            compute_fields(arena, prev, prevprev_event, operation);
            compute_fields(arena, event_idx, Some(prev), operation);
        }
    }
}

/// Process a right event: find its corresponding left event in the
/// sweep line, remove it, and check whether the neighbors just left
/// behind now need to be tested for intersection.
fn sweep_line_remove_path(
    arena: &mut Vec<SweepEvent>,
    queue: &mut EventQueue,
    sweep_line: &mut SweepLine,
    event_idx: usize,
) {
    let left_event_idx = arena[event_idx]
        .other_event
        .expect("subdivide_segments: right event has no peer");

    /*
     * The left event may have been swept past already (e.g. if a
     * divide pulled it out). If it's not on the sweep line, there's
     * nothing to remove.
     */
    let Some(position) = sweep_line.position(arena, left_event_idx) else {
        return;
    };

    let prev_event = sweep_line.prev(position);
    let next_event = sweep_line.next(position);
    sweep_line.remove_at(position);

    /*
     * Removing the left event brings prev and next into contact;
     * test them for a fresh intersection.
     */
    if let (Some(prev), Some(next)) = (prev_event, next_event) {
        possible_intersection(arena, queue, prev, next);
    }
}

/**********************************************************************
 * Tests — direct smoke tests of the orchestration. End-to-end
 * geometric parity is verified later by the parity harness once
 * connect_edges + boolean_op are in place.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fill_queue::fill_queue;
    use crate::types::MultiPolygon;

    fn empty_bbox() -> BBox {
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ]
    }

    fn unit_square_at(origin: [f64; 2]) -> MultiPolygon {
        let [x, y] = origin;
        vec![vec![vec![
            [x, y],
            [x + 1.0, y],
            [x + 1.0, y + 1.0],
            [x, y + 1.0],
            [x, y],
        ]]]
    }

    #[test]
    fn empty_queue_returns_empty_sorted_events() {
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let result = subdivide_segments(
            &mut arena,
            &mut q,
            empty_bbox(),
            empty_bbox(),
            Operation::Union,
        );
        assert!(result.is_empty());
    }

    #[test]
    fn single_subject_drains_the_queue() {
        /*
         * One subject polygon, no clipping. The sweep should drain
         * every event from the queue and return them all in popped
         * order.
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &unit_square_at([0.0, 0.0]),
            &vec![],
            &mut sbb,
            &mut cbb,
            Operation::Union,
        );
        let event_count_before = arena.len();
        let result = subdivide_segments(&mut arena, &mut q, sbb, cbb, Operation::Union);
        /* No intersections ⇒ event count unchanged. All events in result. */
        assert_eq!(arena.len(), event_count_before);
        assert_eq!(result.len(), event_count_before);
    }

    #[test]
    fn two_overlapping_squares_produce_extra_events_from_intersection() {
        /*
         * Two unit squares overlapping in their interior should
         * produce intersection points along each pair of crossing
         * edges. The exact count depends on the algorithm, but
         * arena.len() should grow beyond the initial 16 events
         * (4 edges × 2 events × 2 squares).
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &unit_square_at([0.0, 0.0]),
            &unit_square_at([0.5, 0.5]),
            &mut sbb,
            &mut cbb,
            Operation::Union,
        );
        let initial_event_count = arena.len();
        assert_eq!(initial_event_count, 16);

        let result = subdivide_segments(&mut arena, &mut q, sbb, cbb, Operation::Union);
        /* Intersections add events. */
        assert!(arena.len() > initial_event_count);
        assert!(!result.is_empty());
    }

    #[test]
    fn intersection_bbox_shortcut_stops_early() {
        /*
         * Two disjoint squares — for intersection there's no output
         * possible. The sweep should still drain events but the
         * rightbound break kicks in once x passes min(sbbox.maxX,
         * cbbox.maxX) = 1.0 (left square's max x).
         */
        let mut arena = Vec::new();
        let mut q = EventQueue::new();
        let mut sbb = empty_bbox();
        let mut cbb = empty_bbox();
        fill_queue(
            &mut arena,
            &mut q,
            &unit_square_at([0.0, 0.0]),
            &unit_square_at([5.0, 0.0]),
            &mut sbb,
            &mut cbb,
            Operation::Intersection,
        );
        let result = subdivide_segments(&mut arena, &mut q, sbb, cbb, Operation::Intersection);
        /* Early break ⇒ result is shorter than total event count. */
        assert!(result.len() < arena.len());
    }
}
