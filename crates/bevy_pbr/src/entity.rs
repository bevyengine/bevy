use crate::{light::Light, material::StandardMaterial, pipelines::FORWARD_PIPELINE_HANDLE};
use bevy_asset::Handle;
use bevy_derive::EntityArchetype;
use bevy_render::{
    draw::Draw,
    mesh::Mesh,
    pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline, RenderPipelines},
};
use bevy_transform::prelude::{Rotation, Scale, Transform, Translation};

#[derive(EntityArchetype)]
pub struct MeshEntity {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for MeshEntity {
    fn default() -> Self {
        Self {
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                FORWARD_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 1,
                            binding: 0,
                        },
                        // StandardMaterial_albedo
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            mesh: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

#[derive(EntityArchetype, Default)]
pub struct LightEntity {
    pub light: Light,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
}
