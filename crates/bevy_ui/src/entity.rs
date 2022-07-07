//! This module contains the bundles used in Bevy's UI

use crate::{
    widget::{Button, ImageMode},
    CalculatedSize, FocusPolicy, Interaction, Node, Style, UiColor, UiImage, UI_CAMERA_FAR,
};
use bevy_ecs::{bundle::Bundle, prelude::Component};
use bevy_math::Vec2;
use bevy_render::{
    camera::{DepthCalculation, OrthographicProjection, WindowOrigin},
    view::Visibility,
};
use bevy_text::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};

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
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
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
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// The color of the node
    pub color: UiColor,
    /// The image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
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
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
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
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
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

/// Data related to the UI camera attached to this camera.
#[derive(Component, Clone, Debug)]
pub struct UiCamera {
    /// Toggle whether this camera should display UI
    pub show_ui: bool,
    /// The position of the UI camera in UI space.
    pub position: Vec2,
    pub(crate) projection: OrthographicProjection,
}
impl Default for UiCamera {
    fn default() -> Self {
        Self {
            show_ui: true,
            position: Vec2::ZERO,
            projection: OrthographicProjection {
                far: UI_CAMERA_FAR,
                window_origin: WindowOrigin::BottomLeft,
                depth_calculation: DepthCalculation::ZDifference,
                ..Default::default()
            },
        }
    }
}
impl UiCamera {
    /// The orthographic projection used by the UI camera.
    pub fn projection(&self) -> &OrthographicProjection {
        &self.projection
    }
    pub fn set_scale(&mut self, scale: f32) {
        // We can update the projection scale without running `projection.update(local_size)`
        // because update is not affected by scale, unless:
        // 1. window_origin = Center AND
        // 2. scaling_mode is WindowSize AND
        // 3. scale = 1.0
        // which is currently not possible, since projection is read-only
        // and its window_origin field never set to Center.
        self.projection.scale = scale;
    }
}
