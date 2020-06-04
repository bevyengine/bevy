use crate::{light::Light, material::StandardMaterial};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{mesh::Mesh, Renderable};
use bevy_transform::prelude::{LocalToWorld, Rotation, Scale, Translation};

#[derive(EntityArchetype, Default)]
pub struct MeshEntity {
    // #[tag]
    pub mesh: Handle<Mesh>,
    // #[tag]
    pub material: Handle<StandardMaterial>,
    pub renderable: Renderable,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype, Default)]
pub struct LightEntity {
    pub light: Light,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
}
