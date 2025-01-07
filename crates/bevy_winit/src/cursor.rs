//! Components to customize winit cursor

use crate::{
    converters::convert_system_cursor_icon,
    state::{CursorSource, PendingCursor},
};
#[cfg(feature = "custom_cursor")]
use crate::{
    state::{CustomCursorCache, CustomCursorCacheKey},
    WinitCustomCursor,
};
use bevy_app::{App, Last, Plugin};
#[cfg(feature = "custom_cursor")]
use bevy_asset::{Assets, Handle};
#[cfg(feature = "custom_cursor")]
use bevy_ecs::system::Res;
use bevy_ecs::{
    change_detection::DetectChanges,
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Local, Query},
    world::{OnRemove, Ref},
};
#[cfg(feature = "custom_cursor")]
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
#[cfg(feature = "custom_cursor")]
use bevy_math::{Rect, URect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_utils::HashSet;
use bevy_window::{SystemCursorIcon, Window};
#[cfg(feature = "custom_cursor")]
use tracing::warn;
#[cfg(feature = "custom_cursor")]
use wgpu_types::TextureFormat;

pub(crate) struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "custom_cursor")]
        app.init_resource::<CustomCursorCache>();

        app.register_type::<CursorIcon>()
            .add_systems(Last, update_cursors);

        app.add_observer(on_remove_cursor_icon);
    }
}

/// Insert into a window entity to set the cursor for that window.
#[derive(Component, Debug, Clone, Reflect, PartialEq, Eq)]
#[reflect(Component, Debug, Default, PartialEq)]
pub enum CursorIcon {
    #[cfg(feature = "custom_cursor")]
    /// Custom cursor image.
    Custom(CustomCursor),
    /// System provided cursor icon.
    System(SystemCursorIcon),
}

impl Default for CursorIcon {
    fn default() -> Self {
        CursorIcon::System(Default::default())
    }
}

impl From<SystemCursorIcon> for CursorIcon {
    fn from(icon: SystemCursorIcon) -> Self {
        CursorIcon::System(icon)
    }
}

#[cfg(feature = "custom_cursor")]
impl From<CustomCursor> for CursorIcon {
    fn from(cursor: CustomCursor) -> Self {
        CursorIcon::Custom(cursor)
    }
}

#[cfg(feature = "custom_cursor")]
/// Custom cursor image data.
#[derive(Debug, Clone, Reflect, PartialEq, Eq, Hash)]
pub enum CustomCursor {
    /// Image to use as a cursor.
    Image {
        /// The image must be in 8 bit int or 32 bit float rgba. PNG images
        /// work well for this.
        handle: Handle<Image>,
        /// The (optional) texture atlas used to render the image.
        texture_atlas: Option<TextureAtlas>,
        /// Whether the image should be flipped along its x-axis.
        flip_x: bool,
        /// Whether the image should be flipped along its y-axis.
        flip_y: bool,
        /// An optional rectangle representing the region of the image to
        /// render, instead of rendering the full image. This is an easy one-off
        /// alternative to using a [`TextureAtlas`].
        ///
        /// When used with a [`TextureAtlas`], the rect is offset by the atlas's
        /// minimal (top-left) corner position.
        rect: Option<URect>,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be
        /// within the image bounds.
        hotspot: (u16, u16),
    },
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    /// A URL to an image to use as the cursor.
    Url {
        /// Web URL to an image to use as the cursor. PNGs preferred. Cursor
        /// creation can fail if the image is invalid or not reachable.
        url: String,
        /// X and Y coordinates of the hotspot in pixels. The hotspot must be
        /// within the image bounds.
        hotspot: (u16, u16),
    },
}

