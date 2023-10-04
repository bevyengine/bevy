use crate::{
    measurement::AvailableSpace, ContentSize, Measure, Node, UiImage, UiScale, UiTextureAtlasImage,
};
use bevy_asset::{Assets, Handle};

use bevy_ecs::change_detection::DetectChanges;
use bevy_ecs::query::Without;
use bevy_ecs::{
    prelude::Component,
    query::With,
    reflect::ReflectComponent,
    system::{Local, Query, Res},
};
use bevy_math::Vec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::TextureAtlas;
use bevy_window::{PrimaryWindow, Window};

/// The size of the image's texture
///
/// This component is updated automatically by [`update_image_content_size_system`]
#[derive(Component, Debug, Copy, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct UiImageSize {
    /// The size of the image's texture
    ///
    /// This field is updated automatically by [`update_image_content_size_system`]
    size: Vec2,
}

impl UiImageSize {
    /// The size of the image's texture
    pub fn size(&self) -> Vec2 {
        self.size
    }
}

#[derive(Clone)]
/// Used to calculate the size of UI image nodes
pub struct ImageMeasure {
    /// The size of the image's texture
    pub size: Vec2,
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

#[cfg(feature = "bevy_text")]
type UpdateImageFilter = (With<Node>, Without<bevy_text::Text>);
#[cfg(not(feature = "bevy_text"))]
type UpdateImageFilter = With<Node>;

/// Updates content size of the node based on the image provided
pub fn update_image_content_size_system(
    mut previous_combined_scale_factor: Local<f64>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    textures: Res<Assets<Image>>,
    mut query: Query<(&mut ContentSize, &UiImage, &mut UiImageSize), UpdateImageFilter>,
) {
    let combined_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.)
        * ui_scale.0;

    for (mut content_size, image, mut image_size) in &mut query {
        if let Some(texture) = textures.get(&image.texture) {
            let size = Vec2::new(
                texture.texture_descriptor.size.width as f32,
                texture.texture_descriptor.size.height as f32,
            );
            // Update only if size or scale factor has changed to avoid needless layout calculations
            if size != image_size.size
                || combined_scale_factor != *previous_combined_scale_factor
                || content_size.is_added()
            {
                image_size.size = size;
                content_size.set(ImageMeasure {
                    // multiply the image size by the scale factor to get the physical size
                    size: size * combined_scale_factor as f32,
                });
            }
        }
    }

    *previous_combined_scale_factor = combined_scale_factor;
}

/// Updates content size of the node based on the texture atlas sprite
pub fn update_atlas_content_size_system(
    mut previous_combined_scale_factor: Local<f64>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    atlases: Res<Assets<TextureAtlas>>,
    mut atlas_query: Query<
        (
            &mut ContentSize,
            &Handle<TextureAtlas>,
            &UiTextureAtlasImage,
            &mut UiImageSize,
        ),
        (UpdateImageFilter, Without<UiImage>),
    >,
) {
    let combined_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.)
        * ui_scale.0;

    for (mut content_size, atlas, atlas_image, mut image_size) in &mut atlas_query {
        if let Some(atlas) = atlases.get(atlas) {
            let size = atlas.textures[atlas_image.index].size();
            // Update only if size or scale factor has changed to avoid needless layout calculations
            if size != image_size.size
                || combined_scale_factor != *previous_combined_scale_factor
                || content_size.is_added()
            {
                image_size.size = size;
                content_size.set(ImageMeasure {
                    // multiply the image size by the scale factor to get the physical size
                    size: size * combined_scale_factor as f32,
                });
            }
        }
    }

    *previous_combined_scale_factor = combined_scale_factor;
}
