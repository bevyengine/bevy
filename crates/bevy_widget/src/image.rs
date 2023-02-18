use bevy_app::{App, CoreSet, Plugin};
use bevy_asset::Assets;
use bevy_ecs::{
    prelude::Bundle,
    query::Without,
    schedule::IntoSystemConfig,
    system::{Query, Res},
};
use bevy_math::Vec2;
use bevy_render::{
    texture::Image,
    view::{ComputedVisibility, Visibility},
};
use bevy_text::Text;
use bevy_transform::prelude::{GlobalTransform, Transform};
use bevy_ui::{
    BackgroundColor, CalculatedSize, FocusPolicy, Node, Style, UiImage, UiSystem, ZIndex,
};

use super::text_system;

/// Updates calculated size of the node based on the image provided
pub fn update_image_calculated_size_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut CalculatedSize, &UiImage), Without<Text>>,
) {
    for (mut calculated_size, image) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Vec2::new(
                texture.texture_descriptor.size.width as f32,
                texture.texture_descriptor.size.height as f32,
            );
            // Update only if size has changed to avoid needless layout calculations
            if size != calculated_size.size {
                calculated_size.size = size;
                calculated_size.preserve_aspect_ratio = true;
            }
        }
    }
}

/// A plugin for image widgets
#[derive(Default)]
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        app.add_system(
            update_image_calculated_size_system
                .in_base_set(CoreSet::PostUpdate)
                .before(UiSystem::Flex)
                // Potential conflicts: `Assets<Image>`
                // They run independently since `widget::image_node_system` will only ever observe
                // its own UiImage, and `widget::text_system` & `bevy_text::update_text2d_layout`
                // will never modify a pre-existing `Image` asset.
                .ambiguous_with(bevy_text::update_text2d_layout)
                .ambiguous_with(text_system),
        );
    }
}

/// A UI node that is an image
#[derive(Bundle, Clone, Debug, Default)]
pub struct ImageBundle {
    /// Describes the size of the node
    pub node: Node,
    /// Describes the style including flexbox settings
    pub style: Style,
    /// The calculated size based on the given image
    pub calculated_size: CalculatedSize,
    /// The background color, which serves as a "fill" for this node
    ///
    /// Combines with `UiImage` to tint the provided image.
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
    /// Indicates the depth at which the node should appear in the UI
    pub z_index: ZIndex,
}
