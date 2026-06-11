//! Segment subdivision at intersection points, port of upstream
//! `src/divide_segment.ts`.
//!
//! When the sweep discovers that two segments intersect, each segment
//! needs to be split at the intersection point. `divide_segment`
//! mutates the arena: it appends two new `SweepEvent`s representing
//! the two halves of the original segment that meet at the new
//! point, patches the `other_event` links so the original peer event
//! now refers to the new "left" half, and returns the indices of the
//! freshly created events so the caller can push them onto the event
//! queue.
//!
//! **API shape vs upstream.** Upstream's `divideSegment(se, p, queue)`
//! takes the queue and pushes the new events itself. We split that
//! concern: this function does the arena surgery, the caller does
//! queue insertion. Cleaner separation, easier to test without
//! committing to a queue type yet.

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::compare_events::compare_events;
use crate::edge_type::EdgeType;
#[cfg(debug_assertions)]
use crate::equals::equals;
use crate::sweep_event::SweepEvent;
use crate::types::Position;

/// Split the segment associated with `event_idx` at point `p`.
///
/// Mutates `arena`: appends two new events (a right endpoint at `p`
/// for the original event's segment, and a left endpoint at `p` for
/// the original peer's segment), and re-wires the `other_event`
/// links so both halves are properly connected.
///
/// Returns `(left_event_idx, right_event_idx)`. Neither is pushed
/// onto any queue — that's the caller's job.
pub(crate) fn divide_segment(
    arena: &mut Vec<SweepEvent>,
    event_idx: usize,
    p: Position,
) -> (usize, usize) {
    let polygon_type = arena[event_idx].polygon_type;
    let contour_id = arena[event_idx].contour_id;
    let other_event_idx = arena[event_idx]
        .other_event
        .expect("divide_segment: event has no peer");

    /*
     * Diagnostic from upstream: a "collapsed segment" (both endpoints
     * at the same point) usually signals a bug in the caller. We
     * surface it only in debug builds to avoid log spam in release.
     */
    #[cfg(debug_assertions)]
    if equals(arena[event_idx].point, arena[other_event_idx].point) {
        eprintln!(
            "divide_segment: collapsed segment at {:?}",
            arena[event_idx].point,
        );
    }

    /*
     * Construct the two new events. They share `contour_id` with the
     * original; `polygon_type` is also inherited so downstream
     * `compute_fields` sees a consistent classification.
     */
    let r_idx = arena.len();
    let mut r = SweepEvent::new(p, false, polygon_type, EdgeType::Normal);
    r.other_event = Some(event_idx);
    r.contour_id = contour_id;
    arena.push(r);

    let l_idx = arena.len();
    let mut l = SweepEvent::new(p, true, polygon_type, EdgeType::Normal);
    l.other_event = Some(other_event_idx);
    l.contour_id = contour_id;
    arena.push(l);

    /*
     * Rounding-error guard from upstream. If the freshly-created `l`
     * would, by `compare_events` ordering, be processed *after* the
     * original peer, the sweep would visit `l` (a "left" event) after
     * already having processed its partner — geometrically wrong. The
     * fix is to swap their leftness so the original peer becomes the
     * left endpoint of the back-half segment instead.
     *
     * Order matters: this comparison runs *before* the link patches
     * below, matching upstream's sequencing. `compare_events` reads
     * `other_event` during the collinear-tiebreak branch; doing the
     * patch first would feed it the wrong topology.
     */
    if compare_events(arena, l_idx, other_event_idx) == Ordering::Greater {
        arena[other_event_idx].left = true;
        arena[l_idx].left = false;
    }

    /*
     * Re-wire the peer's link: the original right-endpoint event now
     * pairs with the freshly-minted `l` (left half of the back portion
     * of the segment) instead of with the original event. And the
     * original event pairs with `r`.
     */
    arena[other_event_idx].other_event = Some(l_idx);
    arena[event_idx].other_event = Some(r_idx);

    (l_idx, r_idx)
}

