use crate::{UiPosition, Val};
use bevy_color::{Color, Srgba};
use bevy_ecs::component::Component;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use bevy_utils::default;
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

    /// A color stop with its position in logical pixels.
    pub fn px(color: impl Into<Color>, px: f32) -> Self {
        Self {
            color: color.into(),
            point: Val::Px(px),
            hint: 0.5,
        }
    }

    /// A color stop with a percentage position.
    pub fn percent(color: impl Into<Color>, percent: f32) -> Self {
        Self {
            color: color.into(),
            point: Val::Percent(percent),
            hint: 0.5,
        }
    }

    // Set the interpolation midpoint between this and the following stop
    pub fn with_hint(mut self, hint: f32) -> Self {
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

impl From<Srgba> for ColorStop {
    fn from(color: Srgba) -> Self {
        Self {
            color: color.into(),
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
#[derive(Debug, Copy, Clone, PartialEq, Reflect)]
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
    /// Given angles are clamped to between `0.`, and [`TAU`].
    /// This means that a list of stops:
    /// ```
    /// # use std::f32::consts::TAU;
    /// # use bevy_ui::AngularColorStop;
    /// # use bevy_color::{Color, palettes::css::{RED, BLUE}};
    /// let stops = [
    ///     AngularColorStop::new(Color::WHITE, 0.),
    ///     AngularColorStop::new(Color::BLACK, -1.),
    ///     AngularColorStop::new(RED, 2. * TAU),
    ///     AngularColorStop::new(BLUE, TAU),
    /// ];
    /// ```
    /// is equivalent to:
    /// ```
    /// # use std::f32::consts::TAU;
    /// # use bevy_ui::AngularColorStop;
    /// # use bevy_color::{Color, palettes::css::{RED, BLUE}};
    /// let stops = [
    ///     AngularColorStop::new(Color::WHITE, 0.),
    ///     AngularColorStop::new(Color::BLACK, 0.),
    ///     AngularColorStop::new(RED, TAU),
    ///     AngularColorStop::new(BLUE, TAU),
    /// ];
    /// ```
    /// Resulting in a black to red gradient, not white to blue.
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

    // Set the interpolation midpoint between this and the following stop
    pub fn with_hint(mut self, hint: f32) -> Self {
        self.hint = hint;
        self
    }
}

impl From<(Color, f32)> for AngularColorStop {
    fn from((color, angle): (Color, f32)) -> Self {
        Self {
            color,
            angle: Some(angle),
            hint: 0.5,
        }
    }
}

impl From<Color> for AngularColorStop {
    fn from(color: Color) -> Self {
        Self {
            color,
            angle: None,
            hint: 0.5,
        }
    }
}

impl From<Srgba> for AngularColorStop {
    fn from(color: Srgba) -> Self {
        Self {
            color: color.into(),
            angle: None,
            hint: 0.5,
        }
    }
}

impl Default for AngularColorStop {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            angle: None,
            hint: 0.5,
        }
    }
}

/// A linear gradient
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient>
#[derive(Default, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct LinearGradient {
    /// The color space used for interpolation.
    pub color_space: InterpolationColorSpace,
    /// The direction of the gradient in radians.
    /// An angle of `0.` points upward, with the value increasing in the clockwise direction.
    pub angle: f32,
    /// The list of color stops
    pub stops: Vec<ColorStop>,
}

impl LinearGradient {
    /// Angle of a linear gradient transitioning from bottom to top
    pub const TO_TOP: f32 = 0.;
    /// Angle of a linear gradient transitioning from bottom-left to top-right
    pub const TO_TOP_RIGHT: f32 = TAU / 8.;
    /// Angle of a linear gradient transitioning from left to right
    pub const TO_RIGHT: f32 = 2. * Self::TO_TOP_RIGHT;
    /// Angle of a linear gradient transitioning from top-left to bottom-right
    pub const TO_BOTTOM_RIGHT: f32 = 3. * Self::TO_TOP_RIGHT;
    /// Angle of a linear gradient transitioning from top to bottom
    pub const TO_BOTTOM: f32 = 4. * Self::TO_TOP_RIGHT;
    /// Angle of a linear gradient transitioning from top-right to bottom-left
    pub const TO_BOTTOM_LEFT: f32 = 5. * Self::TO_TOP_RIGHT;
    /// Angle of a linear gradient transitioning from right to left
    pub const TO_LEFT: f32 = 6. * Self::TO_TOP_RIGHT;
    /// Angle of a linear gradient transitioning from bottom-right to top-left
    pub const TO_TOP_LEFT: f32 = 7. * Self::TO_TOP_RIGHT;

