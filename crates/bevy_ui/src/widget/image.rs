use crate::{CalculatedSize, UiImage};
use bevy_asset::Assets;
#[cfg(feature = "bevy_text")]
use bevy_ecs::query::Without;
use bevy_ecs::system::{Query, Res};
use bevy_math::Vec2;
use bevy_render::texture::Image;
#[cfg(feature = "bevy_text")]
use bevy_text::Text;

/// Updates calculated size of the node based on the image provided
pub fn update_image_calculated_size_system(
    textures: Res<Assets<Image>>,
    #[cfg(feature = "bevy_text")] mut query: Query<(&mut CalculatedSize, &UiImage), Without<Text>>,
    #[cfg(not(feature = "bevy_text"))] mut query: Query<(&mut CalculatedSize, &UiImage)>,
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
