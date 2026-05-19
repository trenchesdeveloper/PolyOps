//! Priority queue of sweep events ordered by [`crate::compare_events`].
//!
//! Upstream uses `tinyqueue`, a JS binary-heap library that takes a
//! comparator function. We model the same shape: a min-heap keyed by
//! the algorithm's event-comparison order, with the catch that
//! comparing two events requires access to the arena (to dereference
//! `other_event`). So `push` and `pop` take `&[SweepEvent]` as an
//! external parameter — the queue itself only stores `usize` indices.
//!
//! This keeps event memory in one cache-friendly `Vec<SweepEvent>`
//! owned by the sweep driver, and avoids `Rc<RefCell<_>>` or
//! lifetime gymnastics on the queue.

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::compare_events::compare_events;
use crate::sweep_event::SweepEvent;

/// Min-heap of event indices, ordered by `compare_events`.
#[derive(Debug, Default)]
pub(crate) struct EventQueue {
    heap: Vec<usize>,
}

impl EventQueue {
    pub(crate) fn new() -> Self {
        Self { heap: Vec::new() }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub(crate) fn len(&self) -> usize {
        self.heap.len()
    }

    /// Insert `idx` into the heap, restoring the heap property by
    /// sifting up.
    pub(crate) fn push(&mut self, arena: &[SweepEvent], idx: usize) {
        self.heap.push(idx);
        self.sift_up(arena, self.heap.len() - 1);
    }

    /// Remove and return the minimum-priority event index, or `None`
    /// if the queue is empty.
    pub(crate) fn pop(&mut self, arena: &[SweepEvent]) -> Option<usize> {
        let n = self.heap.len();
        if n == 0 {
            return None;
        }
        if n == 1 {
            return self.heap.pop();
        }
        /* Swap root with last, pop the (former) root, sift the new root down. */
        let result = self.heap.swap_remove(0);
        self.sift_down(arena, 0);
        Some(result)
    }

    /*
     * Internal heap operations — manual implementation rather than
     * `BinaryHeap` because Rust's `BinaryHeap` requires elements to
     * implement `Ord` standalone, but our ordering needs the arena.
     */

    fn sift_down(&mut self, arena: &[SweepEvent], mut i: usize) {
        let n = self.heap.len();
        loop {
            let l = 2 * i + 1;
            let r = 2 * i + 2;
            let mut smallest = i;
            if l < n
                && compare_events(arena, self.heap[l], self.heap[smallest]) == Ordering::Less
            {
                smallest = l;
            }
            if r < n
                && compare_events(arena, self.heap[r], self.heap[smallest]) == Ordering::Less
            {
                smallest = r;
            }
            if smallest == i {
                return;
            }
            self.heap.swap(i, smallest);
            i = smallest;
        }
    }

    fn sift_up(&mut self, arena: &[SweepEvent], mut i: usize) {
        while i > 0 {
            let parent = (i - 1) / 2;
            if compare_events(arena, self.heap[i], self.heap[parent]) == Ordering::Less {
                self.heap.swap(i, parent);
                i = parent;
            } else {
                break;
            }
        }
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

    fn add_bare_event(arena: &mut Vec<SweepEvent>, point: [f64; 2], left: bool) -> usize {
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
    fn empty_queue_pops_none() {
        let mut q = EventQueue::new();
        let arena: Vec<SweepEvent> = Vec::new();
        assert!(q.is_empty());
        assert_eq!(q.pop(&arena), None);
    }

    #[test]
    fn pops_in_compare_events_order_by_x() {
        let mut arena = Vec::new();
        let e0 = add_bare_event(&mut arena, [5.0, 5.0], true);
        let e1 = add_bare_event(&mut arena, [0.0, 0.0], true);
        let e2 = add_bare_event(&mut arena, [3.0, 3.0], true);

        let mut q = EventQueue::new();
        q.push(&arena, e0);
        q.push(&arena, e1);
        q.push(&arena, e2);

        /* Should pop in ascending x order: e1, e2, e0. */
        assert_eq!(q.pop(&arena), Some(e1));
        assert_eq!(q.pop(&arena), Some(e2));
        assert_eq!(q.pop(&arena), Some(e0));
        assert_eq!(q.pop(&arena), None);
    }

    #[test]
    fn pops_in_compare_events_order_by_y_when_x_ties() {
        let mut arena = Vec::new();
        let e0 = add_bare_event(&mut arena, [1.0, 5.0], true);
        let e1 = add_bare_event(&mut arena, [1.0, 1.0], true);
        let e2 = add_bare_event(&mut arena, [1.0, 3.0], true);

        let mut q = EventQueue::new();
        q.push(&arena, e0);
        q.push(&arena, e1);
        q.push(&arena, e2);

        assert_eq!(q.pop(&arena), Some(e1));
        assert_eq!(q.pop(&arena), Some(e2));
        assert_eq!(q.pop(&arena), Some(e0));
    }

    #[test]
    fn right_endpoint_sorts_before_left_at_same_point() {
        let mut arena = Vec::new();
        let left = add_bare_event(&mut arena, [2.0, 2.0], true);
        let right = add_bare_event(&mut arena, [2.0, 2.0], false);

        let mut q = EventQueue::new();
        q.push(&arena, left);
        q.push(&arena, right);
        /* Per compare_events: same point + different left ⇒ right first. */
        assert_eq!(q.pop(&arena), Some(right));
        assert_eq!(q.pop(&arena), Some(left));
    }

    #[test]
    fn len_tracks_pushes_and_pops() {
        let mut arena = Vec::new();
        let a = add_bare_event(&mut arena, [0.0, 0.0], true);
        let b = add_bare_event(&mut arena, [1.0, 1.0], true);
        let mut q = EventQueue::new();
        assert_eq!(q.len(), 0);
        q.push(&arena, a);
        assert_eq!(q.len(), 1);
        q.push(&arena, b);
        assert_eq!(q.len(), 2);
        q.pop(&arena);
        assert_eq!(q.len(), 1);
        q.pop(&arena);
        assert!(q.is_empty());
    }
}
