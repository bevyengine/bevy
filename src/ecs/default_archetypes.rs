use crate::prelude::*;
use crate::render::render_graph_2::{StandardMaterial, ShaderUniforms};
use bevy_derive::EntityArchetype;

#[derive(EntityArchetype)]
pub struct MeshEntity {
    pub mesh: Handle<Mesh>,
    pub material: Material,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
}

#[derive(EntityArchetype)]
pub struct NewMeshEntity {
    pub mesh: Handle<Mesh>,
    pub material: StandardMaterial,
    pub shader_uniforms: ShaderUniforms,
    pub local_to_world: LocalToWorld,
    pub translation: Translation,
}

#[derive(EntityArchetype)]
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