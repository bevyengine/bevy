use crate::ReflectResource;
use bevy_ecs::system::Resource;
use bevy_reflect::Reflect;
use bevy_render::{color::Color, extract_resource::ExtractResource};

#[derive(Debug, Clone, Default, ExtractResource, Resource, Reflect)]
#[reflect(Resource)]
pub struct Fog {
    pub color: Color,
    pub mode: FogMode,
}

#[derive(Debug, Clone, Default, Reflect)]
pub enum FogMode {
    #[default]
    Off,
    Linear {
        start: f32,
        end: f32,
    },
    Exponential {
        density: f32,
    },
    ExponentialSquared {
        density: f32,
    },
}
