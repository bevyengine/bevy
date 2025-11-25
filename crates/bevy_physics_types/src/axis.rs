//! Shared axis enumeration for joint types.

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    #[default]
    X,
    Y,
    Z,
}
