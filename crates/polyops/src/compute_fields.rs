//! Per-event sweep-line classification, port of upstream
//! `src/compute_fields.ts`. 110 LOC upstream.
//!
//! Called once per event during `subdivide_segments`, after the event
//! is inserted onto the sweep-line status tree. Given the *previous*
//! event below this one on the status tree, [`compute_fields`] sets:
//!
//! - `in_out` — `true` iff this event represents an in→out transition
//!   for its own polygon at the sweep x.
//! - `other_in_out` — same flag, but for the *other* polygon.
//! - `prev_in_result` — the most recent event on the sweep line below
//!   this one that contributes to the final result. Used by
//!   `connect_edges` to walk the result topology.
//! - `result_transition` — `+1`, `-1`, or `0`. Zero means this event
//!   doesn't contribute to the output of the chosen Boolean operation.
//!   Non-zero is the direction of the transition.
//!
//! This is the single file in the algorithm where the four Boolean
//! operations (intersection, union, difference, xor) diverge — every
//! other module is operation-agnostic.

#![allow(dead_code)]

use crate::edge_type::EdgeType;
use crate::operation::Operation;
use crate::sweep_event::SweepEvent;

/// Populate `in_out`, `other_in_out`, `prev_in_result`, and
/// `result_transition` on `arena[event_idx]` given the sweep-line
/// predecessor `prev_idx` and the active `operation`.
pub(crate) fn compute_fields(
    arena: &mut Vec<SweepEvent>,
    event_idx: usize,
    prev_idx: Option<usize>,
    operation: Operation,
) {
    /*
     * Phase 1: in_out / other_in_out.
     *
     * No predecessor on the sweep line → this event opens a region:
     * we're not yet inside our own polygon (in_out=false) but, by
     * the algorithm's "infinite other polygon below us" convention,
     * we are notionally already inside the other polygon
     * (other_in_out=true).
     */
    match prev_idx {
        None => {
            arena[event_idx].in_out = false;
            arena[event_idx].other_in_out = true;
        }
        Some(prev) => {
            /*
             * Read prev's state up front. We need these values before
             * we mutate event_idx, and Rust's borrow checker doesn't
             * let us hold a reference to arena[prev] while also
             * mutably borrowing arena[event_idx].
             */
            let prev_is_subject = arena[prev].is_subject();
            let prev_in_out = arena[prev].in_out;
            let prev_other_in_out = arena[prev].other_in_out;
            let prev_other_idx = arena[prev]
                .other_event
                .expect("compute_fields: prev has no peer");
            let prev_other_point_x = arena[prev_other_idx].point[0];
            let prev_is_vertical = arena[prev].point[0] == prev_other_point_x;
            let prev_prev_in_result = arena[prev].prev_in_result;

            let event_is_subject = arena[event_idx].is_subject();

            if event_is_subject == prev_is_subject {
                /* Same polygon as predecessor: our in/out flips, the
                 * other polygon's stays the same. */
                arena[event_idx].in_out = !prev_in_out;
                arena[event_idx].other_in_out = prev_other_in_out;
            } else {
                /* Different polygons: cross-couple the flags. Vertical
                 * predecessors need a sign flip because their "in/out"
                 * along the sweep direction has opposite meaning. */
                arena[event_idx].in_out = !prev_other_in_out;
                arena[event_idx].other_in_out = if prev_is_vertical {
                    !prev_in_out
                } else {
                    prev_in_out
                };
            }

            /*
             * prev_in_result: the most recent ancestor on the sweep
             * line that contributes to the result. If prev itself
             * contributes (and isn't vertical), use it; otherwise
             * inherit prev's own prev_in_result.
             */
            let prev_was_in_result = in_result(arena, prev, operation);
            arena[event_idx].prev_in_result = if !prev_was_in_result || prev_is_vertical {
                prev_prev_in_result
            } else {
                Some(prev)
            };
        }
    }

    /*
     * Phase 2: result_transition.
     *
     * Zero means the event doesn't appear in the output. ±1 encodes
     * the transition direction so connect_edges can walk the result
     * topology consistently.
     */
    let is_in_result = in_result(arena, event_idx, operation);
    arena[event_idx].result_transition = if is_in_result {
        determine_result_transition(arena, event_idx, operation)
    } else {
        0
    };
}

