use bevy_ecs::component::Component;
use bevy_math::Affine3;

/// Transforms of the mesh
#[derive(Component)]
pub struct Mesh2dTransforms {
    /// World location of the mesh
    pub world_from_local: Affine3,
    /// Flags
    pub flags: u32,
}

#[derive(Component, Default)]
pub struct Mesh2dMarker;
