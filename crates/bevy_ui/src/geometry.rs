use crate::val::*;
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_utils::default;

/// A type which is commonly used to define margins, paddings and borders.
///
/// # Examples
///
/// ## Margin
///
/// A margin is used to create space around UI elements, outside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let margin = UiRect::all(Val::Auto); // Centers the UI element
/// ```
///
/// ## Padding
///
/// A padding is used to create space around UI elements, inside of any defined borders.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let padding = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
///
/// ## Borders
///
/// A border is used to define the width of the border of a UI element.
///
/// ```
/// # use bevy_ui::{UiRect, Val};
/// #
/// let border = UiRect {
///     left: Val::Px(10.0),
///     right: Val::Px(20.0),
///     top: Val::Px(30.0),
///     bottom: Val::Px(40.0),
/// };
/// ```
#[derive(Copy, Clone, PartialEq, Debug, Reflect)]
#[reflect(Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct UiRect {
    /// The value corresponding to the left side of the UI rect.
    pub left: Val,
    /// The value corresponding to the right side of the UI rect.
    pub right: Val,
    /// The value corresponding to the top side of the UI rect.
    pub top: Val,
    /// The value corresponding to the bottom side of the UI rect.
    pub bottom: Val,
}

impl UiRect {
    pub const DEFAULT: Self = Self::all(Val::ZERO);
    pub const ZERO: Self = Self::all(Val::ZERO);
    pub const AUTO: Self = Self::all(Val::Auto);

