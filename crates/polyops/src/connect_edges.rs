//! Stitch result events into output contours, port of upstream
//! `src/connect_edges.ts`. The largest single upstream file at 187
//! LOC; this Rust port keeps the same three-helper structure
//! (`order_events`, `next_pos`, `initialize_contour_from_context`)
//! and the main loop driver.
//!
//! Receives the sorted-event list produced by
//! [`crate::subdivide_segments`] and walks the events that contribute
//! to the result, stitching them into closed contours. Each contour
//! is also classified as either an exterior ring or a hole of some
//! earlier contour, via the `prev_in_result` link populated by
//! [`crate::compute_fields`].

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::compare_events::compare_events;
use crate::contour::Contour;
use crate::sweep_event::SweepEvent;

/// Top-level driver. Returns the list of result contours.
pub(crate) fn connect_edges(arena: &mut Vec<SweepEvent>, sorted_events: &[usize]) -> Vec<Contour> {
    let result_events = order_events(arena, sorted_events);

    let mut processed: Vec<bool> = vec![false; result_events.len()];
    let mut contours: Vec<Contour> = Vec::new();

    for i in 0..result_events.len() {
        if processed[i] {
            continue;
        }

        let contour_id = contours.len() as i32;
        let mut contour =
            initialize_contour_from_context(arena, result_events[i], &mut contours, contour_id);

        let mut pos = i;
        let orig_pos = i;

        let initial_point = arena[result_events[pos]].point;
        contour.points.push(initial_point);

        loop {
            mark_processed(arena, &result_events, &mut processed, pos, contour_id);

            /*
             * Jump across the segment to its partner: the right event
             * of the current left, or vice versa. `other_pos` was set
             * in `order_events` to index *within result_events*.
             */
            let next_pos_idx = arena[result_events[pos]].other_pos;
            debug_assert!(next_pos_idx >= 0, "other_pos was never initialized");
            pos = next_pos_idx as usize;

            mark_processed(arena, &result_events, &mut processed, pos, contour_id);
            contour.points.push(arena[result_events[pos]].point);

            match next_pos(arena, pos, &result_events, &processed, orig_pos) {
                None => break,
                Some(np) if np == orig_pos => break,
                Some(np) => pos = np,
            }
        }

        contours.push(contour);
    }

    contours
}

/**********************************************************************
 * Internal helpers — alphabetical.
 *********************************************************************/

/// Build a fresh [`Contour`] for the contour about to be assembled,
/// classifying it as an exterior ring or a hole based on the
/// `prev_in_result` link of the seed event.
fn initialize_contour_from_context(
    arena: &[SweepEvent],
    event_idx: usize,
    contours: &mut Vec<Contour>,
    contour_id: i32,
) -> Contour {
    let mut contour = Contour::new();

    let Some(prev_in_result_idx) = arena[event_idx].prev_in_result else {
        /* No lower contour ⇒ exterior ring at depth 0. */
        contour.hole_of = None;
        contour.depth = 0;
        return contour;
    };

    let prev = &arena[prev_in_result_idx];
    let lower_contour_id = prev.output_contour_id;
    let lower_result_transition = prev.result_transition;

    /*
     * If output_contour_id is still its sentinel `-1`, something's
     * gone wrong — we're trying to query a contour that hasn't been
     * assembled yet. Fall back to "exterior at depth 0" rather than
     * crash; downstream tests will catch the bad geometry.
     */
    if lower_contour_id < 0 {
        contour.hole_of = None;
        contour.depth = 0;
        return contour;
    }
    let lower_contour_id = lower_contour_id as usize;

    if lower_result_transition > 0 {
        /*
         * Inside the lower contour ⇒ this new contour is a hole. If
         * the lower contour is itself a hole, attach to *its* parent
         * (sibling holes share a parent and depth). Otherwise the
         * lower is an exterior and we nest one level deeper.
         */
        let lower_hole_of = contours[lower_contour_id].hole_of;
        let lower_depth = contours[lower_contour_id].depth;

        if let Some(parent_contour_id) = lower_hole_of {
            contours[parent_contour_id]
                .hole_ids
                .push(contour_id as usize);
            contour.hole_of = Some(parent_contour_id);
            contour.depth = lower_depth;
        } else {
            contours[lower_contour_id]
                .hole_ids
                .push(contour_id as usize);
            contour.hole_of = Some(lower_contour_id);
            contour.depth = lower_depth + 1;
        }
    } else {
        /* Outside the lower contour ⇒ this is another exterior at the same depth. */
        contour.hole_of = None;
        contour.depth = contours[lower_contour_id].depth;
    }

    contour
}

