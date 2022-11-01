//! This module contains the bundles used in Bevy's UI

use crate::{
    widget::{Button, ImageMode},
    BackgroundColor, CalculatedSize, FocusPolicy, Interaction, Node, Style, UiImage,
};
use bevy_ecs::{
    bundle::Bundle,
    prelude::{Component, With},
    query::QueryItem,
};
use bevy_render::{
    camera::Camera,
    extract_component::ExtractComponent,
    prelude::{Color, ComputedVisibility},
    view::Visibility,
};
use bevy_text::{Text, TextAlignment, TextSection, TextStyle};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The basic UI node
#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// The background color, which serves as a "fill" for this node
    pub background_color: BackgroundColor,
    /// Describes the image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `nodebundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            // Transparent background
            background_color: Color::NONE.into(),
            node: Default::default(),
            style: Default::default(),
            image: Default::default(),
            focus_policy: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
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
    /// The background color, which serves as a "fill" for this node
    ///
    /// When combined with `UiImage`, tints the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
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
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
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
            computed_visibility: Default::default(),
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
    /// The background color, which serves as a "fill" for this node
    ///
    /// When combined with `UiImage`, tints the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// The transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub transform: Transform,
    /// The global transform of the node
    ///
    /// This field is automatically managed by the UI layout system.
    /// To alter the position of the `NodeBundle`, use the properties of the [`Style`] component.
    pub global_transform: GlobalTransform,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

impl Default for ButtonBundle {
    fn default() -> Self {
        ButtonBundle {
            button: Button,
            interaction: Default::default(),
            focus_policy: Default::default(),
            node: Default::default(),
            style: Default::default(),
            background_color: Default::default(),
            image: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            computed_visibility: Default::default(),
        }
    }
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
