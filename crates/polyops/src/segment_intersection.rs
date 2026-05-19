//! Segment-segment intersection, port of upstream
//! `src/segment_intersection.ts`.
//!
//! One of the algorithm's hot kernels — every pair of sweep-line
//! neighbors passes through this function. Algorithm is from
//! Schneider & Eberly, *Geometric Tools for Computer Graphics*, p.244.
//!
//! Returns one of three outcomes:
//!
//! - [`SegmentIntersection::None`] — no intersection.
//! - [`SegmentIntersection::Point`] — segments cross or touch at a
//!   single point.
//! - [`SegmentIntersection::Overlap`] — collinear segments with a
//!   non-degenerate shared interval.
//!
//! The `no_endpoint_touch` flag suppresses single-point intersections
//! that are purely endpoint touches (one segment's endpoint coincides
//! with the other's). This is the signal `possible_intersection` uses
//! to skip already-connected segment pairs that don't actually need
//! to be subdivided.
//!
//! **Parity-sensitive.** The collinear-overlap branch is where most
//! Martinez parity bugs hide. We mirror upstream's branch structure
//! byte-for-byte rather than rewriting it idiomatically; once parity
//! is locked, refactoring is fair game.

#![allow(dead_code)]

use crate::types::Position;

/// What kind of intersection two segments produce.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SegmentIntersection {
    /// Segments don't touch.
    None,
    /// Segments meet at a single point.
    Point(Position),
    /// Segments are collinear and share an interval; the two endpoints
    /// of the overlap are returned. Order follows the parameterization
    /// of segment `a`: the first point corresponds to the smaller
    /// `s` value (or the clamped `[0, 1]` boundary).
    Overlap(Position, Position),
}

/// Find the intersection of segments `(a1, a2)` and `(b1, b2)`.
///
/// If `no_endpoint_touch` is `true`, intersections that exist only
/// because the two segments share an endpoint are reported as
/// [`SegmentIntersection::None`].
pub(crate) fn intersection(
    a1: Position,
    a2: Position,
    b1: Position,
    b2: Position,
    no_endpoint_touch: bool,
) -> SegmentIntersection {
    /*
     * The algorithm expects lines in the form P + sd, where P is a
     * point, s ∈ [0, 1], and d is a direction vector. Convert by
     * subtracting endpoints.
     */
    let va: Position = [a2[0] - a1[0], a2[1] - a1[1]];
    let vb: Position = [b2[0] - b1[0], b2[1] - b1[1]];
    let e: Position = [b1[0] - a1[0], b1[1] - a1[1]];

    let mut kross = cross_product(va, vb);
    let mut sqr_kross = kross * kross;
    let sqr_len_a = dot_product(va, va);

    /*
     * Lines are not parallel ⇒ unique line intersection. Test whether
     * it lies on both segments.
     */
    if sqr_kross > 0.0 {
        let s = cross_product(e, vb) / kross;
        if !(0.0..=1.0).contains(&s) {
            /* Intersection point off segment a. */
            return SegmentIntersection::None;
        }
        let t = cross_product(e, va) / kross;
        if !(0.0..=1.0).contains(&t) {
            /* Intersection point off segment b. */
            return SegmentIntersection::None;
        }
        if s == 0.0 || s == 1.0 {
            /* Intersection is at an endpoint of segment a. */
            return if no_endpoint_touch {
                SegmentIntersection::None
            } else {
                SegmentIntersection::Point(to_point(a1, s, va))
            };
        }
        if t == 0.0 || t == 1.0 {
            /* Intersection is at an endpoint of segment b. */
            return if no_endpoint_touch {
                SegmentIntersection::None
            } else {
                SegmentIntersection::Point(to_point(b1, t, vb))
            };
        }
        return SegmentIntersection::Point(to_point(a1, s, va));
    }

    /*
     * Reaching here means the lines are parallel or identical.
     * Determine which: if e is parallel to va, the lines are the
     * same. Otherwise they're truly parallel and don't meet.
     */
    kross = cross_product(e, va);
    sqr_kross = kross * kross;

    if sqr_kross > 0.0 {
        /* Parallel but distinct lines: no overlap possible. */
        return SegmentIntersection::None;
    }

    /*
     * Same line. Project b's endpoints onto a's parameterization,
     * derive the overlap interval [smin, smax] in a's parameter
     * space, and intersect with [0, 1].
     */
    let sa = dot_product(va, e) / sqr_len_a;
    let sb = sa + dot_product(va, vb) / sqr_len_a;
    let smin = sa.min(sb);
    let smax = sa.max(sb);

    if smin <= 1.0 && smax >= 0.0 {
        /* Overlap exists. Distinguish "single point" from "interval". */
        if smin == 1.0 {
            /* Overlap is just a, b sharing one endpoint at a2. */
            return if no_endpoint_touch {
                SegmentIntersection::None
            } else {
                SegmentIntersection::Point(to_point(a1, if smin > 0.0 { smin } else { 0.0 }, va))
            };
        }
        if smax == 0.0 {
            /* Overlap is just the shared endpoint at a1. */
            return if no_endpoint_touch {
                SegmentIntersection::None
            } else {
                SegmentIntersection::Point(to_point(a1, if smax < 1.0 { smax } else { 1.0 }, va))
            };
        }
        if no_endpoint_touch && smin == 0.0 && smax == 1.0 {
            /*
             * Full coincidence with both endpoints touching. Caller
             * has asked us to suppress endpoint-only matches.
             */
            return SegmentIntersection::None;
        }
        /* Real interval overlap. Return both clamped endpoints. */
        let p1 = to_point(a1, if smin > 0.0 { smin } else { 0.0 }, va);
        let p2 = to_point(a1, if smax < 1.0 { smax } else { 1.0 }, va);
        return SegmentIntersection::Overlap(p1, p2);
    }

    SegmentIntersection::None
}

