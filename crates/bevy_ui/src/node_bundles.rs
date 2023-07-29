//! This module contains basic node bundles used to build UIs

#[cfg(feature = "bevy_text")]
use crate::widget::TextFlags;
use crate::{
    widget::{Button, UiImageSize},
    BackgroundColor, BorderColor, ContentSize, FocusPolicy, Interaction, Node, Style, UiImage,
    UiTextureAtlasImage, ZIndex,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    prelude::{Color, ComputedVisibility},
    view::Visibility,
};
use bevy_sprite::TextureAtlas;
#[cfg(feature = "bevy_text")]
use bevy_text::{BreakLineOn, Text, TextAlignment, TextLayoutInfo, TextSection, TextStyle};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// The basic UI node
///
/// Useful as a container for a variety of child nodes.
#[derive(Bundle, Clone, Debug)]
pub struct NodeBundle {
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// The background color, which serves as a "fill" for this node
    pub background_color: BackgroundColor,
    /// The color of the Node's border
    pub border_color: BorderColor,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// Components managed automatically by bevy's internal systems
    /// Any manual modification or initialization to non-default values of these components should be avoided, as it can lead to unpredictable behaviour.
    /// 
    /// * [`Node`]: Describes the logical size of the node
    /// * [`Transform`]: The transform of the node
    /// * [`GlobalTransform`]: The global transform of the node
    /// * [`ComputedVisibility`]: Algorithmically-computed indication of whether an entity is visible.
    pub internal_components: (Node, Transform, GlobalTransform, ComputedVisibility),
}

impl Default for NodeBundle {
    fn default() -> Self {
        NodeBundle {
            // Transparent background
            background_color: Color::NONE.into(),
            border_color: Color::NONE.into(),
            style: Default::default(),
            focus_policy: Default::default(),
            visibility: Default::default(),
            z_index: Default::default(),
            internal_components: Default::default(),
        }
    }
}

/// A UI node that is an image
#[derive(Bundle, Debug, Default)]
pub struct ImageBundle {
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// The background color, which serves as a "fill" for this node
    ///
    /// Combines with `UiImage` to tint the provided image.
    pub background_color: BackgroundColor,
    /// The image of the node
    pub image: UiImage,
    /// The size of the image in pixels
    ///
    /// This field is set automatically
    pub image_size: UiImageSize,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// Components managed automatically by bevy's internal systems.
    /// Any manual modification or initialization to non-default values of these components should be avoided, as it can lead to unpredictable behaviour.
    /// 
    /// * [`Node`]: Describes the logical size of the node
    /// * [`Transform`]: The transform of the node
    /// * [`GlobalTransform`]: The global transform of the node
    /// * [`ComputedVisibility`]: Algorithmically-computed indication of whether an entity is visible.
    /// * [`UiImageSize`]: The size of the image in pixels
    /// * [`ContentSize`]: Used by the layout algorithm to compute the space required for the node's content
    pub internal_components: (
        Node, 
        Transform,
        GlobalTransform,
        ComputedVisibility,
        UiImageSize,
        ContentSize,
    ),
}

/// A UI node that is a texture atlas sprite
#[derive(Bundle, Debug, Default)]
pub struct AtlasImageBundle {
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// The background color, which serves as a "fill" for this node
    ///
    /// Combines with `UiImage` to tint the provided image.
    pub background_color: BackgroundColor,
    /// A handle to the texture atlas to use for this Ui Node
    pub texture_atlas: Handle<TextureAtlas>,
    /// The descriptor for which sprite to use from the given texture atlas
    pub texture_atlas_image: UiTextureAtlasImage,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// Components managed automatically by bevy's internal systems.
    /// Any manual modification or initialization to non-default values of these components should be avoided, as it can lead to unpredictable behaviour.
    ///
    /// * [`Node`]: Describes the logical size of the node
    /// * [`Transform`]: The transform of the node
    /// * [`GlobalTransform`]: The global transform of the node
    /// * [`ComputedVisibility`]: Algorithmically-computed indication of whether an entity is visible.
    /// * [`UiImageSize`]: The size of the image in pixels
    /// * [`ContentSize`]: Used by the layout algorithm to compute the space required for the node's content
    pub internal_components: (
        Transform,
        GlobalTransform,
        ComputedVisibility,
        UiImageSize,
        ContentSize,
    ),
}