fn update_cursors(
    mut commands: Commands,
    windows: Query<(Entity, Ref<CursorIcon>), With<Window>>,
    #[cfg(feature = "custom_cursor")] cursor_cache: Res<CustomCursorCache>,
    #[cfg(feature = "custom_cursor")] images: Res<Assets<Image>>,
    #[cfg(feature = "custom_cursor")] texture_atlases: Res<Assets<TextureAtlasLayout>>,
    mut queue: Local<HashSet<Entity>>,
) {
    for (entity, cursor) in windows.iter() {
        if !(queue.remove(&entity) || cursor.is_changed()) {
            continue;
        }

        let cursor_source = match cursor.as_ref() {
            #[cfg(feature = "custom_cursor")]
            CursorIcon::Custom(CustomCursor::Image {
                handle,
                texture_atlas,
                flip_x,
                flip_y,
                rect,
                hotspot,
            }) => {
                let cache_key = CustomCursorCacheKey::Image {
                    id: handle.id(),
                    texture_atlas_layout_id: texture_atlas.as_ref().map(|a| a.layout.id()),
                    texture_atlas_index: texture_atlas.as_ref().map(|a| a.index),
                    flip_x: *flip_x,
                    flip_y: *flip_y,
                    rect: *rect,
                };

                if cursor_cache.0.contains_key(&cache_key) {
                    CursorSource::CustomCached(cache_key)
                } else {
                    let Some(image) = images.get(handle) else {
                        warn!(
                            "Cursor image {handle:?} is not loaded yet and couldn't be used. Trying again next frame."
                        );
                        queue.insert(entity);
                        continue;
                    };

                    let atlas_rect = texture_atlas
                        .as_ref()
                        .and_then(|s| s.texture_rect(&texture_atlases))
                        .map(|r| r.as_rect());

                    let rect = match (atlas_rect, rect) {
                        (None, None) => Rect {
                            min: Vec2::ZERO,
                            max: Vec2::new(
                                image.texture_descriptor.size.width as f32,
                                image.texture_descriptor.size.height as f32,
                            ),
                        },
                        (None, Some(image_rect)) => image_rect.as_rect(),
                        (Some(atlas_rect), None) => atlas_rect,
                        (Some(atlas_rect), Some(image_rect)) => {
                            let mut image_rect = image_rect.as_rect();
                            image_rect.min += atlas_rect.min;
                            image_rect.max += atlas_rect.min;
                            image_rect
                        }
                    };

                    let Some(rgba) = image_to_rgba_pixels(image, *flip_x, *flip_y, rect) else {
                        warn!("Cursor image {handle:?} not accepted because it's not rgba8 or rgba32float format");
                        continue;
                    };

                    let width = (rect.max.x - rect.min.x) as u16;
                    let height = (rect.max.y - rect.min.y) as u16;
                    let source = match WinitCustomCursor::from_rgba(
                        rgba, width, height, hotspot.0, hotspot.1,
                    ) {
                        Ok(source) => source,
                        Err(err) => {
                            warn!("Cursor image {handle:?} is invalid: {err}");
                            continue;
                        }
                    };

                    CursorSource::Custom((cache_key, source))
                }
            }
            #[cfg(all(
                feature = "custom_cursor",
                target_family = "wasm",
                target_os = "unknown"
            ))]
            CursorIcon::Custom(CustomCursor::Url { url, hotspot }) => {
                let cache_key = CustomCursorCacheKey::Url(url.clone());

                if cursor_cache.0.contains_key(&cache_key) {
                    CursorSource::CustomCached(cache_key)
                } else {
                    use crate::CustomCursorExtWebSys;
                    let source = WinitCustomCursor::from_url(url.clone(), hotspot.0, hotspot.1);
                    CursorSource::Custom((cache_key, source))
                }
            }
            CursorIcon::System(system_cursor_icon) => {
                CursorSource::System(convert_system_cursor_icon(*system_cursor_icon))
            }
        };

        commands
            .entity(entity)
            .insert(PendingCursor(Some(cursor_source)));
    }
}

/// Resets the cursor to the default icon when `CursorIcon` is removed.
fn on_remove_cursor_icon(trigger: Trigger<OnRemove, CursorIcon>, mut commands: Commands) {
    // Use `try_insert` to avoid panic if the window is being destroyed.
    commands
        .entity(trigger.target())
        .try_insert(PendingCursor(Some(CursorSource::System(
            convert_system_cursor_icon(SystemCursorIcon::Default),
        ))));
}

