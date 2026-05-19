//! Output contour representation, port of upstream `src/contour.ts`.

#![allow(dead_code)]

use crate::types::Position;

/// One closed contour in the output. Carries the hole/exterior flag and
/// the IDs of any contours that nest inside it.
#[derive(Debug, Clone)]
pub(crate) struct Contour {
    pub points: Vec<Position>,
    pub hole_ids: Vec<usize>,
    pub hole_of: Option<usize>,
    pub depth: i32,
}

impl Contour {
    pub(crate) fn new() -> Self {
        Self {
            points: Vec::new(),
            hole_ids: Vec::new(),
            hole_of: None,
            depth: 0,
        }
    }

    pub(crate) fn is_exterior(&self) -> bool {
        self.hole_of.is_none()
    }
}
