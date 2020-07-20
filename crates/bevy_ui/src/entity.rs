use super::Node;
use crate::{
    render::UI_PIPELINE_HANDLE,
    widget::{Button, Text},
    Click, FocusPolicy, Hover,
};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_render::{
    draw::Draw,
    mesh::Mesh,
    pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline, RenderPipelines},
};
use bevy_sprite::{ColorMaterial, QUAD_HANDLE};
use bevy_transform::{components::LocalTransform, prelude::Transform};

#[derive(Bundle)]
pub struct NodeComponents {
    pub node: Node,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub local_transform: LocalTransform,
}

impl Default for NodeComponents {
    fn default() -> Self {
        NodeComponents {
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
            local_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct TextComponents {
    pub node: Node,
    pub draw: Draw,
    pub text: Text,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub local_transform: LocalTransform,
}

impl Default for TextComponents {
    fn default() -> Self {
        TextComponents {
            text: Text::default(),
            node: Default::default(),
            focus_policy: FocusPolicy::Pass,
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            local_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct ButtonComponents {
    pub node: Node,
    pub button: Button,
    pub click: Click,
    pub hover: Hover,
    pub focus_policy: FocusPolicy,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub local_transform: LocalTransform,
}

impl Default for ButtonComponents {
    fn default() -> Self {
        ButtonComponents {
            button: Button,
            click: Click::default(),
            hover: Hover::default(),
            focus_policy: FocusPolicy::default(),
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
            local_transform: Default::default(),
        }
    }
}
