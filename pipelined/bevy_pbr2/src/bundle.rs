use crate::{PointLight, StandardMaterial};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::mesh::Mesh;
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone)]
pub struct PbrBundle {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PbrBundle {
    fn default() -> Self {
        Self {
            mesh: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

/// A component bundle for "light" entities
#[derive(Debug, Bundle, Default)]
pub struct PointLightBundle {
    pub point_light: PointLight,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
