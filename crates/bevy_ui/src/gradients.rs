use crate::{Position, Val};
use bevy_color::Color;
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use core::{f32, f32::consts::TAU};

/// A color stop for a gradient
#[derive(Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Default, PartialEq, Debug)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ColorStop {
    /// Color
    pub color: Color,
    /// Logical position along the gradient line.
    /// Stop positions are relative to the start of the gradient and not other stops.
    pub point: Val,
    /// Normalized position between this and the following stop of the interpolation midpoint.
    pub hint: f32,
}

impl ColorStop {
    /// Create a new color stop
    pub fn new(color: impl Into<Color>, point: Val) -> Self {
        Self {
            color: color.into(),
            point,
            hint: 0.5,
        }
    }

    /// An automatic color stop.
    /// The positions of automatic stops are interpolated evenly between explicit stops.
    pub fn auto(color: impl Into<Color>) -> Self {
        Self {
            color: color.into(),
            point: Val::Auto,
            hint: 0.5,
        }
    }

    pub fn hint(mut self, hint: f32) -> Self {
        self.hint = hint;
        self
    }
}

impl From<(Color, Val)> for ColorStop {
    fn from((color, stop): (Color, Val)) -> Self {
        Self {
            color,
            point: stop,
            hint: 0.5,
        }
    }
}

impl From<Color> for ColorStop {
    fn from(color: Color) -> Self {
        Self {
            color,
            point: Val::Auto,
            hint: 0.5,
        }
    }
}

impl Default for ColorStop {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            point: Val::Auto,
            hint: 0.5,
        }
    }
}

/// An angular color stop for a conic gradient
#[derive(Default, Debug, Copy, Clone, PartialEq, Reflect)]
#[reflect(Default, PartialEq, Debug)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct AngularColorStop {
    /// Color of the stop
    pub color: Color,
    /// The angle of the stop.
    /// Angles are relative to the start of the gradient and not other stops.
    /// If set to `None` the angle of the stop will be interpolated between the explicit stops or 0 and 2 PI degrees if there no explicit stops.
    pub angle: Option<f32>,
    /// Normalized angle between this and the following stop of the interpolation midpoint.
    pub hint: f32,
}

impl AngularColorStop {
    // Create a new color stop
    pub fn new(color: impl Into<Color>, angle: f32) -> Self {
        Self {
            color: color.into(),
            angle: Some(angle),
            hint: 0.5,
        }
    }

    /// An angular stop without an explicit angle. The angles of automatic stops
    /// are interpolated evenly between explicit stops.
    pub fn auto(color: impl Into<Color>) -> Self {
        Self {
            color: color.into(),
            angle: None,
            hint: 0.5,
        }
    }

    pub fn hint(mut self, hint: f32) -> Self {
        self.hint = hint;
        self
    }
}

#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum Gradient {
    /// A linear gradient
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient>
    Linear {
        /// The direction of the gradient.
        /// An angle of `0.` points upward, angles increasing clockwise.
        angle: f32,
        /// The list of color stops
        stops: Vec<ColorStop>,
    },
    /// A radial gradient
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/radial-gradient>
    Radial {
        /// The center of the radial gradient
        position: Position,
        /// Defines the end shape of the radial gradient
        shape: RadialGradientShape,
        /// The list of color stops
        stops: Vec<ColorStop>,
    },
    /// A conic gradient
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/radial-gradient>
    Conic {
        /// The center of the conic gradient
        position: Position,
        /// The list of color stops
        stops: Vec<AngularColorStop>,
    },
}

impl Gradient {
    /// A linear gradient transitioning from bottom to top
    pub const TO_TOP: f32 = 0.;
    /// A linear gradient transitioning from bottom-left to top-right
    pub const TO_TOP_RIGHT: f32 = TAU / 8.;
    /// A linear gradient transitioning from left to right
    pub const TO_RIGHT: f32 = 2. * Self::TO_TOP_RIGHT;
    /// A linear gradient transitioning from top-left to bottom-right
    pub const TO_BOTTOM_RIGHT: f32 = 3. * Self::TO_TOP_RIGHT;
    /// A linear gradient transitioning from top to bottom
    pub const TO_BOTTOM: f32 = 4. * Self::TO_TOP_RIGHT;
    /// A linear gradient transitioning from top-right to bottom-left
    pub const TO_BOTTOM_LEFT: f32 = 5. * Self::TO_TOP_RIGHT;
    /// A linear gradient transitioning from right to left
    pub const TO_LEFT: f32 = 6. * Self::TO_TOP_RIGHT;
    /// A linear gradient transitioning from bottom-right to top-left
    pub const TO_TOP_LEFT: f32 = 7. * Self::TO_TOP_RIGHT;

    /// Returns true if the gradient has no stops.
    pub fn is_empty(&self) -> bool {
        match self {
            Gradient::Linear { stops, .. } | Gradient::Radial { stops, .. } => stops.is_empty(),
            Gradient::Conic { stops, .. } => stops.is_empty(),
        }
    }