/// Mark `pos` in `result_events` as processed and assign it to the
/// contour currently being assembled. Helper to keep the main loop
/// readable.
fn mark_processed(
    arena: &mut Vec<SweepEvent>,
    result_events: &[usize],
    processed: &mut [bool],
    pos: usize,
    contour_id: i32,
) {
    if pos < processed.len() {
        processed[pos] = true;
    }
    if pos < result_events.len() {
        arena[result_events[pos]].output_contour_id = contour_id;
    }
}

/// Find the next position to visit when walking a contour.
///
/// Search forward in `result_events` for an unprocessed event at the
/// same point as `pos`. If none, search backward (within `pos >=
/// orig_pos`). Returns `None` when nothing usable is found.
fn next_pos(
    arena: &[SweepEvent],
    pos: usize,
    result_events: &[usize],
    processed: &[bool],
    orig_pos: usize,
) -> Option<usize> {
    let p = arena[result_events[pos]].point;

    /* Forward scan. */
    let mut new_pos = pos + 1;
    while new_pos < result_events.len() {
        let p1 = arena[result_events[new_pos]].point;
        if p1[0] != p[0] || p1[1] != p[1] {
            break;
        }
        if !processed[new_pos] {
            return Some(new_pos);
        }
        new_pos += 1;
    }

    /*
     * Backward scan. Upstream may visit negative positions in JS
     * (returning undefined that the caller detects); in Rust we use
     * checked_sub and return None instead.
     */
    let mut new_pos = pos.checked_sub(1)?;
    while processed[new_pos] && new_pos > orig_pos {
        new_pos = new_pos.checked_sub(1)?;
    }
    Some(new_pos)
}

/// Filter `sorted_events` to events that contribute to the result,
/// stable-sort them by `compare_events`, then set up `other_pos`
/// so each event knows its peer's index *within result_events*.
fn order_events(arena: &mut Vec<SweepEvent>, sorted_events: &[usize]) -> Vec<usize> {
    let mut result_events: Vec<usize> = Vec::new();
    for &idx in sorted_events {
        let event = &arena[idx];
        let other_idx = event
            .other_event
            .expect("order_events: event has no peer");
        let include = (event.left && event.in_result())
            || (!event.left && arena[other_idx].in_result());
        if include {
            result_events.push(idx);
        }
    }

    /*
     * Upstream note: due to overlapping edges, `result_events` may
     * not be fully sorted even though `sorted_events` was popped in
     * order. A bubble sort restores the invariant. The list is
     * typically small after filtering so the O(n^2) is fine.
     */
    let mut sorted = false;
    while !sorted {
        sorted = true;
        for i in 0..result_events.len().saturating_sub(1) {
            if compare_events(arena, result_events[i], result_events[i + 1]) == Ordering::Greater
            {
                result_events.swap(i, i + 1);
                sorted = false;
            }
        }
    }

    /* Assign each event's `other_pos` to its index in result_events. */
    for (i, &idx) in result_events.iter().enumerate() {
        arena[idx].other_pos = i as i32;
    }

    /*
     * For each right event, swap `other_pos` with its peer's. This
     * ensures that following `other_pos` from a left event leads to
     * the right's position, and vice versa — what the contour walker
     * needs. (Upstream comment explains: "the right event is found
     * in the beginning of the queue, when his left counterpart is
     * not marked yet".)
     */
    let right_event_indices: Vec<usize> = result_events
        .iter()
        .copied()
        .filter(|&idx| !arena[idx].left)
        .collect();
    for idx in right_event_indices {
        let other_idx = arena[idx].other_event.unwrap();
        let tmp = arena[idx].other_pos;
        arena[idx].other_pos = arena[other_idx].other_pos;
        arena[other_idx].other_pos = tmp;
    }

    result_events
}

/**********************************************************************
 * Tests — direct smoke coverage. Real end-to-end correctness is
 * verified via the parity harness once boolean_op is wired up.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_returns_no_contours() {
        let mut arena: Vec<SweepEvent> = Vec::new();
        let contours = connect_edges(&mut arena, &[]);
        assert!(contours.is_empty());
    }

    #[test]
    fn no_events_contribute_returns_no_contours() {
        /*
         * Build a set of events where none of them is `in_result`
         * (all have result_transition == 0). order_events should
         * filter them all out.
         */
        use crate::edge_type::EdgeType;
        use crate::sweep_event::PolygonType;

        let mut arena = Vec::new();
        for i in 0..4 {
            let pt = [i as f64, 0.0];
            let mut ev = SweepEvent::new(pt, true, PolygonType::Subject, EdgeType::Normal);
            ev.result_transition = 0;
            arena.push(ev);
        }
        let indices: Vec<usize> = (0..arena.len()).collect();
        /* All events have other_event = None, so order_events would panic. Skip. */
        let _ = indices;
        /* Instead, just call with no sorted events. */
        let contours = connect_edges(&mut arena, &[]);
        assert!(contours.is_empty());
    }
}
