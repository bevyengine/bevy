use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

/// Enables wireframe rendering for any entity it is attached to.
/// It will ignore the [`Wireframe2dConfig`](super::Wireframe2dConfig) global setting.
///
/// This requires the [`Wireframe2dPlugin`](super::Wireframe2dPlugin) to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct Wireframe2d;

/// Sets the color of the [`Wireframe2d`] of the entity it is attached to.
///
/// If this component is present but there's no [`Wireframe2d`] component,
/// it will still affect the color of the wireframe when
/// [`Wireframe2dConfig::global`](super::Wireframe2dConfig::global) is set to true.
///
/// This overrides the [`Wireframe2dConfig::default_color`](super::Wireframe2dConfig::default_color).
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Wireframe2dColor {
    /// Color of the lines of the wireframe
    pub color: Color,
}

/// Disables wireframe rendering for any entity it is attached to.
/// It will ignore the [`Wireframe2dConfig`](super::Wireframe2dConfig) global setting.
///
/// This requires the [`Wireframe2dPlugin`](super::Wireframe2dPlugin) to be enabled.
#[derive(Component, Debug, Clone, Default, Reflect, Eq, PartialEq)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
pub struct NoWireframe2d;
