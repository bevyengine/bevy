use crate::Val;
use crate::ValArithmeticError;
use crate::ValNum;
use bevy_derive::Deref;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_math::Affine2;
use bevy_math::Mat2;
use bevy_math::Rot2;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;
use core::fmt;
use core::ops::Mul;

/// A pair of [`Val`]s used to represent a 2-dimensional size or offset.
///
/// - `Val::Percent` x/y values are resolved based on the computed length of the Ui Node on the respective axis.
/// - `Val::Auto` is resolved to `0.`.
#[derive(Clone, Copy, Reflect, PartialEq)]
#[reflect(Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Val2 {
    values: [f32; 2],
    units: u8,
}

impl Val2 {
    pub const ZERO: Self = Self {
        values: [0.; 2],
        units: Val::PX | (Val::PX << 4),
    };

    /// Creates a new [`Val2`]
    pub const fn new(x: Val, y: Val) -> Self {
        let (ux, vx) = x.pack();
        let (uy, vy) = y.pack();
        Self {
            values: [vx, vy],
            units: ux | uy << 4,
        }
    }

    /// Creates a new [`Val2`] where both components are the same value
    pub const fn all(val: Val) -> Self {
        Self::new(val, val)
    }

    /// Creates a new [`Val2`] where both components are in logical pixels
    pub fn px<X: ValNum, Y: ValNum>(x: X, y: Y) -> Self {
        Self::new(Val::Px(x.val_num_f32()), Val::Px(y.val_num_f32()))
    }

    /// Creates a new [`Val2`] where both components are percentage values
    pub fn percent<X: ValNum, Y: ValNum>(x: X, y: Y) -> Self {
        Self::new(Val::Percent(x.val_num_f32()), Val::Percent(y.val_num_f32()))
    }

    /// Returns the x-axis value.
    #[inline]
    pub const fn x(&self) -> Val {
        Val::unpack(self.units & 0x0f, self.values[0])
    }

    /// Returns the y-axis value.
    #[inline]
    pub const fn y(&self) -> Val {
        Val::unpack(self.units >> 4, self.values[1])
    }

    /// Set a new x value.
    ///
    /// ```
    /// # use bevy_ui::{px, percent, Val2};
    /// let mut val = Val2::new(percent(50), px(20));
    /// val.set_x(px(10));
    ///
    /// assert_eq!(val.x(), px(10));
    /// assert_eq!(val.y(), px(20));
    /// ```
    #[inline]
    pub const fn set_x(&mut self, x: Val) {
        let (unit, value) = x.pack();
        self.values[0] = value;
        self.units = (self.units & 0xf0) | unit;
    }

    /// Set a new y value.
    ///
    /// ```
    /// # use bevy_ui::{px, percent, Val2};
    /// let mut val = Val2::new(px(10), percent(50));
    /// val.set_y(px(20));
    ///
    /// assert_eq!(val.x(), px(10));
    /// assert_eq!(val.y(), px(20));
    /// ```
    #[inline]
    pub const fn set_y(&mut self, y: Val) {
        let (unit, value) = y.pack();
        self.values[1] = value;
        self.units = (self.units & 0x0f) | (unit << 4);
    }

    /// Resolves this [`Val2`] from the given `scale_factor`, `parent_size`,
    /// and `viewport_size`.
    ///
    /// Component values of [`Val::Auto`] are resolved to 0.
    pub fn resolve(&self, scale_factor: f32, base_size: Vec2, viewport_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x()
                .resolve(scale_factor, base_size.x, viewport_size)
                .unwrap_or(0.),
            self.y()
                .resolve(scale_factor, base_size.y, viewport_size)
                .unwrap_or(0.),
        )
    }

    /// Try to add two `Val2`s component-wise.
    ///
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if either component has mismatched units.
    ///
    /// ```
    /// # use bevy_ui::{Val, Val2, ValArithmeticError};
    /// assert_eq!(Val2::px(1., 2.).try_add(Val2::px(3., 4.)), Ok(Val2::px(4., 6.)));
    /// assert_eq!(
    ///     Val2::new(Val::Px(1.), Val::Px(2.)).try_add(Val2::new(Val::Percent(3.), Val::Px(4.))),
    ///     Err(ValArithmeticError::IncompatibleUnits)
    /// );
    /// ```
    pub fn try_add(self, other: Val2) -> Result<Self, ValArithmeticError> {
        let (Ok(x), Ok(y)) = (self.x().try_add(other.x()), self.y().try_add(other.y())) else {
            return Err(ValArithmeticError::IncompatibleUnits);
        };
        Ok(Self::new(x, y))
    }

    /// Try to subtract two `Val2`s component-wise.
    ///
    /// Returns [`ValArithmeticError::IncompatibleUnits`] if either component has mismatched units.
    ///
    /// ```
    /// # use bevy_ui::{Val, Val2, ValArithmeticError};
    /// assert_eq!(Val2::px(3., 4.).try_sub(Val2::px(1., 2.)), Ok(Val2::px(2., 2.)));
    /// assert_eq!(
    ///     Val2::new(Val::Px(1.), Val::Px(2.)).try_sub(Val2::new(Val::Percent(3.), Val::Px(4.))),
    ///     Err(ValArithmeticError::IncompatibleUnits)
    /// );
    /// ```
    pub fn try_sub(self, other: Val2) -> Result<Self, ValArithmeticError> {
        let (Ok(x), Ok(y)) = (self.x().try_sub(other.x()), self.y().try_sub(other.y())) else {
            return Err(ValArithmeticError::IncompatibleUnits);
        };
        Ok(Self::new(x, y))
    }
}

