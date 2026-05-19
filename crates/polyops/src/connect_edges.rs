//! Connect subdivided edges into output contours, port of upstream
//! `src/connect_edges.ts`. This is the largest upstream file (187 LOC)
//! and the one most likely to expose parity issues.

#![allow(dead_code)]

use crate::contour::Contour;

/// Walk the sorted result events and stitch them into closed contours,
/// then resolve hole-of-which-exterior relationships.
///
/// TODO: port from upstream `src/connect_edges.ts`.
pub(crate) fn connect_edges() -> Vec<Contour> {
    todo!("port src/connect_edges.ts")
}