    /// Creates a new [`UiRect`] from the values specified.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::new(
    ///     Val::Px(10.0),
    ///     Val::Px(20.0),
    ///     Val::Px(30.0),
    ///     Val::Px(40.0),
    /// );
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(30.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(40.0));
    /// ```
    pub const fn new(left: Val, right: Val, top: Val, bottom: Val) -> Self {
        UiRect {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates a new [`UiRect`] where all sides have the same value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn all(value: Val) -> Self {
        UiRect {
            left: value,
            right: value,
            top: value,
            bottom: value,
        }
    }

    /// Creates a new [`UiRect`] from the values specified in logical pixels.
    ///
    /// This is a shortcut for [`UiRect::new()`], applying [`Val::Px`] to all arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::px(10., 20., 30., 40.);
    /// assert_eq!(ui_rect.left, Val::Px(10.));
    /// assert_eq!(ui_rect.right, Val::Px(20.));
    /// assert_eq!(ui_rect.top, Val::Px(30.));
    /// assert_eq!(ui_rect.bottom, Val::Px(40.));
    /// ```
    pub const fn px(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Px(left),
            right: Val::Px(right),
            top: Val::Px(top),
            bottom: Val::Px(bottom),
        }
    }

    /// Creates a new [`UiRect`] from the values specified in percentages.
    ///
    /// This is a shortcut for [`UiRect::new()`], applying [`Val::Percent`] to all arguments.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::percent(5., 10., 2., 1.);
    /// assert_eq!(ui_rect.left, Val::Percent(5.));
    /// assert_eq!(ui_rect.right, Val::Percent(10.));
    /// assert_eq!(ui_rect.top, Val::Percent(2.));
    /// assert_eq!(ui_rect.bottom, Val::Percent(1.));
    /// ```
    pub const fn percent(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        UiRect {
            left: Val::Percent(left),
            right: Val::Percent(right),
            top: Val::Percent(top),
            bottom: Val::Percent(bottom),
        }
    }

    /// Creates a new [`UiRect`] where `left` and `right` take the given value,
    /// and `top` and `bottom` set to zero `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::horizontal(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn horizontal(value: Val) -> Self {
        Self {
            left: value,
            right: value,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `top` and `bottom` take the given value,
    /// and `left` and `right` are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::vertical(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn vertical(value: Val) -> Self {
        Self {
            top: value,
            bottom: value,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where both `left` and `right` take the value of `horizontal`, and both `top` and `bottom` take the value of `vertical`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::axes(Val::Px(10.0), Val::Percent(15.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Percent(15.0));
    /// assert_eq!(ui_rect.bottom, Val::Percent(15.0));
    /// ```
    pub const fn axes(horizontal: Val, vertical: Val) -> Self {
        Self {
            left: horizontal,
            right: horizontal,
            top: vertical,
            bottom: vertical,
        }
    }

    /// Creates a new [`UiRect`] where `left` takes the given value, and
    /// the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::left(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn left(left: Val) -> Self {
        Self {
            left,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `right` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::right(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn right(right: Val) -> Self {
        Self {
            right,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `top` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::top(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::ZERO);
    /// ```
    pub const fn top(top: Val) -> Self {
        Self {
            top,
            ..Self::DEFAULT
        }
    }

    /// Creates a new [`UiRect`] where `bottom` takes the given value,
    /// and the other fields are set to `Val::ZERO`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::bottom(Val::Px(10.0));
    ///
    /// assert_eq!(ui_rect.left, Val::ZERO);
    /// assert_eq!(ui_rect.right, Val::ZERO);
    /// assert_eq!(ui_rect.top, Val::ZERO);
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    pub const fn bottom(bottom: Val) -> Self {
        Self {
            bottom,
            ..Self::DEFAULT
        }
    }

    /// Returns the [`UiRect`] with its `left` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_left(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(10.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub const fn with_left(mut self, left: Val) -> Self {
        self.left = left;
        self
    }

    /// Returns the [`UiRect`] with its `right` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_right(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(10.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub const fn with_right(mut self, right: Val) -> Self {
        self.right = right;
        self
    }

    /// Returns the [`UiRect`] with its `top` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_top(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(10.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(20.0));
    /// ```
    #[inline]
    pub const fn with_top(mut self, top: Val) -> Self {
        self.top = top;
        self
    }

    /// Returns the [`UiRect`] with its `bottom` field set to the given value.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ui::{UiRect, Val};
    /// #
    /// let ui_rect = UiRect::all(Val::Px(20.0)).with_bottom(Val::Px(10.0));
    /// assert_eq!(ui_rect.left, Val::Px(20.0));
    /// assert_eq!(ui_rect.right, Val::Px(20.0));
    /// assert_eq!(ui_rect.top, Val::Px(20.0));
    /// assert_eq!(ui_rect.bottom, Val::Px(10.0));
    /// ```
    #[inline]
    pub const fn with_bottom(mut self, bottom: Val) -> Self {
        self.bottom = bottom;
        self
    }
}

impl Default for UiRect {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl From<Val> for UiRect {
    fn from(value: Val) -> Self {
        UiRect::all(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Default, Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
/// Responsive position relative to a UI node.
pub struct UiPosition {
    /// Normalized anchor point
    pub anchor: Vec2,
    /// Responsive horizontal position relative to the anchor point
    pub x: Val,
    /// Responsive vertical position relative to the anchor point
    pub y: Val,
}

impl Default for UiPosition {
    fn default() -> Self {
        Self::CENTER
    }
}

impl UiPosition {
    /// Position at the given normalized anchor point
    pub const fn anchor(anchor: Vec2) -> Self {
        Self {
            anchor,
            x: Val::ZERO,
            y: Val::ZERO,
        }
    }

    /// Position at the top-left corner
    pub const TOP_LEFT: Self = Self::anchor(Vec2::new(-0.5, -0.5));

    /// Position at the center of the left edge
    pub const LEFT: Self = Self::anchor(Vec2::new(-0.5, 0.0));

    /// Position at the bottom-left corner
    pub const BOTTOM_LEFT: Self = Self::anchor(Vec2::new(-0.5, 0.5));

    /// Position at the center of the top edge
    pub const TOP: Self = Self::anchor(Vec2::new(0.0, -0.5));

    /// Position at the center of the element
    pub const CENTER: Self = Self::anchor(Vec2::new(0.0, 0.0));

    /// Position at the center of the bottom edge
    pub const BOTTOM: Self = Self::anchor(Vec2::new(0.0, 0.5));

    /// Position at the top-right corner
    pub const TOP_RIGHT: Self = Self::anchor(Vec2::new(0.5, -0.5));

    /// Position at the center of the right edge
    pub const RIGHT: Self = Self::anchor(Vec2::new(0.5, 0.0));

    /// Position at the bottom-right corner
    pub const BOTTOM_RIGHT: Self = Self::anchor(Vec2::new(0.5, 0.5));

    /// Create a new position
    pub const fn new(anchor: Vec2, x: Val, y: Val) -> Self {
        Self { anchor, x, y }
    }

    /// Creates a position from self with the given `x` and `y` coordinates
    pub const fn at(self, x: Val, y: Val) -> Self {
        Self { x, y, ..self }
    }

    /// Creates a position from self with the given `x` coordinate
    pub const fn at_x(self, x: Val) -> Self {
        Self { x, ..self }
    }

    /// Creates a position from self with the given `y` coordinate
    pub const fn at_y(self, y: Val) -> Self {
        Self { y, ..self }
    }

    /// Creates a position in logical pixels from self with the given `x` and `y` coordinates
    pub const fn at_px(self, x: f32, y: f32) -> Self {
        self.at(Val::Px(x), Val::Px(y))
    }

    /// Creates a percentage position from self with the given `x` and `y` coordinates
    pub const fn at_percent(self, x: f32, y: f32) -> Self {
        self.at(Val::Percent(x), Val::Percent(y))
    }

    /// Creates a position from self with the given `anchor` point
    pub const fn with_anchor(self, anchor: Vec2) -> Self {
        Self { anchor, ..self }
    }

    /// Position relative to the top-left corner
    pub const fn top_left(x: Val, y: Val) -> Self {
        Self::TOP_LEFT.at(x, y)
    }

    /// Position relative to the left edge
    pub const fn left(x: Val, y: Val) -> Self {
        Self::LEFT.at(x, y)
    }

    /// Position relative to the bottom-left corner
    pub const fn bottom_left(x: Val, y: Val) -> Self {
        Self::BOTTOM_LEFT.at(x, y)
    }

    /// Position relative to the top edge
    pub const fn top(x: Val, y: Val) -> Self {
        Self::TOP.at(x, y)
    }

    /// Position relative to the center
    pub const fn center(x: Val, y: Val) -> Self {
        Self::CENTER.at(x, y)
    }

    /// Position relative to the bottom edge
    pub const fn bottom(x: Val, y: Val) -> Self {
        Self::BOTTOM.at(x, y)
    }

    /// Position relative to the top-right corner
    pub const fn top_right(x: Val, y: Val) -> Self {
        Self::TOP_RIGHT.at(x, y)
    }

    /// Position relative to the right edge
    pub const fn right(x: Val, y: Val) -> Self {
        Self::RIGHT.at(x, y)
    }

    /// Position relative to the bottom-right corner
    pub const fn bottom_right(x: Val, y: Val) -> Self {
        Self::BOTTOM_RIGHT.at(x, y)
    }

    /// Resolves the `UiPosition` into physical coordinates.
    pub fn resolve(self, context: &ValContext, physical_size: Vec2) -> Vec2 {
        let d = self.anchor.map(|p| if 0. < p { -1. } else { 1. });

        physical_size * self.anchor
            + d * Vec2::new(
                self.x.resolve(context, physical_size.x).unwrap_or(0.),
                self.y.resolve(context, physical_size.y).unwrap_or(0.),
            )
    }
}

impl From<Val> for UiPosition {
    fn from(x: Val) -> Self {
        Self { x, ..default() }
    }
}

impl From<(Val, Val)> for UiPosition {
    fn from((x, y): (Val, Val)) -> Self {
        Self { x, y, ..default() }
    }
}

/// The horizontal and vertical radii of a rounded corner's elliptical arc.
/// If one field is auto, the resolved radius will be circular and clamped to half the length of the node's shortest side.
/// If either field is zero, or both are auto, the node will have square corners.
///
/// # Example
///
/// ```
/// # use bevy_math::{Vec2, Vec2Swizzles};
/// # use bevy_ui::{CornerRadius, Val, ValContext};
/// # use bevy_text::{EmSize, RemSize};
/// let radius = Val::Px(10.0);
/// let size = Vec2::new(100.0, 50.0);
/// let viewport_size = Vec2::new(1920.0, 1080.0);
/// let em_size = EmSize(20.0);
/// let rem_size = RemSize(20.0);
/// let context = ValContext::new(1.0, viewport_size, em_size, rem_size);
///
/// let c1 = CornerRadius {
///     x: radius,
///     y: Val::ZERO,
/// };
/// let c2 = CornerRadius {
///     x: Val::ZERO,
///     y: radius,
/// };
/// let r = c1.resolve(&context, size);
/// assert_eq!(
///     r,
///     c2.resolve(&context, size).yx(),
/// );
/// assert_eq!(
///     r,
///     Vec2::new(10.0, 0.0),
/// );
/// ```
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct CornerRadius {
    /// Responsive horizontal radius.
    pub x: Val,
    /// Responsive vertical radius.
    pub y: Val,
}

impl CornerRadius {
    /// A fully rounded corner with a circular radius of half the length of the node's shortest side.
    pub const MAX: Self = Self {
        x: Val::MAX,
        y: Val::Auto,
    };

    /// An elliptical corner with a horizontal radius of half the node's width and a vertical radius of half its height.
    pub const MAX_ELLIPTICAL: Self = Self {
        x: Val::MAX,
        y: Val::MAX,
    };

    /// A square corner.
    pub const ZERO: Self = Self {
        x: Val::ZERO,
        y: Val::ZERO,
    };

    /// Creates a circular corner radius, with `radius` resolved relative to the node's shortest side and clamped to half its length.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Vec2;
    /// # use bevy_ui::{CornerRadius, Val, ValContext};
    /// # use bevy_text::{EmSize, RemSize};
    /// let radius = Val::Px(30.0);
    /// let size = Vec2::new(100.0, 50.0);
    /// let viewport_size = Vec2::new(1920.0, 1080.0);
    /// let em_size = EmSize(20.0);
    /// let rem_size = RemSize(20.0);
    /// let context = ValContext::new(1.0, viewport_size, em_size, rem_size);
    ///
    /// let c1 = CornerRadius::circular(radius);
    /// let c2 = CornerRadius {
    ///     x: Val::Auto,
    ///     y: radius,
    /// };
    ///
    /// let r = c1.resolve(&context, size);
    /// assert_eq!(
    ///     r,
    ///     c2.resolve(&context, size),
    /// );
    /// assert_eq!(
    ///     r,
    ///     Vec2::splat(25.0)
    /// );
    /// ```
    #[inline]
    pub const fn circular(radius: Val) -> Self {
        Self {
            x: radius,
            y: Val::Auto,
        }
    }

    /// Creates a corner radius with the same `radius` on both axes.
    ///
    /// Note that since each axis is resolved independently, the resolved radii may not be equal (see example below).
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_math::Vec2;
    /// # use bevy_ui::{CornerRadius, Val, ValContext};
    /// # use bevy_text::{EmSize, RemSize};
    /// let radius = Val::Px(30.0);
    /// let size = Vec2::new(100.0, 50.0);
    /// let viewport_size = Vec2::new(1920.0, 1080.0);
    /// let em_size = EmSize(20.0);
    /// let rem_size = RemSize(20.0);
    /// let context = ValContext::new(1.0, viewport_size, em_size, rem_size);
    ///
    /// let c1 = CornerRadius::all(radius);
    /// let c2 = CornerRadius {
    ///     x: radius,
    ///     y: radius,
    /// };
    ///
    /// let r = c1.resolve(&context, size);
    /// assert_eq!(
    ///     r,
    ///     c2.resolve(&context, size),
    /// );
    /// assert_eq!(
    ///     r,
    ///     Vec2::new(30., 25.)
    /// );
    /// ```
    #[inline]
    pub const fn all(radius: Val) -> Self {
        Self {
            x: radius,
            y: radius,
        }
    }

    /// Creates a corner radius with the given horizontal (`x`) and vertical (`y`) radii.
    #[inline]
    pub const fn new(x: Val, y: Val) -> Self {
        Self { x, y }
    }

    /// Resolves this corner radius into horizontal and vertical radii in physical pixels.
    pub fn resolve(self, context: &ValContext, size: Vec2) -> Vec2 {
        match self {
            Self {
                x: Val::Auto,
                y: Val::Auto,
            } => Vec2::ZERO,
            Self {
                x: radius,
                y: Val::Auto,
            }
            | Self {
                x: Val::Auto,
                y: radius,
            } => Vec2::splat(
                radius
                    .resolve(context, size.min_element())
                    .unwrap_or(0.)
                    .clamp(0., 0.5 * size.min_element()),
            ),
            Self { x, y } => Vec2::new(
                x.resolve(context, size.x).unwrap_or(0.),
                y.resolve(context, size.y).unwrap_or(0.),
            )
            .clamp(Vec2::ZERO, 0.5 * size),
        }
    }
}

impl Default for CornerRadius {
    fn default() -> Self {
        Self {
            x: auto(),
            y: auto(),
        }
    }
}

impl From<Val> for CornerRadius {
    fn from(x: Val) -> Self {
        Self { x, y: auto() }
    }
}

impl From<(Val, Val)> for CornerRadius {
    fn from((x, y): (Val, Val)) -> Self {
        Self { x, y }
    }
}

impl From<[Val; 2]> for CornerRadius {
    fn from([x, y]: [Val; 2]) -> Self {
        Self { x, y }
    }
}

impl PartialEq for CornerRadius {
    fn eq(&self, other: &Self) -> bool {
        // `Val::Auto` is used to indicate that the radius on an axis is unset and that the value on the other axis should be interpreted as a circular radius.
        // So  `CornerRadius { x: v, y: Val::AUTO } == CornerRadius { x: Val::Auto, y: v }`
        match (self, other) {
            (Self { x: r, y: Val::Auto }, Self { x: Val::Auto, y: s })
            | (Self { x: Val::Auto, y: r }, Self { x: s, y: Val::Auto }) => r == s,
            _ => self.x == other.x && self.y == other.y,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::geometry::*;
    use crate::val::Val;
    use bevy_math::vec2;
    use bevy_text::{EmSize, RemSize};

    #[test]
    fn corner_radius_resolve() {
        let size = vec2(100., 50.);
        let viewport_size = vec2(1000., 500.);
        let context = ValContext::new(1., viewport_size, EmSize(20.0), RemSize(20.0));

        assert_eq!(
            CornerRadius {
                x: Val::Px(100.),
                y: Val::Auto,
            }
            .resolve(&context, size),
            vec2(25., 25.)
        );
        assert_eq!(
            CornerRadius {
                x: Val::Px(40.),
                y: Val::Px(40.),
            }
            .resolve(&context, size),
            vec2(40., 25.)
        );
        assert_eq!(
            CornerRadius {
                x: Val::ZERO,
                y: Val::Px(20.),
            }
            .resolve(&context, size),
            vec2(0., 20.)
        );
        assert_eq!(
            CornerRadius::default().resolve(&context, size),
            vec2(0., 0.)
        );
    }

    #[test]
    fn uirect_default_equals_const_default() {
        assert_eq!(UiRect::default(), UiRect::all(Val::ZERO));
        assert_eq!(UiRect::default(), UiRect::DEFAULT);
    }

    #[test]
    fn test_uirect_axes() {
        let x = Val::Px(1.);
        let y = Val::Vw(4.);
        let r = UiRect::axes(x, y);
        let h = UiRect::horizontal(x);
        let v = UiRect::vertical(y);

        assert_eq!(r.top, v.top);
        assert_eq!(r.bottom, v.bottom);
        assert_eq!(r.left, h.left);
        assert_eq!(r.right, h.right);
    }

    #[test]
    fn uirect_px() {
        let r = UiRect::px(3., 5., 20., 999.);
        assert_eq!(r.left, Val::Px(3.));
        assert_eq!(r.right, Val::Px(5.));
        assert_eq!(r.top, Val::Px(20.));
        assert_eq!(r.bottom, Val::Px(999.));
    }

    #[test]
    fn uirect_percent() {
        let r = UiRect::percent(3., 5., 20., 99.);
        assert_eq!(r.left, Val::Percent(3.));
        assert_eq!(r.right, Val::Percent(5.));
        assert_eq!(r.top, Val::Percent(20.));
        assert_eq!(r.bottom, Val::Percent(99.));
    }

    #[test]
    fn val_constructor_fns_return_correct_val_variant() {
        assert_eq!(auto(), Val::Auto);
        assert_eq!(px(0.0), Val::Px(0.0));
        assert_eq!(percent(0.0), Val::Percent(0.0));
        assert_eq!(vw(0.0), Val::Vw(0.0));
        assert_eq!(vh(0.0), Val::Vh(0.0));
        assert_eq!(vmin(0.0), Val::VMin(0.0));
        assert_eq!(vmax(0.0), Val::VMax(0.0));
    }

    #[test]
    fn corner_radius_partial_eq_with_auto() {
        assert_eq!(
            CornerRadius::new(px(1.), Val::Auto),
            CornerRadius::new(Val::Auto, px(1.)),
        );
        assert_eq!(
            CornerRadius::new(Val::Auto, percent(1.)),
            CornerRadius::new(percent(1.), Val::Auto),
        );
    }
}