#[cfg(feature = "custom_cursor")]
/// Returns the `image` data as a `Vec<u8>` for the specified sub-region.
///
/// The image is flipped along the x and y axes if `flip_x` and `flip_y` are
/// `true`, respectively.
///
/// Only supports rgba8 and rgba32float formats.
fn image_to_rgba_pixels(image: &Image, flip_x: bool, flip_y: bool, rect: Rect) -> Option<Vec<u8>> {
    let image_data_as_u8s: Vec<u8>;

    let image_data = match image.texture_descriptor.format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint => Some(&image.data),
        TextureFormat::Rgba32Float => {
            image_data_as_u8s = image
                .data
                .chunks(4)
                .map(|chunk| {
                    let chunk = chunk.try_into().unwrap();
                    let num = bytemuck::cast_ref::<[u8; 4], f32>(chunk);
                    (num * 255.0) as u8
                })
                .collect::<Vec<u8>>();

            Some(&image_data_as_u8s)
        }
        _ => None,
    };

    let image_data = image_data?;

    let width = (rect.max.x - rect.min.x) as usize;
    let height = (rect.max.y - rect.min.y) as usize;
    let mut sub_image_data = Vec::with_capacity(width * height * 4); // assuming 4 bytes per pixel (RGBA8)

    for y in 0..height {
        for x in 0..width {
            let src_x = if flip_x { width - 1 - x } else { x };
            let src_y = if flip_y { height - 1 - y } else { y };
            let index = ((rect.min.y as usize + src_y)
                * image.texture_descriptor.size.width as usize
                + (rect.min.x as usize + src_x))
                * 4;
            sub_image_data.extend_from_slice(&image_data[index..index + 4]);
        }
    }

    Some(sub_image_data)
}

#[cfg(feature = "custom_cursor")]
#[cfg(test)]
mod tests {
    use bevy_asset::RenderAssetUsages;
    use bevy_image::Image;
    use bevy_math::Rect;
    use bevy_math::Vec2;
    use wgpu_types::{Extent3d, TextureDimension};

    use super::*;

