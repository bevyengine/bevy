//! This module contains the bundles used in Bevy's UI

use crate::{
    widget::{Button, ImageMode},
    CalculatedSize, Hover, Node, Style, UiColor, UiImage,
};
use bevy_ecs::{
    bundle::Bundle,
    prelude::{Component, With},
    query::QueryItem,
};
use bevy_render::{
    camera::Camera, extract_component::ExtractComponent, prelude::ComputedVisibility,
    view::Visibility,
};
use bevy_text::{Text, TextAlignment, TextSection, TextStyle};
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui_navigation::prelude::{Focusable, MenuBuilder, MenuSetting};

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
    /// The transform of the node
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

/// A menu. All [`Focusable`] transitive children of this entity will
/// be part of this menu.
#[derive(Bundle)]
pub struct MenuBundle {
    #[bundle]
    pub node_bundle: NodeBundle,
    /// From which [`Focusable`] is this menu accessible (if any).
    pub menu: MenuBuilder,
    /// How this menu should behave.
    pub setting: MenuSetting,
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
    /// The transform of the node
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

/// A UI node that is text
#[derive(Bundle, Clone, Debug, Default)]
pub struct TextBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// The transform of the node
    pub transform: Transform,
    /// The global transform of the node
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl TextBundle {
    /// Create a [`TextBundle`] from a single section.
    ///
    /// See [`Text::from_section`] for usage.
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            text: Text::from_section(value, style),
            ..Default::default()
        }
    }

    /// Create a [`TextBundle`] from a list of sections.
    ///
    /// See [`Text::from_sections`] for usage.
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            text: Text::from_sections(sections),
            ..Default::default()
        }
    }

    /// Returns this [`TextBundle`] with a new [`TextAlignment`] on [`Text`].
    pub const fn with_text_alignment(mut self, alignment: TextAlignment) -> Self {
        self.text.alignment = alignment;
        self
    }

    /// Returns this [`TextBundle`] with a new [`Style`].
    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug, Default)]
pub struct ButtonBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// How this button fits with other ones
    pub focusable: Focusable,
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
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Handle hovering state of the button
    pub hover: Hover,
}

/// Configuration for cameras related to UI.
///
/// When a [`Camera`] doesn't have the [`UiCameraConfig`] component,
/// it will display the UI by default.
///
/// [`Camera`]: bevy_render::camera::Camera
#[derive(Component, Clone)]
pub struct UiCameraConfig {
    /// Whether to output UI to this camera view.
    ///
    /// When a `Camera` doesn't have the [`UiCameraConfig`] component,
    /// it will display the UI by default.
    pub show_ui: bool,
}

impl Default for UiCameraConfig {
    fn default() -> Self {
        Self { show_ui: true }
    }
}

impl ExtractComponent for UiCameraConfig {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}