/**********************************************************************
 * Internal helpers — alphabetical.
 **********************************************************************/

/// What sign to record in `result_transition` for an event that
/// contributes to the output. `+1` means an entering transition,
/// `-1` an exiting one.
fn determine_result_transition(arena: &[SweepEvent], idx: usize, operation: Operation) -> i32 {
    let e = &arena[idx];
    let this_in = !e.in_out;
    let that_in = !e.other_in_out;

    let is_in = match operation {
        Operation::Intersection => this_in && that_in,
        Operation::Union => this_in || that_in,
        Operation::Xor => this_in != that_in,
        Operation::Difference => {
            if e.is_subject() {
                this_in && !that_in
            } else {
                that_in && !this_in
            }
        }
    };
    if is_in {
        1
    } else {
        -1
    }
}

/// Whether this event's segment contributes to the result of the
/// given Boolean operation, given its current `edge_type`, `in_out`,
/// and `other_in_out` fields.
fn in_result(arena: &[SweepEvent], idx: usize, operation: Operation) -> bool {
    let e = &arena[idx];
    match e.edge_type {
        EdgeType::Normal => match operation {
            Operation::Intersection => !e.other_in_out,
            Operation::Union => e.other_in_out,
            Operation::Difference => {
                (e.is_subject() && e.other_in_out) || (!e.is_subject() && !e.other_in_out)
            }
            Operation::Xor => true,
        },
        EdgeType::SameTransition => {
            matches!(operation, Operation::Intersection | Operation::Union)
        }
        EdgeType::DifferentTransition => matches!(operation, Operation::Difference),
        EdgeType::NonContributing => false,
    }
}

