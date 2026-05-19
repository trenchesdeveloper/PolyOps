//! Point equality, port of upstream `src/equals.ts`.
//!
//! Upstream uses strict `===` on both coordinates and explicitly avoids
//! epsilon-based comparison (see the commented-out TODO in the JS
//! file referencing
//! <https://github.com/w8r/martinez/issues/6#issuecomment-262847164>).
//! The algorithm normalizes points through the sweep, so equal events
//! always carry bit-identical coordinates by the time `equals` is
//! called.
//!
//! Rust's `f64 == f64` follows IEEE 754: `NaN != NaN`. This matches
//! JS `===` semantics on numbers, so the direct translation is
//! correct.

use crate::types::Position;

/// Whether two positions are bit-identical on both coordinates.
///
/// `NaN` coordinates compare unequal to everything including
/// themselves, mirroring upstream JS behavior.
pub(crate) fn equals(p1: Position, p2: Position) -> bool {
    p1[0] == p2[0] && p1[1] == p2[1]
}

/**********************************************************************
 * Tests — upstream has no dedicated equals.test.ts; these cover the
 * cases the algorithm's sweep relies on.
 **********************************************************************/
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_points_equal() {
        assert!(equals([1.0, 2.0], [1.0, 2.0]));
    }

    #[test]
    fn different_x_unequal() {
        assert!(!equals([1.0, 2.0], [1.1, 2.0]));
    }

    #[test]
    fn different_y_unequal() {
        assert!(!equals([1.0, 2.0], [1.0, 2.1]));
    }

    #[test]
    fn zero_zero_equals_zero_zero() {
        assert!(equals([0.0, 0.0], [0.0, 0.0]));
    }

    #[test]
    fn positive_and_negative_zero_are_equal() {
        /**
         * IEEE 754: +0.0 == -0.0 is true. Upstream JS agrees
         * (`0 === -0` is true). The sweep treats them as the same
         * point, which is correct because they round-trip identically
         * through any subsequent arithmetic.
         */
        assert!(equals([0.0, 1.0], [-0.0, 1.0]));
        assert!(equals([1.0, 0.0], [1.0, -0.0]));
    }

    #[test]
    fn nan_never_equals_anything() {
        /**
         * IEEE 754: NaN != NaN. The algorithm shouldn't ever produce
         * NaN coordinates; if it does, this guarantees they don't
         * silently get merged.
         */
        let nan = f64::NAN;
        assert!(!equals([nan, 1.0], [nan, 1.0]));
        assert!(!equals([1.0, nan], [1.0, nan]));
        assert!(!equals([nan, nan], [nan, nan]));
    }
}
