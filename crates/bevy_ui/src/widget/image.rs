use crate::{CalculatedSize, UiImage, Val};
use bevy_asset::Assets;
use bevy_ecs::{
    query::Without,
    system::{Query, Res},
};
use bevy_render::texture::Image;
use bevy_text::Text;

/// Updates calculated size of the node based on the image provided
pub fn update_image_calculated_size_system(
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut CalculatedSize, &UiImage), Without<Text>>,
) {
    for (mut calculated_size, image) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let width = Val::Px(texture.texture_descriptor.size.width as f32);
            let height = Val::Px(texture.texture_descriptor.size.height as f32);
            let size = match image.orientation {
                crate::ImageOrientation::Identity
                | crate::ImageOrientation::Rotate180
                | crate::ImageOrientation::Flip
                | crate::ImageOrientation::FlipRotate180 => (width, height),
                crate::ImageOrientation::Rotate90
                | crate::ImageOrientation::Rotate270
                | crate::ImageOrientation::FlipRotate90
                | crate::ImageOrientation::FlipRotate270 => (height, width),
            }
            .into();

            // Update only if size has changed to avoid needless layout calculations
            if size != calculated_size.size {
                calculated_size.size = size;
            }
        }
    }
}