    /// Create a new linear gradient
    pub fn new(angle: f32, stops: Vec<ColorStop>) -> Self {
        Self {
            angle,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from bottom to top
    pub fn to_top(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_TOP,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from bottom-left to top-right
    pub fn to_top_right(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_TOP_RIGHT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from left to right
    pub fn to_right(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_RIGHT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from top-left to bottom-right
    pub fn to_bottom_right(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_BOTTOM_RIGHT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from top to bottom
    pub fn to_bottom(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_BOTTOM,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from top-right to bottom-left
    pub fn to_bottom_left(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_BOTTOM_LEFT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from right to left
    pub fn to_left(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_LEFT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient transitioning from bottom-right to top-left
    pub fn to_top_left(stops: Vec<ColorStop>) -> Self {
        Self {
            angle: Self::TO_TOP_LEFT,
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    /// A linear gradient with the given angle in degrees
    pub fn degrees(degrees: f32, stops: Vec<ColorStop>) -> Self {
        Self {
            angle: degrees.to_radians(),
            stops,
            color_space: InterpolationColorSpace::default(),
        }
    }

    pub fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
        self
    }
}

/// A radial gradient
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/radial-gradient>
#[derive(Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RadialGradient {
    /// The color space used for interpolation.
    pub color_space: InterpolationColorSpace,
    /// The center of the radial gradient
    pub position: UiPosition,
    /// Defines the end shape of the radial gradient
    pub shape: RadialGradientShape,
    /// The list of color stops
    pub stops: Vec<ColorStop>,
}

impl RadialGradient {
    /// Create a new radial gradient
    pub fn new(position: UiPosition, shape: RadialGradientShape, stops: Vec<ColorStop>) -> Self {
        Self {
            color_space: default(),
            position,
            shape,
            stops,
        }
    }

    pub fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
        self
    }
}

impl Default for RadialGradient {
    fn default() -> Self {
        Self {
            position: UiPosition::CENTER,
            shape: RadialGradientShape::ClosestCorner,
            stops: Vec::new(),
            color_space: default(),
        }
    }
}

/// A conic gradient
///
/// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/conic-gradient>
#[derive(Default, Clone, PartialEq, Debug, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ConicGradient {
    /// The color space used for interpolation.
    pub color_space: InterpolationColorSpace,
    /// The starting angle of the gradient in radians
    pub start: f32,
    /// The center of the conic gradient
    pub position: UiPosition,
    /// The list of color stops
    pub stops: Vec<AngularColorStop>,
}

impl ConicGradient {
    /// Create a new conic gradient
    pub fn new(position: UiPosition, stops: Vec<AngularColorStop>) -> Self {
        Self {
            color_space: default(),
            start: 0.,
            position,
            stops,
        }
    }

    /// Sets the starting angle of the gradient in radians
    pub fn with_start(mut self, start: f32) -> Self {
        self.start = start;
        self
    }

    /// Sets the position of the gradient
    pub fn with_position(mut self, position: UiPosition) -> Self {
        self.position = position;
        self
    }

    pub fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
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
    Linear(LinearGradient),
    /// A radial gradient
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/linear-gradient>
    Radial(RadialGradient),
    /// A conic gradient
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gradient/radial-gradient>
    Conic(ConicGradient),
}

impl Gradient {
    /// Returns true if the gradient has no stops.
    pub fn is_empty(&self) -> bool {
        match self {
            Gradient::Linear(gradient) => gradient.stops.is_empty(),
            Gradient::Radial(gradient) => gradient.stops.is_empty(),
            Gradient::Conic(gradient) => gradient.stops.is_empty(),
        }
    }

    /// If the gradient has only a single color stop, `get_single` returns its color.
    pub fn get_single(&self) -> Option<Color> {
        match self {
            Gradient::Linear(gradient) => gradient
                .stops
                .first()
                .and_then(|stop| (gradient.stops.len() == 1).then_some(stop.color)),
            Gradient::Radial(gradient) => gradient
                .stops
                .first()
                .and_then(|stop| (gradient.stops.len() == 1).then_some(stop.color)),
            Gradient::Conic(gradient) => gradient
                .stops
                .first()
                .and_then(|stop| (gradient.stops.len() == 1).then_some(stop.color)),
        }
    }
}

impl From<LinearGradient> for Gradient {
    fn from(value: LinearGradient) -> Self {
        Self::Linear(value)
    }
}

impl From<RadialGradient> for Gradient {
    fn from(value: RadialGradient) -> Self {
        Self::Radial(value)
    }
}

impl From<ConicGradient> for Gradient {
    fn from(value: ConicGradient) -> Self {
        Self::Conic(value)
    }
}

#[derive(Component, Clone, PartialEq, Debug, Default, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// A UI node that displays a gradient
pub struct BackgroundGradient(pub Vec<Gradient>);

impl<T: Into<Gradient>> From<T> for BackgroundGradient {
    fn from(value: T) -> Self {
        Self(vec![value.into()])
    }
}

#[derive(Component, Clone, PartialEq, Debug, Default, Reflect)]
#[reflect(PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// A UI node border that displays a gradient
pub struct BorderGradient(pub Vec<Gradient>);

impl<T: Into<Gradient>> From<T> for BorderGradient {
    fn from(value: T) -> Self {
        Self(vec![value.into()])
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
    ClosestSide,
    /// A circle with radius equal to the distance from its center to the farthest side
    FarthestSide,
    /// An ellipse with extents equal to the distance from its center to the nearest corner
    #[default]
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

/// The color space used for interpolation.
#[derive(Default, Copy, Clone, Hash, Debug, PartialEq, Eq, Reflect)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum InterpolationColorSpace {
    /// Interpolates in `OKLab` space.
    #[default]
    OkLab,
    /// Interpolates in OKLCH space, taking the shortest hue path.
    OkLch,
    /// Interpolates in OKLCH space, taking the longest hue path.
    OkLchLong,
    /// Interpolates in sRGB space.
    Srgb,
    /// Interpolates in linear sRGB space.
    LinearRgb,
    /// Interpolates in HSL space, taking the shortest hue path.
    Hsl,
    /// Interpolates in HSL space, taking the longest hue path.
    HslLong,
    /// Interpolates in HSV space, taking the shortest hue path.
    Hsv,
    /// Interpolates in HSV space, taking the longest hue path.
    HsvLong,
}

/// Set the color space used for interpolation.
pub trait InColorSpace: Sized {
    /// Interpolate in the given `color_space`.
    fn in_color_space(self, color_space: InterpolationColorSpace) -> Self;

    /// Interpolate in `OKLab` space.
    fn in_oklab(self) -> Self {
        self.in_color_space(InterpolationColorSpace::OkLab)
    }

    /// Interpolate in OKLCH space (short hue path).
    fn in_oklch(self) -> Self {
        self.in_color_space(InterpolationColorSpace::OkLch)
    }

    /// Interpolate in OKLCH space (long hue path).
    fn in_oklch_long(self) -> Self {
        self.in_color_space(InterpolationColorSpace::OkLchLong)
    }

    /// Interpolate in sRGB space.
    fn in_srgb(self) -> Self {
        self.in_color_space(InterpolationColorSpace::Srgb)
    }

    /// Interpolate in linear sRGB space.
    fn in_linear_rgb(self) -> Self {
        self.in_color_space(InterpolationColorSpace::LinearRgb)
    }
}

impl InColorSpace for LinearGradient {
    /// Interpolate in the given `color_space`.
    fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
        self
    }
}

impl InColorSpace for RadialGradient {
    /// Interpolate in the given `color_space`.
    fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
        self
    }
}

impl InColorSpace for ConicGradient {
    /// Interpolate in the given `color_space`.
    fn in_color_space(mut self, color_space: InterpolationColorSpace) -> Self {
        self.color_space = color_space;
        self
    }
}