    fn create_image_rgba8(data: &[u8]) -> Image {
        Image::new(
            Extent3d {
                width: 3,
                height: 3,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    fn create_image_rgba32float(data: &[u8]) -> Image {
        let float_data: Vec<f32> = data
            .chunks(4)
            .flat_map(|chunk| {
                chunk
                    .iter()
                    .map(|&x| x as f32 / 255.0) // convert each channel to f32
                    .collect::<Vec<f32>>()
            })
            .collect();

        Image::new(
            Extent3d {
                width: 3,
                height: 3,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            bytemuck::cast_slice(&float_data).to_vec(),
            TextureFormat::Rgba32Float,
            RenderAssetUsages::default(),
        )
    }

    macro_rules! test_image_to_rgba_pixels {
        ($name:ident, $flip_x:expr, $flip_y:expr, $rect:expr, $expected:expr) => {
            #[test]
            fn $name() {
                let image_data: &[u8] = &[
                    // Row 1: Red, Green, Blue
                    255, 0, 0, 255, // Red
                    0, 255, 0, 255, // Green
                    0, 0, 255, 255, // Blue
                    // Row 2: Yellow, Cyan, Magenta
                    255, 255, 0, 255, // Yellow
                    0, 255, 255, 255, // Cyan
                    255, 0, 255, 255, // Magenta
                    // Row 3: White, Gray, Black
                    255, 255, 255, 255, // White
                    128, 128, 128, 255, // Gray
                    0, 0, 0, 255, // Black
                ];

                // RGBA8 test
                {
                    let image = create_image_rgba8(image_data);
                    let rect = $rect;
                    let result = image_to_rgba_pixels(&image, $flip_x, $flip_y, rect);
                    assert_eq!(result, Some($expected.to_vec()));
                }

                // RGBA32Float test
                {
                    let image = create_image_rgba32float(image_data);
                    let rect = $rect;
                    let result = image_to_rgba_pixels(&image, $flip_x, $flip_y, rect);
                    assert_eq!(result, Some($expected.to_vec()));
                }
            }
        };
    }

    test_image_to_rgba_pixels!(
        no_flip_full_image,
        false,
        false,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 1: Red, Green, Blue
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
            // Row 2: Yellow, Cyan, Magenta
            255, 255, 0, 255, // Yellow
            0, 255, 255, 255, // Cyan
            255, 0, 255, 255, // Magenta
            // Row 3: White, Gray, Black
            255, 255, 255, 255, // White
            128, 128, 128, 255, // Gray
            0, 0, 0, 255, // Black
        ]
    );

    test_image_to_rgba_pixels!(
        flip_x_full_image,
        true,
        false,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 1 flipped: Blue, Green, Red
            0, 0, 255, 255, // Blue
            0, 255, 0, 255, // Green
            255, 0, 0, 255, // Red
            // Row 2 flipped: Magenta, Cyan, Yellow
            255, 0, 255, 255, // Magenta
            0, 255, 255, 255, // Cyan
            255, 255, 0, 255, // Yellow
            // Row 3 flipped: Black, Gray, White
            0, 0, 0, 255, // Black
            128, 128, 128, 255, // Gray
            255, 255, 255, 255, // White
        ]
    );

    test_image_to_rgba_pixels!(
        flip_y_full_image,
        false,
        true,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 3: White, Gray, Black
            255, 255, 255, 255, // White
            128, 128, 128, 255, // Gray
            0, 0, 0, 255, // Black
            // Row 2: Yellow, Cyan, Magenta
            255, 255, 0, 255, // Yellow
            0, 255, 255, 255, // Cyan
            255, 0, 255, 255, // Magenta
            // Row 1: Red, Green, Blue
            255, 0, 0, 255, // Red
            0, 255, 0, 255, // Green
            0, 0, 255, 255, // Blue
        ]
    );

    test_image_to_rgba_pixels!(
        flip_both_full_image,
        true,
        true,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 3 flipped: Black, Gray, White
            0, 0, 0, 255, // Black
            128, 128, 128, 255, // Gray
            255, 255, 255, 255, // White
            // Row 2 flipped: Magenta, Cyan, Yellow
            255, 0, 255, 255, // Magenta
            0, 255, 255, 255, // Cyan
            255, 255, 0, 255, // Yellow
            // Row 1 flipped: Blue, Green, Red
            0, 0, 255, 255, // Blue
            0, 255, 0, 255, // Green
            255, 0, 0, 255, // Red
        ]
    );

    test_image_to_rgba_pixels!(
        no_flip_rect,
        false,
        false,
        Rect {
            min: Vec2::new(1.0, 1.0),
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Only includes part of the original image (sub-rectangle)
            // Row 2, columns 2-3: Cyan, Magenta
            0, 255, 255, 255, // Cyan
            255, 0, 255, 255, // Magenta
            // Row 3, columns 2-3: Gray, Black
            128, 128, 128, 255, // Gray
            0, 0, 0, 255, // Black
        ]
    );

    test_image_to_rgba_pixels!(
        flip_x_rect,
        true,
        false,
        Rect {
            min: Vec2::new(1.0, 1.0),
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 2 flipped: Magenta, Cyan
            255, 0, 255, 255, // Magenta
            0, 255, 255, 255, // Cyan
            // Row 3 flipped: Black, Gray
            0, 0, 0, 255, // Black
            128, 128, 128, 255, // Gray
        ]
    );

    test_image_to_rgba_pixels!(
        flip_y_rect,
        false,
        true,
        Rect {
            min: Vec2::new(1.0, 1.0),
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 3 first: Gray, Black
            128, 128, 128, 255, // Gray
            0, 0, 0, 255, // Black
            // Row 2 second: Cyan, Magenta
            0, 255, 255, 255, // Cyan
            255, 0, 255, 255, // Magenta
        ]
    );

    test_image_to_rgba_pixels!(
        flip_both_rect,
        true,
        true,
        Rect {
            min: Vec2::new(1.0, 1.0),
            max: Vec2::new(3.0, 3.0)
        },
        [
            // Row 3 flipped: Black, Gray
            0, 0, 0, 255, // Black
            128, 128, 128, 255, // Gray
            // Row 2 flipped: Magenta, Cyan
            255, 0, 255, 255, // Magenta
            0, 255, 255, 255, // Cyan
        ]
    );
}