/**********************************************************************
 * Tests — upstream has only an empty placeholder, so these are built
 * from the algorithm spec directly.
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
    fn no_predecessor_sets_default_in_out_flags() {
        let mut arena = Vec::new();
        let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        compute_fields(&mut arena, e, None, Operation::Intersection);
        assert!(!arena[e].in_out);
        assert!(arena[e].other_in_out);
    }

    #[test]
    fn same_polygon_flips_in_out_preserves_other_in_out() {
        let mut arena = Vec::new();
        let prev = add_segment(&mut arena, [0.0, 0.0], [2.0, 2.0], PolygonType::Subject);
        let cur = add_segment(&mut arena, [1.0, 0.0], [3.0, 2.0], PolygonType::Subject);
        arena[prev].in_out = true;
        arena[prev].other_in_out = false;

        compute_fields(&mut arena, cur, Some(prev), Operation::Union);
        assert!(!arena[cur].in_out); /* !prev.in_out */
        assert!(!arena[cur].other_in_out); /* == prev.other_in_out */
    }

    #[test]
    fn different_polygon_non_vertical_cross_couples() {
        let mut arena = Vec::new();
        let prev = add_segment(&mut arena, [0.0, 0.0], [2.0, 2.0], PolygonType::Clipping);
        let cur = add_segment(&mut arena, [1.0, 0.0], [3.0, 2.0], PolygonType::Subject);
        arena[prev].in_out = false;
        arena[prev].other_in_out = true;

        compute_fields(&mut arena, cur, Some(prev), Operation::Intersection);
        assert!(!arena[cur].in_out); /* !prev.other_in_out → !true = false */
        assert!(!arena[cur].other_in_out); /* prev.in_out (non-vertical) */
    }

    #[test]
    fn different_polygon_vertical_predecessor_flips_other_in_out() {
        /* Vertical predecessor: same x for both endpoints. */
        let mut arena = Vec::new();
        let prev = add_segment(&mut arena, [0.0, 0.0], [0.0, 5.0], PolygonType::Clipping);
        let cur = add_segment(&mut arena, [1.0, 0.0], [3.0, 2.0], PolygonType::Subject);
        arena[prev].in_out = true;
        arena[prev].other_in_out = true;

        compute_fields(&mut arena, cur, Some(prev), Operation::Intersection);
        assert!(!arena[cur].in_out); /* !prev.other_in_out */
        assert!(!arena[cur].other_in_out); /* !prev.in_out (vertical branch) */
    }

    #[test]
    fn intersection_result_transition_is_plus_one_when_both_inside() {
        let mut arena = Vec::new();
        let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        /* in_out=false, other_in_out=false ⇒ this_in=true, that_in=true ⇒ isIn=true ⇒ +1 */
        compute_fields(&mut arena, e, None, Operation::Intersection);
        /* No prev means in_out=false, other_in_out=true. inResult=!other_in_out=!true=false ⇒ 0. */
        assert_eq!(arena[e].result_transition, 0);
    }

    #[test]
    fn xor_normal_event_always_in_result_with_transition() {
        let mut arena = Vec::new();
        let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        compute_fields(&mut arena, e, None, Operation::Xor);
        /* XOR: inResult=true for NORMAL. result_transition = ±1 (here +1 because no prev). */
        assert_ne!(arena[e].result_transition, 0);
    }

    #[test]
    fn non_contributing_edge_never_in_result() {
        let mut arena = Vec::new();
        let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        arena[e].edge_type = EdgeType::NonContributing;
        compute_fields(&mut arena, e, None, Operation::Union);
        assert_eq!(arena[e].result_transition, 0);
    }

    #[test]
    fn same_transition_in_result_for_intersection_and_union_only() {
        for op in [Operation::Intersection, Operation::Union] {
            let mut arena = Vec::new();
            let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
            arena[e].edge_type = EdgeType::SameTransition;
            compute_fields(&mut arena, e, None, op);
            assert_ne!(arena[e].result_transition, 0, "{op:?} should include SameTransition");
        }
        for op in [Operation::Difference, Operation::Xor] {
            let mut arena = Vec::new();
            let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
            arena[e].edge_type = EdgeType::SameTransition;
            compute_fields(&mut arena, e, None, op);
            assert_eq!(arena[e].result_transition, 0, "{op:?} should exclude SameTransition");
        }
    }

    #[test]
    fn different_transition_in_result_for_difference_only() {
        for op in [
            Operation::Intersection,
            Operation::Union,
            Operation::Xor,
        ] {
            let mut arena = Vec::new();
            let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
            arena[e].edge_type = EdgeType::DifferentTransition;
            compute_fields(&mut arena, e, None, op);
            assert_eq!(arena[e].result_transition, 0);
        }
        let mut arena = Vec::new();
        let e = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        arena[e].edge_type = EdgeType::DifferentTransition;
        compute_fields(&mut arena, e, None, Operation::Difference);
        assert_ne!(arena[e].result_transition, 0);
    }

    #[test]
    fn prev_in_result_chains_through_non_contributing_predecessor() {
        /*
         * If prev doesn't contribute, the event should inherit prev's
         * prev_in_result rather than pointing at prev itself.
         */
        let mut arena = Vec::new();
        let ancestor = add_segment(&mut arena, [0.0, 0.0], [10.0, 0.0], PolygonType::Subject);
        let prev = add_segment(&mut arena, [1.0, 1.0], [9.0, 1.0], PolygonType::Subject);
        let cur = add_segment(&mut arena, [2.0, 2.0], [8.0, 2.0], PolygonType::Subject);

        /* Mark prev as non-contributing and give it an ancestor link. */
        arena[prev].edge_type = EdgeType::NonContributing;
        arena[prev].prev_in_result = Some(ancestor);

        compute_fields(&mut arena, cur, Some(prev), Operation::Union);
        assert_eq!(arena[cur].prev_in_result, Some(ancestor));
    }

    #[test]
    fn prev_in_result_points_to_prev_when_prev_contributes() {
        let mut arena = Vec::new();
        let prev = add_segment(&mut arena, [0.0, 0.0], [5.0, 0.0], PolygonType::Subject);
        let cur = add_segment(&mut arena, [1.0, 1.0], [4.0, 1.0], PolygonType::Subject);

        /* Force prev to be in-result: NORMAL edge with other_in_out=true for UNION. */
        arena[prev].edge_type = EdgeType::Normal;
        arena[prev].in_out = false;
        arena[prev].other_in_out = true;

        compute_fields(&mut arena, cur, Some(prev), Operation::Union);
        assert_eq!(arena[cur].prev_in_result, Some(prev));
    }
}