#[cfg(feature = "bevy_text")]
/// A UI node that is text
#[derive(Bundle, Debug)]
pub struct TextBundle {
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// Contains the text of the node
    pub text: Text,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// The background color that will fill the containing node
    pub background_color: BackgroundColor,
    /// Components managed automatically by bevy's internal systems.
    /// Any manual modification or initialization to non-default values of these components should be avoided, as it can lead to unpredictable behaviour.
    /// 
    /// * [`Node`]: Describes the logical size of the node
    /// * [`Transform`]: The transform of the node
    /// * [`GlobalTransform`]: The global transform of the node
    /// * [`ComputedVisibility`]: Algorithmically-computed indication of whether an entity is visible.
    /// * [`ContentSize`]: Used by the layout algorithm to compute the space required for the node's content
    /// * [`TextLayoutInfo`]: The list of glyphs to be rendered, along with their positions and the size of the text block
    /// * [`TextFlags`]: Text system flags
    pub internal_components: (
        Node,
        Transform,
        GlobalTransform,
        ComputedVisibility,
        ContentSize,
        TextLayoutInfo,
        TextFlags,
    ),
}

#[cfg(feature = "bevy_text")]
impl Default for TextBundle {
    fn default() -> Self {
        Self {
            text: Default::default(),
            // Transparent background
            background_color: BackgroundColor(Color::NONE),
            style: Default::default(),
            focus_policy: Default::default(),
            visibility: Default::default(),
            z_index: Default::default(),
            internal_components: Default::default(),
        }
    }
}

#[cfg(feature = "bevy_text")]
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
    pub fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    /// Returns this [`TextBundle`] with a new [`BackgroundColor`].
    pub const fn with_background_color(mut self, color: Color) -> Self {
        self.background_color = BackgroundColor(color);
        self
    }

    /// Returns this [`TextBundle`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub const fn with_no_wrap(mut self) -> Self {
        self.text.linebreak_behavior = BreakLineOn::NoWrap;
        self
    }
}

/// A UI node that is a button
#[derive(Bundle, Clone, Debug)]
pub struct ButtonBundle {
    /// Marker component that signals this node is a button
    pub button: Button,
    /// Styles which control the layout (size and position) of the node and it's children
    /// In some cases these styles also affect how the node drawn/painted.
    pub style: Style,
    /// Describes whether and how the button has been interacted with by the input
    pub interaction: Interaction,
    /// Whether this node should block interaction with lower nodes
    pub focus_policy: FocusPolicy,
    /// The background color, which serves as a "fill" for this node
    ///
    /// When combined with `UiImage`, tints the provided image.
    pub background_color: BackgroundColor,
    /// The color of the Node's border
    pub border_color: BorderColor,
    /// The image of the node
    pub image: UiImage,
    /// Describes the visibility properties of the node
    pub visibility: Visibility,
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
    /// Components managed automatically by bevy's internal systems.
    /// Any manual modification or initialization to non-default values of these components should be avoided, as it can lead to unpredictable behaviour.
    /// 
    /// * [`Node`]: Describes the logical size of the node
    /// * [`Transform`]: The transform of the node
    /// * [`GlobalTransform`]: The global transform of the node
    /// * [`ComputedVisibility`]: Algorithmically-computed indication of whether an entity is visible.
    pub internal_components: (Node, Transform, GlobalTransform, ComputedVisibility),
}

impl Default for ButtonBundle {
    fn default() -> Self {
        Self {
            focus_policy: FocusPolicy::Block,
            button: Default::default(),
            style: Default::default(),
            border_color: BorderColor(Color::NONE),
            interaction: Default::default(),
            background_color: Default::default(),
            image: Default::default(),
            visibility: Default::default(),
            z_index: Default::default(),
            internal_components: Default::default(),
        }
    }
}