impl fmt::Debug for Val2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Val2")
            .field("x", &self.x())
            .field("y", &self.y())
            .finish()
    }
}

impl Default for Val2 {
    fn default() -> Self {
        Self::ZERO
    }
}

impl From<Val> for Val2 {
    fn from(val: Val) -> Val2 {
        Val2::all(val)
    }
}

/// Relative 2D transform for UI nodes
///
/// [`UiGlobalTransform`] is automatically inserted whenever [`UiTransform`] is inserted.
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[require(UiGlobalTransform)]
pub struct UiTransform {
    /// Translate the node.
    pub translation: Val2,
    /// Scale the node. A negative value reflects the node in that axis.
    pub scale: Vec2,
    /// Rotate the node clockwise.
    pub rotation: Rot2,
}

impl UiTransform {
    pub const IDENTITY: Self = Self {
        translation: Val2::ZERO,
        scale: Vec2::ONE,
        rotation: Rot2::IDENTITY,
    };

    /// Creates a UI transform representing a rotation.
    pub const fn from_rotation(rotation: Rot2) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// Creates a UI transform representing a responsive translation.
    pub const fn from_translation(translation: Val2) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Creates a UI transform representing a scaling.
    pub const fn from_scale(scale: Vec2) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Create a new UI transform at the position `(x, y)`
    pub const fn from_xy(x: Val, y: Val) -> Self {
        Self {
            translation: Val2::new(x, y),
            ..Self::IDENTITY
        }
    }

    /// Resolves the translation from the given `scale_factor`, `base_value`, and `target_size`
    /// and returns a 2d affine transform from the resolved translation, and the `UiTransform`'s rotation, and scale.
    pub fn compute_affine(&self, scale_factor: f32, base_size: Vec2, target_size: Vec2) -> Affine2 {
        Affine2::from_mat2_translation(
            Mat2::from(self.rotation) * Mat2::from_diagonal(self.scale),
            self.translation
                .resolve(scale_factor, base_size, target_size),
        )
    }
}

impl Default for UiTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Absolute 2D transform for UI nodes
///
/// [`UiGlobalTransform`]s are updated from [`UiTransform`] and [`Node`](crate::ui_node::Node)
///  in [`ui_layout_system`](crate::layout::ui_layout_system)
#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, Deref)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct UiGlobalTransform(Affine2);

impl Default for UiGlobalTransform {
    fn default() -> Self {
        Self(Affine2::IDENTITY)
    }
}

impl UiGlobalTransform {
    /// If the transform is invertible returns its inverse.
    /// Otherwise returns `None`.
    #[inline]
    pub fn try_inverse(&self) -> Option<Affine2> {
        (self.matrix2.determinant() != 0.).then_some(self.inverse())
    }

    /// Creates a `UiGlobalTransform` from the given 2D translation.
    #[inline]
    pub fn from_translation(translation: Vec2) -> Self {
        Self(Affine2::from_translation(translation))
    }

    /// Creates a `UiGlobalTransform` from the given 2D translation.
    #[inline]
    pub fn from_xy(x: f32, y: f32) -> Self {
        Self::from_translation(Vec2::new(x, y))
    }

    /// Creates a `UiGlobalTransform` from the given rotation.
    #[inline]
    pub fn from_rotation(rotation: Rot2) -> Self {
        Self(Affine2::from_mat2(rotation.into()))
    }

    /// Creates a `UiGlobalTransform` from the given scaling.
    #[inline]
    pub fn from_scale(scale: Vec2) -> Self {
        Self(Affine2::from_scale(scale))
    }

    /// Extracts scale, angle and translation from self.
    /// The transform is expected to be non-degenerate and without shearing, or the output will be invalid.
    #[inline]
    pub fn to_scale_angle_translation(&self) -> (Vec2, f32, Vec2) {
        self.0.to_scale_angle_translation()
    }

    /// Returns the transform as an [`Affine2`]
    #[inline]
    pub fn affine(&self) -> Affine2 {
        self.0
    }
}

impl From<Affine2> for UiGlobalTransform {
    fn from(value: Affine2) -> Self {
        Self(value)
    }
}

impl From<UiGlobalTransform> for Affine2 {
    fn from(value: UiGlobalTransform) -> Self {
        value.0
    }
}

impl From<&UiGlobalTransform> for Affine2 {
    fn from(value: &UiGlobalTransform) -> Self {
        value.0
    }
}

impl Mul for UiGlobalTransform {
    type Output = Self;

    #[inline]
    fn mul(self, value: Self) -> Self::Output {
        Self(self.0 * value.0)
    }
}

impl Mul<Affine2> for UiGlobalTransform {
    type Output = Affine2;

    #[inline]
    fn mul(self, affine2: Affine2) -> Self::Output {
        self.0 * affine2
    }
}

impl Mul<UiGlobalTransform> for Affine2 {
    type Output = Affine2;

    #[inline]
    fn mul(self, transform: UiGlobalTransform) -> Self::Output {
        self * transform.0
    }
}

impl Mul<Vec2> for UiGlobalTransform {
    type Output = Vec2;

    #[inline]
    fn mul(self, value: Vec2) -> Vec2 {
        self.transform_point2(value)
    }
}
