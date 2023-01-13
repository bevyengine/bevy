use crate::{CalculatedSize, Size, UiImage, UiSystem, Val};
use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::Assets;
use bevy_ecs::{
    query::Without,
    schedule::IntoSystemDescriptor,
    system::{Query, Res},
};
use bevy_render::texture::Image;
use bevy_text::Text;

use super::text_system;

/// Updates calculated size of the node based on the image provided
pub fn update_image_calculated_size_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut CalculatedSize, &UiImage), Without<Text>>,
) {
    for (mut calculated_size, image) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Size {
                width: Val::Px(texture.texture_descriptor.size.width as f32),
                height: Val::Px(texture.texture_descriptor.size.height as f32),
            };
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
        app.add_system_to_stage(
            CoreStage::PostUpdate,
            update_image_calculated_size_system
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
