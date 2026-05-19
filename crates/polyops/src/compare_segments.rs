//! Sweep-line segment ordering, port of upstream `src/compare_segments.ts`.
//!
//! Used to keep the sweep-line status tree (a `BTreeSet` in our
//! implementation, a splay tree upstream) sorted by vertical position
//! along the current sweep x.
//!
//! Unlike [`crate::compare_events::compare_events`], this comparator
//! *can* return [`Ordering::Equal`] — but only when the two arena
//! indices point at the same event, or when two events from the same
//! polygon represent the same segment endpoint-for-endpoint. Most
//! sweep-line code never inserts the same segment twice, so in
//! practice `Equal` is rare.

#![allow(dead_code)]

use std::cmp::Ordering;

use crate::compare_events::compare_events;
use crate::equals::equals;
use crate::signed_area::signed_area;
use crate::sweep_event::SweepEvent;

/// Compare segments `i1` and `i2` along the sweep line.
pub(crate) fn compare_segments(arena: &[SweepEvent], i1: usize, i2: usize) -> Ordering {
    if i1 == i2 {
        return Ordering::Equal;
    }

    let e1 = &arena[i1];
    let e2 = &arena[i2];
    let o1 = arena[e1.other_event.expect("compare_segments: e1 has no peer")].point;
    let o2 = arena[e2.other_event.expect("compare_segments: e2 has no peer")].point;

    /*
     * Segments are *not* collinear if either:
     *   - e2.point is off the line through (e1.point, o1), or
     *   - o2 is off the same line.
     */
    let not_collinear =
        signed_area(e1.point, o1, e2.point) != 0 || signed_area(e1.point, o1, o2) != 0;

    if not_collinear {
        /* Shared left endpoint: order by where each one's right point sits. */
        if equals(e1.point, e2.point) {
            return if e1.is_below(o1, o2) {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        /* Same x, different y at the left endpoint: lower y is below. */
        if e1.point[0] == e2.point[0] {
            return if e1.point[1] < e2.point[1] {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }

        /*
         * Different left endpoints. Upstream's two branches both
         * resolve to the same geometric question — is e1 above or
         * below the sweep position of e2 (or vice versa). The
         * `compareEvents` call decides which event's "now" we use as
         * the reference x.
         */
        if compare_events(arena, i1, i2) == Ordering::Greater {
            return if e2.is_above(o2, e1.point) {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        return if e1.is_below(o1, e2.point) {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    /*
     * Collinear branch.
     */
    if e1.is_subject() == e2.is_subject() {
        /* Same polygon type. */
        if e1.point == e2.point {
            /* Same left point: tiebreak by right point, then contour id. */
            if o1 == o2 {
                return Ordering::Equal;
            }
            let c1 = e1.contour_id.unwrap_or(0);
            let c2 = e2.contour_id.unwrap_or(0);
            return if c1 > c2 {
                Ordering::Greater
            } else {
                Ordering::Less
            };
        }
        /* Different left points: fall through to compareEvents tiebreak. */
    } else {
        /*
         * Collinear segments from different polygons: subject sorts
         * below clipping.
         */
        return if e1.is_subject() {
            Ordering::Less
        } else {
            Ordering::Greater
        };
    }

    /*
     * Reached only for: collinear, same polygon type, different left
     * points. Tiebreak by event ordering.
     */
    if compare_events(arena, i1, i2) == Ordering::Greater {
        Ordering::Greater
    } else {
        Ordering::Less
    }
}

/*
 * Tests — mirror upstream `test/compare_segments.test.ts` 1:1.
 */
#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge_type::EdgeType;
    use crate::sweep_event::PolygonType;

    fn add_segment(
        arena: &mut Vec<SweepEvent>,
        left_pt: [f64; 2],
        right_pt: [f64; 2],
        polygon_type: PolygonType,
    ) -> usize {
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
        left_idx
    }

    #[test]
    fn not_collinear_shared_left_point_orders_by_right_point() {
        /*
         * se1: (0,0) → (1,1); se2: (0,0) → (2,3). Shared left point;
         * se1 ends below se2 ⇒ se1 < se2.
         */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        let e2 = add_segment(&mut arena, [0.0, 0.0], [2.0, 3.0], PolygonType::Subject);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_segments(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn not_collinear_same_x_different_y_orders_by_left_y() {
        /*
         * se1: (0,1) → (1,1); se2: (0,2) → (2,3). Same x at the left
         * endpoint, se1's y is lower ⇒ se1 < se2.
         */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [0.0, 1.0], [1.0, 1.0], PolygonType::Subject);
        let e2 = add_segment(&mut arena, [0.0, 2.0], [2.0, 3.0], PolygonType::Subject);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_segments(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn maintains_events_order_in_sweep_line() {
        /*
         * Upstream's "should maintain events order in sweep line"
         * test. Two pairs of segments with different left endpoints,
         * exercising the `compareEvents == 1` branch.
         */
        let mut arena = Vec::new();
        let se1 = add_segment(&mut arena, [0.0, 1.0], [2.0, 1.0], PolygonType::Subject);
        let se2 = add_segment(&mut arena, [-1.0, 0.0], [2.0, 3.0], PolygonType::Subject);

        /* compareEvents(se1, se2) compares left points (0,1) vs (-1,0) ⇒ se1 > se2. */
        assert_eq!(compare_events(&arena, se1, se2), Ordering::Greater);

        /* se2 should be ABOVE se1.point=(0,1)? Upstream says no. */
        let o2 = arena[arena[se2].other_event.unwrap()].point;
        assert!(!arena[se2].is_below(o2, arena[se1].point));
        assert!(arena[se2].is_above(o2, arena[se1].point));

        assert_eq!(compare_segments(&arena, se1, se2), Ordering::Less);
        assert_eq!(compare_segments(&arena, se2, se1), Ordering::Greater);
    }

    #[test]
    fn handles_when_first_point_is_below() {
        /*
         * Upstream "should handle when first point is below". se1
         * passes through se2.point on its way — by upstream's
         * is_below convention on collinear-at-query, returns false,
         * so we land in the `... ? -1 : 1` else branch ⇒ Greater.
         */
        let mut arena = Vec::new();
        let se2 = add_segment(&mut arena, [0.0, 1.0], [2.0, 1.0], PolygonType::Subject);
        let se1 = add_segment(&mut arena, [-1.0, 0.0], [2.0, 3.0], PolygonType::Subject);

        let o1 = arena[arena[se1].other_event.unwrap()].point;
        assert!(!arena[se1].is_below(o1, arena[se2].point));
        assert_eq!(compare_segments(&arena, se1, se2), Ordering::Greater);
    }

    #[test]
    fn collinear_subject_below_clipping() {
        /*
         * se1 subject, se2 clipping, both on y=1 ⇒ collinear,
         * different polygon types ⇒ subject sorts first (Less).
         */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [1.0, 1.0], [5.0, 1.0], PolygonType::Subject);
        let e2 = add_segment(&mut arena, [2.0, 1.0], [3.0, 1.0], PolygonType::Clipping);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_segments(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn collinear_shared_left_point_tiebreaks_by_contour_id() {
        /*
         * Both clipping, both at (0,1), collinear on y=1, different
         * right points ⇒ tiebreak by `contour_id`.
         */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [0.0, 1.0], [5.0, 1.0], PolygonType::Clipping);
        let e2 = add_segment(&mut arena, [0.0, 1.0], [3.0, 1.0], PolygonType::Clipping);
        arena[e1].contour_id = Some(1);
        arena[e2].contour_id = Some(2);

        assert_eq!(arena[e1].is_subject(), arena[e2].is_subject());
        assert_eq!(arena[e1].point, arena[e2].point);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Less);

        arena[e1].contour_id = Some(2);
        arena[e2].contour_id = Some(1);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Greater);
    }

    #[test]
    fn collinear_same_polygon_different_left_points() {
        /*
         * Both subject, collinear on y=1, different left points
         * ((1,1) vs (2,1)) ⇒ falls through to compareEvents tiebreak.
         * compareEvents puts the smaller-x event first; here se1's x=1
         * < se2's x=2 ⇒ Less.
         */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [1.0, 1.0], [5.0, 1.0], PolygonType::Subject);
        let e2 = add_segment(&mut arena, [2.0, 1.0], [3.0, 1.0], PolygonType::Subject);

        assert_eq!(arena[e1].is_subject(), arena[e2].is_subject());
        assert_ne!(arena[e1].point, arena[e2].point);
        assert_eq!(compare_segments(&arena, e1, e2), Ordering::Less);
        assert_eq!(compare_segments(&arena, e2, e1), Ordering::Greater);
    }

    #[test]
    fn same_index_compares_equal() {
        /* Pointer-equality short-circuit. */
        let mut arena = Vec::new();
        let e1 = add_segment(&mut arena, [0.0, 0.0], [1.0, 1.0], PolygonType::Subject);
        assert_eq!(compare_segments(&arena, e1, e1), Ordering::Equal);
    }
}
