use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{Reflect, std_traits::ReflectDefault};

/// Marker struct for labels
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Label;
