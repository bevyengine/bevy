use crate::{light::Light, material::StandardMaterial};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};
use bevy_transform::prelude::{Transform, Rotation, Scale, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshEntity {
    // #[tag]
    pub mesh: Handle<Mesh>,
    // #[tag]
    pub material: Handle<StandardMaterial>,
    pub renderable: Renderable,
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
