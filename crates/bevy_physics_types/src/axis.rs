//! Shared axis enumeration for joint types.
//!
//! The [`Axis`] enum specifies which local axis a joint operates on.

/// Primary axis for joint constraints.
///
/// Specifies which of the three local coordinate axes a joint operates
/// along or around. Used by revolute, prismatic, and spherical joints.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// The X axis (typically "right" in many conventions).
    #[default]
    X,
    /// The Y axis (typically "up" in Y-up conventions).
    Y,
    /// The Z axis (typically "forward" or "up" depending on convention).
    Z,
}
