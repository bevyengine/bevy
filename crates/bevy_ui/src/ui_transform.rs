use crate::Val;
use bevy_derive::Deref;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_math::Affine2;
use bevy_math::Rot2;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;

/// A pair of [`Val`]s used to represent a 2-dimensional size or offset.
#[derive(Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct Val2 {
    /// Translate the node along the x-axis.
    /// `Val::Percent` values are resolved based on the computed width of the Ui Node.
    /// `Val::Auto` is resolved to `0.`.
    pub x: Val,
    /// Translate the node along the y-axis.
    /// `Val::Percent` values are resolved based on the computed height of the UI Node.
    /// `Val::Auto` is resolved to `0.`.
    pub y: Val,
}

impl Val2 {
    pub const ZERO: Self = Self {
        x: Val::ZERO,
        y: Val::ZERO,
    };

    /// Creates a new [`Val2`] where both components are in logical pixels
    pub const fn px(x: f32, y: f32) -> Self {
        Self {
            x: Val::Px(x),
            y: Val::Px(y),
        }
    }

    /// Creates a new [`Val2`] where both components are percentage values
    pub const fn percent(x: f32, y: f32) -> Self {
        Self {
            x: Val::Percent(x),
            y: Val::Percent(y),
        }
    }

    /// Creates a new [`Val2`]
    pub const fn new(x: Val, y: Val) -> Self {
        Self { x, y }
    }

    /// Resolves this [`Val2`] from the given `scale_factor`, `parent_size`,
    /// and `viewport_size`.
    ///
    /// Component values of [`Val::Auto`] are resolved to 0.
    pub fn resolve(&self, scale_factor: f32, base_size: Vec2, viewport_size: Vec2) -> Vec2 {
        Vec2::new(
            self.x
                .resolve(scale_factor, base_size.x, viewport_size)
                .unwrap_or(0.),
            self.y
                .resolve(scale_factor, base_size.y, viewport_size)
                .unwrap_or(0.),
        )
    }
}

impl Default for Val2 {
    fn default() -> Self {
        Self::ZERO
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
    pub fn from_rotation(rotation: Rot2) -> Self {
        Self {
            rotation,
            ..Self::IDENTITY
        }
    }

    /// Creates a UI transform representing a responsive translation.
    pub fn from_translation(translation: Val2) -> Self {
        Self {
            translation,
            ..Self::IDENTITY
        }
    }

    /// Creates a UI transform representing a scaling.
    pub fn from_scale(scale: Vec2) -> Self {
        Self {
            scale,
            ..Self::IDENTITY
        }
    }

    /// Resolves the translation from the given `scale_factor`, `base_value`, and `target_size`
    /// and returns a 2d affine transform from the resolved translation, and the `UiTransform`'s rotation, and scale.
    pub fn compute_affine(&self, scale_factor: f32, base_size: Vec2, target_size: Vec2) -> Affine2 {
        Affine2::from_scale_angle_translation(
            self.scale,
            self.rotation.as_radians(),
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
