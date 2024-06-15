use crate::{
    measurement::AvailableSpace, ContentSize, Measure, Node, NodeMeasure, UiImage, UiScale,
};
use bevy_asset::Assets;
use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::texture::Image;
use bevy_sprite::{TextureAtlas, TextureAtlasLayout};
use bevy_window::{PrimaryWindow, Window};
use taffy::{MaybeMath, MaybeResolve};

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
        available_width: AvailableSpace,
        available_height: AvailableSpace,
        style: &taffy::Style,
    ) -> Vec2 {
        // Convert available width/height into an option
        let parent_width = available_width.into_option();
        let parent_height = available_height.into_option();

        // Resolve styles
        let s_aspect_ratio = style.aspect_ratio;
        let s_width = style.size.width.maybe_resolve(parent_width);
        let s_min_width = style.min_size.width.maybe_resolve(parent_width);
        let s_max_width = style.max_size.width.maybe_resolve(parent_width);
        let s_height = style.size.height.maybe_resolve(parent_height);
        let s_min_height = style.min_size.height.maybe_resolve(parent_height);
        let s_max_height = style.max_size.height.maybe_resolve(parent_height);

        // Determine width and height from styles and known_sizes (if a size is available
        // from any of these sources)
        let width = width.or(s_width
            .or(s_min_width)
            .maybe_clamp(s_min_width, s_max_width));
        let height = height.or(s_height
            .or(s_min_height)
            .maybe_clamp(s_min_height, s_max_height));

        // Use aspect_ratio from style, fall back to inherent aspect ratio
        let aspect_ratio = s_aspect_ratio.unwrap_or_else(|| self.size.x / self.size.y);

        // Apply aspect ratio
        // If only one of width or height was determined at this point, then the other is set beyond this point using the aspect ratio.
        let taffy_size = taffy::Size { width, height }.maybe_apply_aspect_ratio(Some(aspect_ratio));

        // Use computed sizes or fall back to image's inherent size
        Vec2 {
            x: taffy_size
                .width
                .unwrap_or(self.size.x)
                .maybe_clamp(s_min_width, s_max_width),
            y: taffy_size
                .height
                .unwrap_or(self.size.y)
                .maybe_clamp(s_min_height, s_max_height),
        }
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
        ),
        UpdateImageFilter,
    >,
) {
    let combined_scale_factor = windows
        .get_single()
        .map(|window| window.resolution.scale_factor())
        .unwrap_or(1.)
        * ui_scale.0;

    for (mut content_size, image, mut image_size, atlas_image) in &mut query {
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
            }
        }
    }

    *previous_combined_scale_factor = combined_scale_factor;
}
