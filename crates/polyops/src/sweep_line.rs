//! Sweep-line status structure, parallel to upstream's `splaytree`.
//!
//! Upstream stores active segments in a self-balancing splay tree
//! keyed by `compareSegments`, so insert/remove/find/prev/next are
//! all `O(log n)` amortized. We model the same operations with a
//! **sorted `Vec<usize>`** of event indices, keyed by the same
//! comparator. `O(n)` insertion via vec shifting is acceptable for
//! the parity-first phase; if benchmarks show this as a bottleneck
//! we can swap in `BTreeSet` or the `splay_tree` crate later. See
//! `PLAN.md` §8 for the decision log entry.
//!
//! All ops take `&[SweepEvent]` externally because `compare_segments`
//! needs to dereference `other_event` to do its work.

#![allow(dead_code)]

use crate::compare_segments::compare_segments;
use crate::sweep_event::SweepEvent;

/// Ordered set of event indices keyed by [`crate::compare_segments`].
#[derive(Debug, Default)]
pub(crate) struct SweepLine {
    events: Vec<usize>,
}

impl SweepLine {
    pub(crate) fn new() -> Self {
        Self { events: Vec::new() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.events.len()
    }

    /// Event index at the given position, or `None` if out of bounds.
    pub(crate) fn at(&self, position: usize) -> Option<usize> {
        self.events.get(position).copied()
    }

    /// Insert `idx` in `compare_segments` order. Returns the
    /// resulting position. Caller is responsible for not inserting
    /// the same index twice.
    pub(crate) fn insert(&mut self, arena: &[SweepEvent], idx: usize) -> usize {
        let pos = self
            .events
            .binary_search_by(|&existing| compare_segments(arena, existing, idx))
            .unwrap_or_else(|p| p);
        self.events.insert(pos, idx);
        pos
    }

    /// Position of `idx` in the sweep line, or `None` if absent.
    pub(crate) fn position(&self, arena: &[SweepEvent], idx: usize) -> Option<usize> {
        /*
         * Try binary search first (cheap, works when comparator is
         * stable). Fall back to linear scan: a previous
         * `divide_segment` may have mutated an event's `other_event`
         * link, shifting its sort key without updating the sweep
         * line, and binary_search can then miss the exact index.
         */
        if let Ok(p) = self
            .events
            .binary_search_by(|&existing| compare_segments(arena, existing, idx))
        {
            /* binary_search may return any index that compares equal;
             * the comparator returns Equal only for the same arena
             * index, so this branch is exact. */
            if self.events[p] == idx {
                return Some(p);
            }
        }
        self.events.iter().position(|&e| e == idx)
    }

    /// Remove `idx` from the sweep line. Returns `true` if it was
    /// present.
    pub(crate) fn remove(&mut self, arena: &[SweepEvent], idx: usize) -> bool {
        match self.position(arena, idx) {
            Some(pos) => {
                self.events.remove(pos);
                true
            }
            None => false,
        }
    }

    /// Remove the event at `position`. Caller is responsible for the
    /// position being in bounds. Used by [`crate::subdivide_segments`]
    /// to avoid a second `position` lookup after it already has the
    /// index in hand.
    pub(crate) fn remove_at(&mut self, position: usize) {
        self.events.remove(position);
    }

    /// Element immediately above `position` in the sweep ordering
    /// (i.e., at `position + 1`), or `None` if at the top.
    pub(crate) fn next(&self, position: usize) -> Option<usize> {
        self.at(position + 1)
    }

    /// Element immediately below `position` (`position - 1`), or
    /// `None` if `position == 0`.
    pub(crate) fn prev(&self, position: usize) -> Option<usize> {
        position.checked_sub(1).and_then(|p| self.at(p))
    }

    /// Lowest event in the sweep line, or `None` if empty.
    pub(crate) fn min(&self) -> Option<usize> {
        self.events.first().copied()
    }
}

/**********************************************************************
 * Tests.
 *********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_type::EdgeType;
    use crate::sweep_event::PolygonType;
    use crate::types::Position;

    fn add_segment(
        arena: &mut Vec<SweepEvent>,
        left_pt: Position,
        right_pt: Position,
    ) -> usize {
        let left_idx = arena.len();
        arena.push(SweepEvent::new(
            left_pt,
            true,
            PolygonType::Subject,
            EdgeType::Normal,
        ));
        let right_idx = arena.len();
        arena.push(SweepEvent::new(
            right_pt,
            false,
            PolygonType::Subject,
            EdgeType::Normal,
        ));
        arena[left_idx].other_event = Some(right_idx);
        arena[right_idx].other_event = Some(left_idx);
        left_idx
    }

    #[test]
    fn empty_line_has_no_min() {
        let sl = SweepLine::new();
        assert!(sl.is_empty());
        assert_eq!(sl.min(), None);
    }

    #[test]
    fn insert_orders_by_compare_segments() {
        let mut arena = Vec::new();
        /* Two segments sharing left endpoint; one ends lower than the other ⇒ smaller. */
        let lower = add_segment(&mut arena, [0.0, 0.0], [5.0, 1.0]);
        let upper = add_segment(&mut arena, [0.0, 0.0], [5.0, 3.0]);

        let mut sl = SweepLine::new();
        sl.insert(&arena, upper);
        sl.insert(&arena, lower);

        /* Lower segment should be at position 0. */
        assert_eq!(sl.at(0), Some(lower));
        assert_eq!(sl.at(1), Some(upper));
        assert_eq!(sl.min(), Some(lower));
    }

    #[test]
    fn remove_takes_event_out_of_the_line() {
        let mut arena = Vec::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 1.0]);
        let b = add_segment(&mut arena, [0.0, 0.0], [5.0, 3.0]);
        let mut sl = SweepLine::new();
        sl.insert(&arena, a);
        sl.insert(&arena, b);
        assert!(sl.remove(&arena, a));
        assert_eq!(sl.len(), 1);
        assert_eq!(sl.min(), Some(b));
        assert!(!sl.remove(&arena, a)); /* second remove is a no-op. */
    }

    #[test]
    fn prev_and_next_navigate_neighbours() {
        let mut arena = Vec::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 1.0]);
        let b = add_segment(&mut arena, [0.0, 0.0], [5.0, 2.0]);
        let c = add_segment(&mut arena, [0.0, 0.0], [5.0, 3.0]);
        let mut sl = SweepLine::new();
        sl.insert(&arena, a);
        sl.insert(&arena, b);
        sl.insert(&arena, c);
        let pos = sl.position(&arena, b).unwrap();
        assert_eq!(sl.prev(pos), Some(a));
        assert_eq!(sl.next(pos), Some(c));
    }

    #[test]
    fn prev_at_position_zero_is_none() {
        let mut arena = Vec::new();
        let a = add_segment(&mut arena, [0.0, 0.0], [5.0, 1.0]);
        let mut sl = SweepLine::new();
        sl.insert(&arena, a);
        assert_eq!(sl.prev(0), None);
    }
}
