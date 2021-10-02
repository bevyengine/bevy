use super::Node;
use crate::{
    render::UI_PIPELINE_HANDLE,
    widget::{Button, Image},
    CalculatedSize, ControlNode, FocusPolicy, Interaction, Style,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    camera::{Camera, DepthCalculation, OrthographicProjection, VisibleEntities, WindowOrigin},
    draw::Draw,
    mesh::Mesh,
    pipeline::{RenderPipeline, RenderPipelines},
    prelude::Visible,
};
use bevy_sprite::{ColorMaterial, QUAD_HANDLE};
use bevy_text::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};

/// If you add this to an entity, it should be the *only* bundle on it from bevy_ui.
/// This bundle will mark the entity as transparent to the UI layout system, meaning the
/// children of this entity will be treated as the children of this entity s parent by the layout system.
pub struct ControlBundle {
    pub control_node: ControlNode,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    pub node: Node,
    pub style: Style,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            node: Default::default(),
            style: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct ImageBundle {
    pub node: Node,
    pub style: Style,
    pub image: Image,
    pub calculated_size: CalculatedSize,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ImageBundle {
    fn default() -> Self {
        ImageBundle {
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            node: Default::default(),
            image: Default::default(),
            calculated_size: Default::default(),
            style: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct TextBundle {
    pub node: Node,
    pub style: Style,
    pub draw: Draw,
    pub visible: Visible,
    pub text: Text,
    pub calculated_size: CalculatedSize,
    pub focus_policy: FocusPolicy,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for TextBundle {
    fn default() -> Self {
        TextBundle {
            focus_policy: FocusPolicy::Pass,
            draw: Draw {
                ..Default::default()
            },
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            text: Default::default(),
            node: Default::default(),
            calculated_size: Default::default(),
            style: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    pub node: Node,
    pub button: Button,
    pub style: Style,
    pub interaction: Interaction,
    pub focus_policy: FocusPolicy,
    pub mesh: Handle<Mesh>, // TODO: maybe abstract this out
    pub material: Handle<ColorMaterial>,
    pub draw: Draw,
    pub visible: Visible,
    pub render_pipelines: RenderPipelines,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        ButtonBundle {
            button: Button,
            mesh: QUAD_HANDLE.typed(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                UI_PIPELINE_HANDLE.typed(),
            )]),
            interaction: Default::default(),
            focus_policy: Default::default(),
            node: Default::default(),
            style: Default::default(),
            material: Default::default(),
            draw: Default::default(),
            visible: Visible {
                is_transparent: true,
                ..Default::default()
            },
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Debug)]
pub struct UiCameraBundle {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub visible_entities: VisibleEntities,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for UiCameraBundle {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        UiCameraBundle {
            camera: Camera {
                name: Some(crate::camera::CAMERA_UI.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                window_origin: WindowOrigin::BottomLeft,
                depth_calculation: DepthCalculation::ZDifference,
                ..Default::default()
            },
            visible_entities: Default::default(),
            transform: Transform::from_xyz(0.0, 0.0, far - 0.1),
            global_transform: Default::default(),
        }
    }
}
