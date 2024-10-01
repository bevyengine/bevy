use bevy_asset::Handle;
use bevy_ecs::component::Component;

#[derive(Component)]
pub struct SimplifiedMesh {
    pub mesh: Handle<bevy_render::mesh::Mesh>,
}

#[derive(Component)]
pub struct NoBackfaceCulling;
