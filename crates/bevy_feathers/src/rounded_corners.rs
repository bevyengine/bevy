use bevy::prelude::*;

/// Options for rendering rounded corners.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum RoundedCorners {
    None,
    #[default]
    All,
    TopLeft,
    TopRight,
    BottomRight,
    BottomLeft,
    Top,
    Right,
    Bottom,
    Left,
}

impl RoundedCorners {
    /// Convert the `RoundedCorners` to a `Vec4` for use in a shader.
    pub fn to_vec(&self, radius: f32) -> Vec4 {
        match self {
            RoundedCorners::None => Vec4::new(0.0, 0.0, 0.0, 0.0),
            RoundedCorners::All => Vec4::new(radius, radius, radius, radius),
            RoundedCorners::TopLeft => Vec4::new(radius, 0.0, 0.0, 0.0),
            RoundedCorners::TopRight => Vec4::new(0.0, radius, 0.0, 0.0),
            RoundedCorners::BottomRight => Vec4::new(0.0, 0.0, radius, 0.0),
            RoundedCorners::BottomLeft => Vec4::new(0.0, 0.0, 0.0, radius),
            RoundedCorners::Top => Vec4::new(radius, radius, 0.0, 0.0),
            RoundedCorners::Right => Vec4::new(0.0, radius, radius, 0.0),
            RoundedCorners::Bottom => Vec4::new(0.0, 0.0, radius, radius),
            RoundedCorners::Left => Vec4::new(radius, 0.0, 0.0, radius),
        }
    }

    /// Convert the `RoundedCorners` to a `BorderRadius` for use in a `Node`.
    pub fn to_border_radius(&self, radius: f32) -> BorderRadius {
        let radius = bevy::ui::Val::Px(radius);
        let zero = bevy::ui::Val::Px(0.0);
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
