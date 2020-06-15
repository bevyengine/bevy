use crate::{light::Light, material::StandardMaterial};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{draw::Draw, mesh::Mesh, pipeline::RenderPipelines};
use bevy_transform::prelude::{Rotation, Scale, Transform, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshEntity {
    // #[tag]
    pub mesh: Handle<Mesh>,
    // #[tag]
    pub material: Handle<StandardMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype, Default)]
pub struct LightEntity {
    pub light: Light,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
}
