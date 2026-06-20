//! Boolean operation enum, mirroring upstream `src/operation.ts`.

/// Which Boolean operation to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Operation {
    /// `subject ∩ clipping`
    Intersection = 0,
    /// `subject ∪ clipping`
    Union = 1,
    /// `subject \ clipping`
    Difference = 2,
    /// `subject ⊕ clipping`
    Xor = 3,
}
