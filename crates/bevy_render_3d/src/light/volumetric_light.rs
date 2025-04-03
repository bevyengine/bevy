use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

/// Add this component to a [`DirectionalLight`](crate::DirectionalLight) with a shadow map
/// (`shadows_enabled: true`) to make volumetric fog interact with it.
///
/// This allows the light to generate light shafts/god rays.
#[derive(Clone, Copy, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VolumetricLight;
