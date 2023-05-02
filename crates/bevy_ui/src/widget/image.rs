use crate::{measurement::AvailableSpace, ContentSize, Measure, Node, UiImage};
use bevy_asset::Assets;
#[cfg(feature = "bevy_text")]
use bevy_ecs::query::Without;
use bevy_ecs::{
    prelude::Component,
    query::With,
    reflect::ReflectComponent,
    system::{Query, Res},
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, FromReflect, Reflect, ReflectFromReflect};
use bevy_render::texture::Image;
#[cfg(feature = "bevy_text")]
use bevy_text::Text;

/// The size of the image in physical pixels
///
/// This field is set automatically by `update_image_calculated_size_system`
#[derive(Component, Debug, Copy, Clone, Default, Reflect, FromReflect)]
#[reflect(Component, Default, FromReflect)]
pub struct UiImageSize {
    size: Vec2,
}

impl UiImageSize {
    pub fn size(&self) -> Vec2 {
        self.size
    }
}

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
}

/// Updates content size of the node based on the image provided
pub fn update_image_content_size_system(
    textures: Res<Assets<Image>>,
    #[cfg(feature = "bevy_text")] mut query: Query<
        (&mut ContentSize, &UiImage, &mut UiImageSize),
        (With<Node>, Without<Text>),
    >,
    #[cfg(not(feature = "bevy_text"))] mut query: Query<
        (&mut ContentSize, &UiImage, &mut UiImageSize),
        With<Node>,
    >,
) {
    for (mut content_size, image, mut image_size) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Vec2::new(
                texture.texture_descriptor.size.width as f32,
                texture.texture_descriptor.size.height as f32,
            );
            // Update only if size has changed to avoid needless layout calculations
            if size != image_size.size {
                image_size.size = size;
                content_size.set(ImageMeasure { size });
            }
        }
    }
}
