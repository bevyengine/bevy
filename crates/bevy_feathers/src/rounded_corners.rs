//! Mechanism for specifying which corners of a widget are rounded, used for segmented buttons
//! and control groups.
use bevy_ui::{BorderRadius, Val};

/// Allows specifying which corners are rounded and which are sharp. All rounded corners
/// have the same radius. Not all combinations are supported, only the ones that make
/// sense for a segmented buttons.
///
/// A typical use case would be a segmented button consisting of 3 individual buttons in a
/// row. In that case, you would have the leftmost button have rounded corners on the left,
/// the right-most button have rounded corners on the right, and the center button have
/// only sharp corners.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RoundedCorners {
    /// No corners are rounded.
    None,
    #[default]
    /// All corners are rounded.
    All,
    /// Top-left corner is rounded.
    TopLeft,
    /// Top-right corner is rounded.
    TopRight,
    /// Bottom-right corner is rounded.
    BottomRight,
    /// Bottom-left corner is rounded.
    BottomLeft,
    /// Top corners are rounded.
    Top,
    /// Right corners are rounded.
    Right,
    /// Bottom corners are rounded.
    Bottom,
    /// Left corners are rounded.
    Left,
}

impl RoundedCorners {
    /// Convert the `RoundedCorners` to a `BorderRadius` for use in a `Node`.
    pub fn to_border_radius(&self, radius: f32) -> BorderRadius {
        let radius = Val::Px(radius);
        let zero = Val::ZERO;
        match self {
            RoundedCorners::None => BorderRadius::all(zero),
            RoundedCorners::All => BorderRadius::all(radius),
            RoundedCorners::TopLeft => BorderRadius {
                top_left: radius,
                top_right: zero,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::TopRight => BorderRadius {
                top_left: zero,
                top_right: radius,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::BottomRight => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: radius,
                bottom_left: zero,
            },
            RoundedCorners::BottomLeft => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: zero,
                bottom_left: radius,
            },
            RoundedCorners::Top => BorderRadius {
                top_left: radius,
                top_right: radius,
                bottom_right: zero,
                bottom_left: zero,
            },
            RoundedCorners::Right => BorderRadius {
                top_left: zero,
                top_right: radius,
                bottom_right: radius,
                bottom_left: zero,
            },
            RoundedCorners::Bottom => BorderRadius {
                top_left: zero,
                top_right: zero,
                bottom_right: radius,
                bottom_left: radius,
            },
            RoundedCorners::Left => BorderRadius {
                top_left: radius,
                top_right: zero,
                bottom_right: zero,
                bottom_left: radius,
            },
        }
    }
}
