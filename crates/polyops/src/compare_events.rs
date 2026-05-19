//! Sweep event ordering for the priority queue, port of upstream
//! `src/compare_events.ts`.
//!
//! Returns `Less`/`Greater` only — never `Equal`. Two events are
//! considered the same only when they're the same arena entry, and
//! [`std::collections::BinaryHeap`] only needs strict ordering. If a
//! future parity bug shows that two genuinely-distinct events compare
//! equal here, the fix is to add an insertion-sequence tiebreak — see
//! `PLAN.md` §14 risks.

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::signed_area::signed_area;
use crate::sweep_event::SweepEvent;

/// Compare two sweep events `i1` and `i2` by their position in the
/// priority queue.
///
/// **Priority key**, in order:
///
/// 1. Smaller x first.
/// 2. Smaller y first (when x ties).
/// 3. Right endpoint before left (when both points tie).
/// 4. The event whose segment lies *below* the other comes first
///    (when both points tie and both are left or both are right).
/// 5. Subject before clipping (when segments are collinear).
pub(crate) fn compare_events(arena: &[SweepEvent], i1: usize, i2: usize) -> Ordering {
    let e1 = &arena[i1];
    let e2 = &arena[i2];
    let p1 = e1.point;
    let p2 = e2.point;

    /* 1. x-coordinate is the dominant key. */
    if p1[0] > p2[0] {
        return Ordering::Greater;
    }
    if p1[0] < p2[0] {
        return Ordering::Less;
    }

    /* 2. Same x: lower y is processed first. */
    if p1[1] != p2[1] {
        return if p1[1] > p2[1] {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    special_cases(arena, e1, e2)
}

/*
 * Internal — same-point tiebreaking.
 */
fn special_cases(arena: &[SweepEvent], e1: &SweepEvent, e2: &SweepEvent) -> Ordering {
    /*
     * 3. Same coordinates but one is a left endpoint and the other a
     *    right endpoint. Upstream comment: "The right endpoint is
     *    processed first." In our return convention that means the
     *    right endpoint should sort `Less`.
     */
    if e1.left != e2.left {
        return if e1.left {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    /*
     * 4. Same coords, both same side. Look at the other endpoint of
     *    each segment. If the three points aren't collinear, the
     *    segment that lies *below* the other goes first.
     */
    let o1 = arena[e1.other_event.expect("compare_events: e1 has no peer")].point;
    let o2 = arena[e2.other_event.expect("compare_events: e2 has no peer")].point;
    if signed_area(e1.point, o1, o2) != 0 {
        /*
         * Upstream: `(!e1.isBelow(e2.otherEvent.point)) ? 1 : -1`.
         * `is_below` is geometric: true means e1's segment is below
         * the query point. Negation: e1 is above ⇒ e1 sorts greater.
         */
        return if !e1.is_below(o1, o2) {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }

    /*
     * 5. Collinear: subject sorts before clipping. Trace from
     *    upstream `(!e1.isSubject && e2.isSubject) ? 1 : -1`:
     *    - subject vs clipping: subject first (Less).
     *    - clipping vs subject: clipping last (Greater).
     *    - same polygon type: arbitrary but deterministic (Less).
     */
    if !e1.is_subject() && e2.is_subject() {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

/*
 * Tests — mirror upstream `test/compare_events.test.ts` 1:1.
 */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_type::EdgeType;
    use crate::sweep_event::PolygonType;

    /*
     * Test helper: append a left/right event pair representing the
     * directed segment from `left_pt` to `right_pt` of polygon
     * `polygon_type`, with `other_event` cross-linked. Returns the
     * `(left_idx, right_idx)` pair.
     */
    fn add_segment(
        arena: &mut Vec<SweepEvent>,
        left_pt: [f64; 2],
        right_pt: [f64; 2],
        polygon_type: PolygonType,
    ) -> (usize, usize) {
        let left_idx = arena.len();
        arena.push(SweepEvent::new(left_pt, true, polygon_type, EdgeType::Normal));
        let right_idx = arena.len();
        arena.push(SweepEvent::new(
            right_pt,
            false,
            polygon_type,
            EdgeType::Normal,
        ));
        arena[left_idx].other_event = Some(right_idx);
        arena[right_idx].other_event = Some(left_idx);
        (left_idx, right_idx)
    }

    /*
     * Convenience: append a single event with no peer. Used when
     * upstream tests construct stub objects that the comparator never
     * dereferences `otherEvent` on.
     */
    fn add_bare_event(
        arena: &mut Vec<SweepEvent>,
        point: [f64; 2],
        left: bool,
    ) -> usize {
        let idx = arena.len();
        arena.push(SweepEvent::new(
            point,
            left,
            PolygonType::Subject,
            EdgeType::Normal,
        ));
        idx
    }

    #[test]
    fn compares_x_coordinates() {
        let mut arena = Vec::new();
        let e1 = add_bare_event(&mut arena, [0.0, 0.0], true);
        let e2 = add_bare_event(&mut arena, [0.5, 0.5], true);
        assert_eq!(compare_events(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_events(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn compares_y_coordinates_when_x_ties() {
        let mut arena = Vec::new();
        let e1 = add_bare_event(&mut arena, [0.0, 0.0], true);
        let e2 = add_bare_event(&mut arena, [0.0, 0.5], true);
        assert_eq!(compare_events(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_events(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn processes_right_endpoint_before_left_when_points_tie() {
        /*
         * Upstream test: when same point + different `left`, the
         * not-left event sorts first (Less). e1.left=true, e2.left=false,
         * so compare(e1, e2) = Greater (the left one sorts later).
         */
        let mut arena = Vec::new();
        let e1 = add_bare_event(&mut arena, [0.0, 0.0], true);
        let e2 = add_bare_event(&mut arena, [0.0, 0.0], false);
        assert_eq!(compare_events(&arena, e1, e2), Ordering::Greater);
        assert_eq!(compare_events(&arena, e2, e1), Ordering::Less);
    }

    #[test]
    fn shared_start_not_collinear_processes_lower_edge_first() {
        /*
         * Upstream test "should process lower edge first": both events
         * start at (0,0); e1 ends at (1,1), e2 ends at (2,3). e1's
         * segment is below e2's. e1 sorts first (Less).
         */
        let mut arena = Vec::new();
        let (e1, _) = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        let (e2, _) = add_segment(&mut arena, [0.0, 0.0], [2.0, 3.0], PolygonType::Subject);
        assert_eq!(compare_events(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_events(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn collinear_subject_sorts_before_clipping() {
        /*
         * Upstream test (despite its misleading "should process
         * clipping before subject" name): when both segments are
         * collinear at the same point, *subject* is processed before
         * *clipping*. Trace: (!e1.isSubject && e2.isSubject) is false
         * when e1 is subject, so returns -1 ⇒ Ordering::Less.
         */
        let mut arena = Vec::new();
        let (e1, _) = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        let (e2, _) = add_segment(&mut arena, [0.0, 0.0], [2.0, 2.0], PolygonType::Clipping);
        assert_eq!(compare_events(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_events(&arena, e2, e1), Ordering::Greater);
    }
}
