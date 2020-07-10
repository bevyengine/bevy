use super::Node;
use crate::{render::UI_PIPELINE_HANDLE, widget::Label};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_render::{
    draw::Draw,
    mesh::Mesh,
    pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline, RenderPipelines},
};
use bevy_sprite::{ColorMaterial, QUAD_HANDLE};
use bevy_transform::prelude::{Rotation, Scale, Transform, Translation};

#[derive(Bundle)]
pub struct UiComponents {
    pub node: Node,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for UiComponents {
    fn default() -> Self {
        UiComponents {
            mesh: QUAD_HANDLE,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                UI_PIPELINE_HANDLE,
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 1,
                            binding: 0,
                        },
                        // Node_size
                        DynamicBinding {
                            bind_group: 1,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            node: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct LabelComponents {
    pub node: Node,
    pub draw: Draw,
    pub label: Label,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for LabelComponents {
    fn default() -> Self {
        LabelComponents {
            label: Label::default(),
            node: Default::default(),
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            translation: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}
