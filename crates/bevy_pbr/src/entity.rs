use crate::{light::Light, material::StandardMaterial, render_graph::FORWARD_PIPELINE_HANDLE};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    pipeline::{RenderPipeline, RenderPipelines},
    prelude::MeshBundle,
};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// A component bundle for "pbr mesh" entities
#[derive(Bundle)]
pub struct PbrBundle {
    pub material: Handle<StandardMaterial>,
    #[bundle]
    pub mesh: MeshBundle,
}

impl Default for PbrBundle {
    fn default() -> Self {
        Self {
            mesh: MeshBundle {
                render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                    FORWARD_PIPELINE_HANDLE.typed(),
                )]),
                ..Default::default()
            },
            material: Default::default(),
        }
    }
}

/// A component bundle for "light" entities
#[derive(Debug, Bundle, Default)]
pub struct LightBundle {
    pub light: Light,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}