/**********************************************************************
 * Tests — modeled on upstream `test/divide_segment.test.ts`. The
 * second and third upstream cases drag in `possible_intersection`,
 * `fill_queue`, and `subdivide_segments` (still stubs here), so we
 * cover only the directly-portable first case plus arena-state
 * checks that the upstream test asserts implicitly via `q.length`.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sweep_event::PolygonType;

    /*
     * Set up a single segment `(left_pt → right_pt)` in the arena and
     * return the index of the left event. Cross-links `other_event`
     * for both endpoints.
     */
    fn add_segment(
        arena: &mut Vec<SweepEvent>,
        left_pt: Position,
        right_pt: Position,
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
    fn dividing_two_segments_at_their_intersection_produces_six_events() {
        /*
         * Mirrors upstream's "should divide 2 segments" — two
         * segments (0,0)→(5,5) and (0,5)→(5,0) crossing at (2.5, 2.5).
         * Each gets split at the crossing, producing 4 + 4 = 8 events.
         * Upstream pushed only the left events into the queue
         * initially (2), then `divide_segment` adds 2 each (total 6
         * in the queue). Here we verify the arena state instead.
         */
        let mut arena = Vec::new();
        let se1 = add_segment(&mut arena, [0.0, 0.0], [5.0, 5.0], PolygonType::Subject);
        let se2 = add_segment(&mut arena, [0.0, 5.0], [5.0, 0.0], PolygonType::Clipping);
        assert_eq!(arena.len(), 4);

        let cross = [2.5, 2.5];
        let (l1, r1) = divide_segment(&mut arena, se1, cross);
        let (l2, r2) = divide_segment(&mut arena, se2, cross);

        /* Arena grew by 4: 2 per division. */
        assert_eq!(arena.len(), 8);

        /* Each new event sits at the intersection point. */
        for idx in [l1, r1, l2, r2] {
            assert_eq!(arena[idx].point, cross);
        }
    }

    #[test]
    fn divide_segment_patches_links_correctly() {
        let mut arena = Vec::new();
        let se = add_segment(&mut arena, [0.0, 0.0], [10.0, 10.0], PolygonType::Subject);
        let original_other = arena[se].other_event.unwrap();
        let p = [3.0, 3.0];

        let (l, r) = divide_segment(&mut arena, se, p);

        /* New events r and l point at the right halves of the link chain. */
        assert_eq!(arena[r].other_event, Some(se));
        assert_eq!(arena[l].other_event, Some(original_other));

        /* The original event now pairs with r, not its old peer. */
        assert_eq!(arena[se].other_event, Some(r));

        /* The original peer now pairs with l, not se. */
        assert_eq!(arena[original_other].other_event, Some(l));
    }

    #[test]
    fn divide_segment_propagates_contour_id_and_polygon_type() {
        let mut arena = Vec::new();
        let se = add_segment(&mut arena, [0.0, 0.0], [4.0, 4.0], PolygonType::Clipping);
        arena[se].contour_id = Some(7);

        let (l, r) = divide_segment(&mut arena, se, [2.0, 2.0]);
        assert_eq!(arena[l].contour_id, Some(7));
        assert_eq!(arena[r].contour_id, Some(7));
        assert_eq!(arena[l].polygon_type, PolygonType::Clipping);
        assert_eq!(arena[r].polygon_type, PolygonType::Clipping);
    }

    #[test]
    fn divided_events_are_left_right_at_split_point() {
        /*
         * Default (non-rounding-error) case: `l` is a left event,
         * `r` is a right event. The rounding-error branch only fires
         * when `compare_events(l, original_other) > 0`, which
         * shouldn't happen for normal interior splits.
         */
        let mut arena = Vec::new();
        let se = add_segment(&mut arena, [0.0, 0.0], [10.0, 10.0], PolygonType::Subject);
        let (l, r) = divide_segment(&mut arena, se, [3.0, 3.0]);

        assert!(arena[l].left);
        assert!(!arena[r].left);
    }
}