/*
 * Internal helpers — alphabetical.
 */

fn cross_product(a: Position, b: Position) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}

fn dot_product(a: Position, b: Position) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

fn to_point(p: Position, s: f64, d: Position) -> Position {
    [p[0] + s * d[0], p[1] + s * d[1]]
}

/*
 * Tests — mirror upstream `test/segment_intersection.test.ts` 1:1.
 */
#[cfg(test)]
mod tests {
    use super::*;

    /*
     * Convenience: assert `intersection(...)` returned a single-point
     * intersection at exactly `expected`.
     */
    fn assert_point(result: SegmentIntersection, expected: Position) {
        match result {
            SegmentIntersection::Point(p) => assert_eq!(p, expected),
            other => panic!("expected Point({expected:?}), got {other:?}"),
        }
    }

    /*
     * Convenience: assert `intersection(...)` returned an interval
     * overlap with the given two endpoints, in order.
     */
    fn assert_overlap(result: SegmentIntersection, e1: Position, e2: Position) {
        match result {
            SegmentIntersection::Overlap(p1, p2) => {
                assert_eq!(p1, e1, "first overlap endpoint");
                assert_eq!(p2, e2, "second overlap endpoint");
            }
            other => panic!("expected Overlap({e1:?}, {e2:?}), got {other:?}"),
        }
    }

    #[test]
    fn no_intersection_when_segments_dont_meet() {
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [2.0, 2.0], false),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [10.0, 2.0], false),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([2.0, 2.0], [3.0, 3.0], [0.0, 6.0], [2.0, 4.0], false),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn single_intersection_in_interior() {
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0], false),
            [0.5, 0.5],
        );
    }

    #[test]
    fn shared_endpoint_points() {
        /* Endpoint of segment b lies on endpoint of segment a. */
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0], false),
            [0.0, 0.0],
        );
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 1.0], false),
            [1.0, 1.0],
        );
    }

    #[test]
    fn t_crossing() {
        /* Endpoint of segment b lies in the interior of segment a. */
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [0.5, 0.5], [1.0, 0.0], false),
            [0.5, 0.5],
        );
    }

    #[test]
    fn overlapping_segments() {
        /*
         * Five upstream cases for collinear overlapping segments —
         * each verifies the orientation rule that the returned pair
         * matches segment a's parameterization order.
         */
        assert_overlap(
            intersection([0.0, 0.0], [10.0, 10.0], [1.0, 1.0], [5.0, 5.0], false),
            [1.0, 1.0],
            [5.0, 5.0],
        );
        assert_overlap(
            intersection([1.0, 1.0], [10.0, 10.0], [1.0, 1.0], [5.0, 5.0], false),
            [1.0, 1.0],
            [5.0, 5.0],
        );
        assert_overlap(
            intersection([3.0, 3.0], [10.0, 10.0], [0.0, 0.0], [5.0, 5.0], false),
            [3.0, 3.0],
            [5.0, 5.0],
        );
        assert_overlap(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], false),
            [0.0, 0.0],
            [1.0, 1.0],
        );
        /*
         * Reversed segment a: the result preserves a's direction, so
         * the first point is a's "start" (1,1).
         */
        assert_overlap(
            intersection([1.0, 1.0], [0.0, 0.0], [0.0, 0.0], [1.0, 1.0], false),
            [1.0, 1.0],
            [0.0, 0.0],
        );
    }

    #[test]
    fn collinear_segments_touching_at_endpoint() {
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 1.0], [2.0, 2.0], false),
            [1.0, 1.0],
        );
        assert_point(
            intersection([1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0], false),
            [1.0, 1.0],
        );
    }

    #[test]
    fn collinear_segments_disjoint() {
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [2.0, 2.0], [4.0, 4.0], false),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn parallel_but_not_collinear_segments() {
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, -1.0], [1.0, 0.0], false),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([1.0, 1.0], [0.0, 0.0], [0.0, -1.0], [1.0, 0.0], false),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([0.0, -1.0], [1.0, 0.0], [0.0, 0.0], [1.0, 1.0], false),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn skip_touches_shared_endpoints() {
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [0.0, 0.0], true),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 1.0], true),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn skip_touches_collinear_segments() {
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 1.0], [2.0, 2.0], true),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([1.0, 1.0], [0.0, 0.0], [1.0, 1.0], [2.0, 2.0], true),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn skip_touches_fully_overlapping_segments() {
        /*
         * When `no_endpoint_touch` is on AND smin==0 AND smax==1, the
         * "overlap" is actually just two shared endpoints with no
         * real interior overlap to report.
         */
        assert_eq!(
            intersection([0.0, 0.0], [1.0, 1.0], [0.0, 0.0], [1.0, 1.0], true),
            SegmentIntersection::None,
        );
        assert_eq!(
            intersection([1.0, 1.0], [0.0, 0.0], [0.0, 0.0], [1.0, 1.0], true),
            SegmentIntersection::None,
        );
    }

    #[test]
    fn skip_touches_does_not_suppress_real_intersections() {
        /*
         * A genuine interior crossing should still come through when
         * `no_endpoint_touch` is on — the flag only filters endpoint-
         * only matches.
         */
        assert_point(
            intersection([0.0, 0.0], [1.0, 1.0], [1.0, 0.0], [0.0, 1.0], true),
            [0.5, 0.5],
        );
    }
}
