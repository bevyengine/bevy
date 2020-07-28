use super::Node;
use crate::{
    render::UI_PIPELINE_HANDLE,
    widget::{Button, Text, Image},
    FocusPolicy, Interaction, Style, CalculatedSize,
};
use bevy_asset::Handle;
use bevy_ecs::Bundle;
use bevy_render::{
    camera::{Camera, OrthographicProjection, VisibleEntities, WindowOrigin},
    draw::Draw,
    mesh::Mesh,
    pipeline::{DynamicBinding, PipelineSpecialization, RenderPipeline, RenderPipelines},
};
use bevy_sprite::{ColorMaterial, QUAD_HANDLE};
use bevy_transform::{
    components::LocalTransform,
    prelude::{Rotation, Scale, Transform, Translation},
};

#[derive(Bundle)]
pub struct NodeComponents {
    pub node: Node,
    pub style: Style,
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
            style: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            local_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct ImageComponents {
    pub node: Node,
    pub style: Style,
    pub image: Image,
    pub calculated_size: CalculatedSize,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub local_transform: LocalTransform,
}

impl Default for ImageComponents {
    fn default() -> Self {
        ImageComponents {
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
            image: Default::default(),
            calculated_size: Default::default(),
            style: Default::default(),
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
    pub style: Style,
    pub draw: Draw,
    pub text: Text,
    pub calculated_size: CalculatedSize,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub local_transform: LocalTransform,
}

impl Default for TextComponents {
    fn default() -> Self {
        TextComponents {
            focus_policy: FocusPolicy::Pass,
            draw: Draw {
                is_transparent: true,
                ..Default::default()
            },
            text: Default::default(),
            node: Default::default(),
            calculated_size: Default::default(),
            style: Default::default(),
            transform: Default::default(),
            local_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct ButtonComponents {
    pub node: Node,
    pub button: Button,
    pub style: Style,
    pub interaction: Interaction,
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
            interaction: Default::default(),
            focus_policy: Default::default(),
            node: Default::default(),
            style: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            local_transform: Default::default(),
        }
    }
}

#[derive(Bundle)]
pub struct UiCameraComponents {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub translation: Translation,
    pub rotation: Rotation,
    pub scale: Scale,
}

impl Default for UiCameraComponents {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        UiCameraComponents {
            camera: Camera {
                name: Some(crate::camera::UI_CAMERA.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                window_origin: WindowOrigin::BottomLeft,
                ..Default::default()
            },
            translation: Translation::new(0.0, 0.0, far - 0.1),
            visible_entities: Default::default(),
            transform: Default::default(),
            rotation: Default::default(),
            scale: Default::default(),
        }
    }
}