    /// If the gradient has only a single color stop `get_single` returns its color.
    pub fn get_single(&self) -> Option<Color> {
        match self {
            Gradient::Linear { stops, .. } | Gradient::Radial { stops, .. } => stops
                .first()
                .and_then(|stop| (stops.len() == 1).then_some(stop.color)),
            Gradient::Conic { stops, .. } => stops
                .first()
                .and_then(|stop| (stops.len() == 1).then_some(stop.color)),
        }
    }

    /// A linear gradient transitioning from bottom to top
    pub fn linear_to_top(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_TOP,
            stops,
        }
    }

    /// A linear gradient transitioning from bottom-left to top-right
    pub fn linear_to_top_right(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_TOP_RIGHT,
            stops,
        }
    }

    /// A linear gradient transitioning from left to right
    pub fn linear_to_right(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_RIGHT,
            stops,
        }
    }

    /// A linear gradient transitioning from top-left to bottom right
    pub fn linear_to_bottom_right(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_BOTTOM_RIGHT,
            stops,
        }
    }

    /// A linear gradient transitioning from top to bottom
    pub fn linear_to_bottom(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_BOTTOM,
            stops,
        }
    }

    /// A linear gradient transitioning from top-right to bottom-left
    pub fn linear_to_bottom_left(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_BOTTOM_LEFT,
            stops,
        }
    }

    /// A linear gradient transitioning from right-to-left
    pub fn linear_to_left(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_LEFT,
            stops,
        }
    }

    /// A linear gradient transitioning from bottom-right to top-left
    pub fn linear_to_top_left(stops: Vec<ColorStop>) -> Gradient {
        Self::Linear {
            angle: Self::TO_TOP_LEFT,
            stops,
        }
    }

    /// A radial gradient
    pub fn radial(
        position: Position,
        shape: RadialGradientShape,
        stops: Vec<ColorStop>,
    ) -> Gradient {
        Self::Radial {
            position,
            shape,
            stops,
        }
    }

    /// A conic gradient
    pub fn conic(position: Position, stops: Vec<AngularColorStop>) -> Gradient {
        Self::Conic { position, stops }
    }
}

#[derive(Component, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// A UI node that displays a gradient
pub struct BackgroundGradient(pub Vec<Gradient>);

impl From<Gradient> for BackgroundGradient {
    fn from(value: Gradient) -> Self {
        Self(vec![value])
    }
}

#[derive(Component, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// A UI node border that displays a gradient
pub struct BorderGradient(pub Vec<Gradient>);

impl From<Gradient> for BorderGradient {
    fn from(value: Gradient) -> Self {
        Self(vec![value])
    }
}

#[derive(Default, Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq, Default)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum RadialGradientShape {
    /// A circle with radius equal to the distance from its center to the closest side
    #[default]
    ClosestSide,
    /// A circle with radius equal to the distance from its center to the farthest side
    FarthestSide,
    /// An ellipse with extents equal to the distance from its center to the nearest corner
    ClosestCorner,
    /// An ellipse with extents equal to the distance from its center to the farthest corner
    FarthestCorner,
    /// A circle
    Circle(Val),
    /// An ellipse
    Ellipse(Val, Val),
}

fn close_side(p: f32, h: f32) -> f32 {
    (-h - p).abs().min((h - p).abs())
}

fn far_side(p: f32, h: f32) -> f32 {
    (-h - p).abs().max((h - p).abs())
}

fn close_side2(p: Vec2, h: Vec2) -> f32 {
    close_side(p.x, h.x).min(close_side(p.y, h.y))
}

fn far_side2(p: Vec2, h: Vec2) -> f32 {
    far_side(p.x, h.x).max(far_side(p.y, h.y))
}

impl RadialGradientShape {
    /// Resolve the physical dimensions of the end shape of the radial gradient
    pub fn resolve(
        self,
        position: Vec2,
        scale_factor: f32,
        physical_size: Vec2,
        physical_target_size: Vec2,
    ) -> Vec2 {
        let half_size = 0.5 * physical_size;
        match self {
            RadialGradientShape::ClosestSide => Vec2::splat(close_side2(position, half_size)),
            RadialGradientShape::FarthestSide => Vec2::splat(far_side2(position, half_size)),
            RadialGradientShape::ClosestCorner => Vec2::new(
                close_side(position.x, half_size.x),
                close_side(position.y, half_size.y),
            ),
            RadialGradientShape::FarthestCorner => Vec2::new(
                far_side(position.x, half_size.x),
                far_side(position.y, half_size.y),
            ),
            RadialGradientShape::Circle(radius) => Vec2::splat(
                radius
                    .resolve(scale_factor, physical_size.x, physical_target_size)
                    .unwrap_or(0.),
            ),
            RadialGradientShape::Ellipse(x, y) => Vec2::new(
                x.resolve(scale_factor, physical_size.x, physical_target_size)
                    .unwrap_or(0.),
                y.resolve(scale_factor, physical_size.y, physical_target_size)
                    .unwrap_or(0.),
            ),
        }
    }
}
