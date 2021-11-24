use super::Node;
use crate::{
    widget::{Button, Image},
    CalculatedSize, FocusPolicy, Interaction, Style, CAMERA_UI,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::{
    camera::{Camera, DepthCalculation, OrthographicProjection, WindowOrigin},
    color::Color,
    view::VisibleEntities,
};
use bevy_text2::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};

#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    pub node: Node,
    pub style: Style,
    pub color: Color,
    pub texture: Option<Handle<bevy_render2::texture::Image>>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            node: Default::default(),
            style: Default::default(),
            color: Default::default(),
            texture: Default::default(),
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
    pub color: Color,
    pub texture: Option<Handle<bevy_render2::texture::Image>>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ImageBundle {
    fn default() -> Self {
        ImageBundle {
            node: Default::default(),
            image: Default::default(),
            calculated_size: Default::default(),
            style: Default::default(),
            color: Default::default(),
            texture: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone, Debug)]
pub struct TextBundle {
    pub node: Node,
    pub style: Style,
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
    pub color: Color,
    pub texture: Option<Handle<bevy_render2::texture::Image>>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        ButtonBundle {
            button: Button,
            interaction: Default::default(),
            focus_policy: Default::default(),
            node: Default::default(),
            style: Default::default(),
            color: Default::default(),
            texture: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Debug)]
pub struct UiCameraBundle {
    pub camera: Camera,
    pub orthographic_projection: OrthographicProjection,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    // FIXME there is no frustrum culling for UI
    pub visible_entities: VisibleEntities,
}

impl Default for UiCameraBundle {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        UiCameraBundle {
            camera: Camera {
                name: Some(CAMERA_UI.to_string()),
                ..Default::default()
            },
            orthographic_projection: OrthographicProjection {
                far,
                window_origin: WindowOrigin::BottomLeft,
                depth_calculation: DepthCalculation::ZDifference,
                ..Default::default()
            },
            transform: Transform::from_xyz(0.0, 0.0, far - 0.1),
            global_transform: Default::default(),
            visible_entities: Default::default(),
        }
    }
}
