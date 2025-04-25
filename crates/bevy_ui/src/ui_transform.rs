use crate::Val;
use bevy_derive::Deref;
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_math::Affine2;
use bevy_math::Vec2;
use bevy_reflect::prelude::*;

#[derive(Debug, PartialEq, Clone, Copy, Reflect)]
pub struct UiVec {
    /// Translate the node along the x-axis.
    /// `Val::Percent` values are resolved based on the computed width of the Ui Node.
    /// `Val::Auto` is resolved to `0.`.
    pub x: Val,
    /// Translate the node along the y-axis.
    /// `Val::Percent` values are resolved based on the computed width of the Ui Node.
    /// `Val::Auto` is resolved to `0.`.
    pub y: Val,
}

impl UiVec {
    pub const ZERO: Self = Self {
        x: Val::ZERO,
        y: Val::ZERO,
    };

    pub const fn px(x: f32, y: f32) -> Self {
        Self {
            x: Val::Px(x),
            y: Val::Px(y),
        }
    }

    pub const fn percent(x: f32, y: f32) -> Self {
        Self {
            x: Val::Percent(x),
            y: Val::Percent(y),
        }
    }

    pub const fn new(x: Val, y: Val) -> Self {
        Self { x, y }
    }
}

impl Default for UiVec {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct UiTransform {
    /// Translate the node.
    pub translation: UiVec,
    /// Scale the node. A negative value reflects the node in that axis.
    pub scale: Vec2,
    /// Rotate the node clockwise by the given value in radians.
    pub rotation: f32,
}

impl UiTransform {
    pub const IDENTITY: Self = Self {
        translation: UiVec::ZERO,
        scale: Vec2::ONE,
        rotation: 0.,
    };
}

impl Default for UiTransform {
    fn default() -> Self {
        Self::IDENTITY
    }
}

#[derive(Component, Debug, PartialEq, Clone, Copy, Reflect, Deref)]
#[reflect(Component, Default, PartialEq, Debug, Clone)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct UiGlobalTransform(pub Affine2);

impl Default for UiGlobalTransform {
    fn default() -> Self {
        Self(Affine2::IDENTITY)
    }
}
