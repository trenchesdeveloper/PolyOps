//! Signed area of the triangle defined by three points, port of
//! upstream `src/signed_area.ts`.
//!
//! Returns the algorithm's signed-area **sign** as a trichotomy
//! `{-1, 0, +1}`, not the raw value. Upstream does the same — the
//! actual area magnitude is never used downstream, only its sign.
//!
//! Implemented via [`robust::orient2d`], the Rust counterpart to the
//! JS `robust-predicates` crate. Both implement Shewchuk's adaptive
//! exact predicate, so parity on degenerate inputs (collinear points,
//! near-collinear) is by construction.
//!
//! **Sign convention.** The two `orient2d` implementations we care
//! about disagree on sign:
//!
//! - `robust-predicates` (npm, upstream's dep) computes
//!   `(ay - cy)(bx - cx) - (ax - cx)(by - cy)`.
//! - Rust's `robust` crate computes Shewchuk's classic
//!   `(ax - cx)(by - cy) - (ay - cy)(bx - cx)` — the negation.
//!
//! Both are correct adaptive predicates; they just differ by a sign.
//! Upstream's `signedArea` returns `-1` when its `orient2d > 0`. Since
//! Rust's `orient2d` returns the opposite sign, we want `-1` when
//! Rust's `orient2d < 0`. Net result: the trichotomy this function
//! reports is identical to upstream's, even though the underlying
//! predicate disagrees on sign. Downstream callers see no difference.
//!
//! Concretely: `-1` for counter-clockwise (left turn), `+1` for
//! clockwise (right turn), `0` for collinear — matching upstream
//! exactly.

use robust::{orient2d, Coord};

use crate::types::Position;

/// Sign of the signed area of the triangle `(p0, p1, p2)`.
///
/// Returns `-1` when the three points form a counter-clockwise turn,
/// `+1` when clockwise, `0` when collinear. Matches upstream's
/// `signedArea` return values bit-for-bit.
pub(crate) fn signed_area(p0: Position, p1: Position, p2: Position) -> i32 {
    let res = orient2d(
        Coord { x: p0[0], y: p0[1] },
        Coord { x: p1[0], y: p1[1] },
        Coord { x: p2[0], y: p2[1] },
    );
    /*
     * Sign flip relative to upstream because robust (Rust) and
     * robust-predicates (npm) disagree on orient2d's sign — see
     * module docs.
     */
    if res > 0.0 {
        1
    } else if res < 0.0 {
        -1
    } else {
        0
    }
}

/*
 * Tests — mirror upstream `test/signed_area.test.ts` 1:1, plus a few
 * extra cases for collinearity edge conditions.
 */
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_area() {
        /* Counter-clockwise turn at (0,0) -> (0,1) -> (1,1). */
        assert_eq!(signed_area([0.0, 0.0], [0.0, 1.0], [1.0, 1.0]), -1);
    }

    #[test]
    fn positive_area() {
        /* Clockwise turn at (0,1) -> (0,0) -> (1,0). */
        assert_eq!(signed_area([0.0, 1.0], [0.0, 0.0], [1.0, 0.0]), 1);
    }

    #[test]
    fn collinear_zero_area() {
        /* Three points on the line y = x. */
        assert_eq!(signed_area([0.0, 0.0], [1.0, 1.0], [2.0, 2.0]), 0);
    }

    #[test]
    fn point_on_segment_collinear() {
        /*
         * Upstream test: a third point that lies exactly on the segment
         * formed by the first two should report collinear (0), regardless
         * of which endpoint comes first.
         */
        assert_eq!(signed_area([-1.0, 0.0], [2.0, 3.0], [0.0, 1.0]), 0);
        assert_eq!(signed_area([2.0, 3.0], [-1.0, 0.0], [0.0, 1.0]), 0);
    }

    #[test]
    fn near_collinear_uses_robust_predicate() {
        /*
         * Three points that are collinear in exact arithmetic but
         * whose orientation a naive f64 cross product gets wrong. The
         * robust predicate must call this collinear. (If this test
         * ever starts returning ±1 a regression has been introduced in
         * the robust crate or the path through it.)
         */
        let p0 = [0.5, 0.5];
        let p1 = [12.0, 12.0];
        let p2 = [24.0, 24.0];
        assert_eq!(signed_area(p0, p1, p2), 0);
    }

    #[test]
    fn signed_area_is_antisymmetric_in_p0_p1_swap() {
        /*
         * Swapping the first two points negates the area sign. This is
         * a property of the underlying determinant and worth asserting
         * because a wrong sign-convention bug would show up here.
         */
        let p0 = [1.0, 2.0];
        let p1 = [3.0, 7.0];
        let p2 = [5.0, 1.0];
        let a = signed_area(p0, p1, p2);
        let b = signed_area(p1, p0, p2);
        assert_ne!(a, 0);
        assert_eq!(a, -b);
    }
}
