//! Classification of segment edges, mirroring upstream `src/edge_type.ts`.

#![allow(dead_code)]

/// How a sweep-event segment behaves relative to the result polygon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EdgeType {
    Normal,
    NonContributing,
    SameTransition,
    DifferentTransition,
}
