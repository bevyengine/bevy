use crate::{light::Light, material::StandardMaterial, render_graph::FORWARD_PIPELINE_HANDLE};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_render::{
    draw::Draw,
    mesh::Mesh,
    pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline, RenderPipelines},
    render_graph::base::MainPass,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// A component bundle for "pbr mesh" entities
#[derive(Bundle)]
pub struct PbrComponents {
    pub mesh: Handle<Mesh>,
    pub material: Handle<StandardMaterial>,
    pub main_pass: MainPass,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PbrComponents {
    fn default() -> Self {
        Self {
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                FORWARD_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 2,
                            binding: 0,
                        },
                        // StandardMaterial_albedo
                        DynamicBinding {
                            bind_group: 3,
                            binding: 0,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            mesh: Default::default(),
            material: Default::default(),
            main_pass: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

/// A component bundle for "light" entities
#[derive(Debug, Bundle, Default)]
pub struct LightComponents {
    pub light: Light,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
