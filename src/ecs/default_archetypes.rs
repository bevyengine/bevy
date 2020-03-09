use crate::{
    prelude::*,
    render::render_graph::{Renderable, StandardMaterial},
};

use crate as bevy; // for macro imports
use bevy_derive::EntityArchetype;

#[derive(EntityArchetype, Default)]
pub struct MeshEntity {
    pub mesh: Handle<Mesh>,
    pub material: StandardMaterial,
    pub renderable: Renderable,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

#[derive(EntityArchetype, Default)]
pub struct MeshMaterialEntity<T: Default + Send + Sync + 'static> {
    pub mesh: Handle<Mesh>,
    pub material: T,
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

#[derive(EntityArchetype)]
pub struct CameraEntity {
    pub camera: Camera,
    pub active_camera: ActiveCamera,
    pub local_to_world: LocalToWorld,
}

#[derive(EntityArchetype)]
pub struct Camera2dEntity {
    pub camera: Camera,
    pub active_camera_2d: ActiveCamera2d,
}

#[derive(EntityArchetype)]
pub struct UiEntity {
    pub node: Node,
}
