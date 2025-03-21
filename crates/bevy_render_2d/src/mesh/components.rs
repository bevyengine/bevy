use bevy_ecs::component::Component;
use bevy_math::Affine3;

#[derive(Component)]
pub struct Mesh2dTransforms {
    pub world_from_local: Affine3,
    pub flags: u32,
}

#[derive(Component, Default)]
pub struct Mesh2dMarker;
