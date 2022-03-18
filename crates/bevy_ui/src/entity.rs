//! This module contains the bundles used in Bevy's UI

use crate::{
    widget::{Button, ImageMode},
    FocusPolicy, Interaction, Node, Style, UiColor, UiImage,
};
use bevy_ecs::{bundle::Bundle, prelude::Component};
use bevy_transform::prelude::{Transform, GlobalTransform};
use bevy_render::{
    camera::{Camera, DepthCalculation, OrthographicProjection, WindowOrigin},
    view::{Visibility, VisibleEntities},
};
use bevy_text::Text;
use bevy_math::{Vec2, Size};

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct RectTransform {
    pub position: Vec2,
    pub size: Size<f32>,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct GlobalRectTransform {
    pub position: Vec2,
    pub size: Size<f32>,
    pub depth: u32,
}

impl GlobalRectTransform {
    pub fn canvas_depth(&self) -> u16 {
        (self.depth >> 16) as u16
    }

    pub fn entity_depth(&self) -> u16 {
        (self.depth & 0xFFFF) as u16
    }

    pub fn inherit(&mut self, parent: &Self, source: RectTransform) {
        self.position = parent.position + source.position;
        self.size = source.size;
        self.depth = parent.depth + 1;
    }
}

/// The basic UI node
#[derive(Bundle, Clone, Debug, Default)]
pub struct NodeBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Describes the color of the node
    pub color: UiColor,
    /// Describes the image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    pub transform: RectTransform,
    /// The global transform of the node
    pub global_transform: GlobalRectTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
}

/// A UI node that is an image
#[derive(Bundle, Clone, Debug, Default)]
pub struct ImageBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Configures how the image should scale
    pub image_mode: ImageMode,
    /// The color of the node
    pub color: UiColor,
    /// The image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    pub transform: RectTransform,
    /// The global transform of the node
    pub global_transform: GlobalRectTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
}

/// A UI node that is text
#[derive(Bundle, Clone, Debug)]
pub struct TextBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    pub transform: RectTransform,
    /// The global transform of the node
    pub global_transform: GlobalRectTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
}

impl Default for TextBundle {
    fn default() -> Self {
        TextBundle {
            focus_policy: FocusPolicy::Pass,
            text: Default::default(),
            node: Default::default(),
            style: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
        }
    }
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Describes whether and how the button has been interacted with by the input
    pub interaction: Interaction,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The color of the node
    pub color: UiColor,
    /// The image of the node
    pub image: UiImage,
    /// The transform of the node
    pub transform: RectTransform,
    /// The global transform of the node
    pub global_transform: GlobalRectTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
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
            image: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
        }
    }
}
#[derive(Component, Default)]
pub struct CameraUi;

/// The camera that is needed to see UI elements
#[derive(Bundle, Debug)]
pub struct UiCameraBundle<M: Component> {
    /// The camera component
    pub camera: Camera,
    /// The orthographic projection settings
    pub orthographic_projection: OrthographicProjection,
    /// The transform of the camera
    pub transform: Transform,
    /// The global transform of the camera
    pub global_transform: GlobalTransform,
    /// Contains visible entities
    // FIXME there is no frustrum culling for UI
    pub visible_entities: VisibleEntities,
    pub marker: M,
}

impl Default for UiCameraBundle<CameraUi> {
    fn default() -> Self {
        // we want 0 to be "closest" and +far to be "farthest" in 2d, so we offset
        // the camera's translation by far and use a right handed coordinate system
        let far = 1000.0;
        UiCameraBundle {
            camera: Camera {
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
            marker: CameraUi,
        }
    }
}
