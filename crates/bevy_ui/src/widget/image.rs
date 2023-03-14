use crate::{measurement::AvailableSpace, IntrinsicSize, Measure, UiImage};
use bevy_asset::Assets;
#[cfg(feature = "bevy_text")]
use bevy_ecs::query::Without;
use bevy_ecs::system::{Query, Res};
use bevy_math::Vec2;
use bevy_render::texture::Image;
#[cfg(feature = "bevy_text")]
use bevy_text::Text;

#[derive(Clone)]
pub struct ImageMeasure {
    // target size of the image
    size: Vec2,
}

impl Measure for ImageMeasure {
    fn measure(
        &self,
        width: Option<f32>,
        height: Option<f32>,
        _: AvailableSpace,
        _: AvailableSpace,
    ) -> Vec2 {
        let mut size = self.size;
        match (width, height) {
            (None, None) => {}
            (Some(width), None) => {
                size.y = width * size.y / size.x;
                size.x = width;
            }
            (None, Some(height)) => {
                size.x = height * size.x / size.y;
                size.y = height;
            }
            (Some(width), Some(height)) => {
                size.x = width;
                size.y = height;
            }
        }
        size
    }

    fn dyn_clone(&self) -> Box<dyn Measure> {
        Box::new(self.clone())
    }
}

/// Updates calculated size of the node based on the image provided
pub fn update_image_calculated_size_system(
    textures: Res<Assets<Image>>,
    #[cfg(feature = "bevy_text")] mut query: Query<(&mut IntrinsicSize, &UiImage), Without<Text>>,
    #[cfg(not(feature = "bevy_text"))] mut query: Query<(&mut IntrinsicSize, &UiImage)>,
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
                calculated_size.measure = Box::new(ImageMeasure { size });
            }
        }
    }
}
