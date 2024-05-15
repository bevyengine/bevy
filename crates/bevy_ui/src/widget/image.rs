use crate::{
    measurement::AvailableSpace, ContentSize, Measure, Node, NodeMeasure, Style, UiImage, UiScale,
};
use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::{TextureAtlas, TextureAtlasLayout};
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
    size: UVec2,
}

impl UiImageSize {
    /// The size of the image's texture
    pub fn size(&self) -> UVec2 {
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
    mut previous_combined_scale_factor: Local<f32>,
    windows: Query<&Window, With<PrimaryWindow>>,
    ui_scale: Res<UiScale>,
    textures: Res<Assets<Image>>,

    atlases: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<
        (
            &mut ContentSize,
            &UiImage,
            &mut UiImageSize,
            Option<&TextureAtlas>,
            Option<&mut Style>,
        ),
        UpdateImageFilter,
    >,
) {
    let combined_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.)
        * ui_scale.0;

    for (mut content_size, image, mut image_size, atlas_image, style) in &mut query {
        if let Some(size) = match atlas_image {
            Some(atlas) => atlas.texture_rect(&atlases).map(|t| t.size()),
            None => textures.get(&image.texture).map(|t| t.size()),
        } {
            // Update only if size or scale factor has changed to avoid needless layout calculations
            if size != image_size.size
                || combined_scale_factor != *previous_combined_scale_factor
                || content_size.is_added()
            {
                image_size.size = size;
                content_size.set(NodeMeasure::Image(ImageMeasure {
                    // multiply the image size by the scale factor to get the physical size
                    size: size.as_vec2() * combined_scale_factor,
                }));
                if let Some(mut style) = style {
                    let Vec2 { x, y } = size.as_vec2();
                    style.aspect_ratio = Some(x / y);
                }
            }
        }
    }

    *previous_combined_scale_factor = combined_scale_factor;
}
