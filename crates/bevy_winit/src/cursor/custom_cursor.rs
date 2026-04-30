use bevy_asset::{AssetId, Assets};
use bevy_ecs::resource::Resource;
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy_math::{ops, Rect, URect, UVec2, Vec2};
use bevy_platform::collections::HashMap;
use wgpu_types::TextureFormat;

/// Caches custom cursors. On many platforms, creating custom cursors is expensive, especially on
/// the web.
#[derive(Debug, Clone, Default, Resource)]
pub struct WinitCustomCursorCache(pub HashMap<CustomCursorCacheKey, winit::window::CustomCursor>);

/// Identifiers for custom cursors used in caching.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CustomCursorCacheKey {
    /// A custom cursor with an image.
    Image {
        id: AssetId<Image>,
        texture_atlas_layout_id: Option<AssetId<TextureAtlasLayout>>,
        texture_atlas_index: Option<usize>,
        flip_x: bool,
        flip_y: bool,
        rect: Option<URect>,
    },
    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    /// A custom cursor with a URL.
    Url(String),
}

/// Determines the effective rect and returns it along with a flag to indicate
/// whether a sub-image operation is needed. The flag allows the caller to
/// determine whether the image data needs a sub-image extracted from it. Note:
/// To avoid lossy comparisons between [`Rect`] and [`URect`], the flag is
/// always set to `true` when a [`TextureAtlas`] is used.
#[inline(always)]
pub(crate) fn calculate_effective_rect(
    texture_atlas_layouts: &Assets<TextureAtlasLayout>,
    image: &Image,
    texture_atlas: &Option<TextureAtlas>,
    rect: &Option<URect>,
) -> (Rect, bool) {
    let atlas_rect = texture_atlas
        .as_ref()
        .and_then(|s| s.texture_rect(texture_atlas_layouts))
        .map(|r| r.as_rect());

    match (atlas_rect, rect) {
        (None, None) => (
            Rect {
                min: Vec2::ZERO,
                max: Vec2::new(
                    image.texture_descriptor.size.width as f32,
                    image.texture_descriptor.size.height as f32,
                ),
            },
            false,
        ),
        (None, Some(image_rect)) => (
            image_rect.as_rect(),
            image_rect
                != &URect {
                    min: UVec2::ZERO,
                    max: UVec2::new(
                        image.texture_descriptor.size.width,
                        image.texture_descriptor.size.height,
                    ),
                },
        ),
        (Some(atlas_rect), None) => (atlas_rect, true),
        (Some(atlas_rect), Some(image_rect)) => (
            {
                let mut image_rect = image_rect.as_rect();
                image_rect.min += atlas_rect.min;
                image_rect.max += atlas_rect.min;
                image_rect
            },
            true,
        ),
    }
}

/// Extracts the RGBA pixel data from `image`, converting it if necessary.
///
/// Only supports rgba8 and rgba32float formats.
pub(crate) fn extract_rgba_pixels(image: &Image) -> Option<Vec<u8>> {
    match image.texture_descriptor.format {
        TextureFormat::Rgba8Unorm
        | TextureFormat::Rgba8UnormSrgb
        | TextureFormat::Rgba8Snorm
        | TextureFormat::Rgba8Uint
        | TextureFormat::Rgba8Sint => Some(image.data.clone()?),
        TextureFormat::Rgba32Float => image.data.as_ref().map(|data| {
            data.chunks(4)
                .map(|chunk| {
                    let chunk = chunk.try_into().unwrap();
                    let num = bytemuck::cast_ref::<[u8; 4], f32>(chunk);
                    ops::round(num.clamp(0.0, 1.0) * 255.0) as u8
                })
                .collect()
        }),
        _ => None,
    }
}

/// Returns the `image` data as a `Vec<u8>` for the specified sub-region.
///
/// The image is flipped along the x and y axes if `flip_x` and `flip_y` are
/// `true`, respectively.
///
/// Only supports rgba8 and rgba32float formats.
pub(crate) fn extract_and_transform_rgba_pixels(
    image: &Image,
    flip_x: bool,
    flip_y: bool,
    rect: Rect,
) -> Option<Vec<u8>> {
    let image_data = extract_rgba_pixels(image)?;

    let width = rect.width() as usize;
    let height = rect.height() as usize;
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

/// Transforms the `hotspot` coordinates based on whether the image is flipped
/// or not. The `rect` is used to determine the image's dimensions.
pub(crate) fn transform_hotspot(
    hotspot: (u16, u16),
    flip_x: bool,
    flip_y: bool,
    rect: Rect,
) -> (u16, u16) {
    let hotspot_x = hotspot.0 as f32;
    let hotspot_y = hotspot.1 as f32;

    let (width, height) = (rect.width(), rect.height());

    let hotspot_x = if flip_x {
        (width - 1.0).max(0.0) - hotspot_x
    } else {
        hotspot_x
    };
    let hotspot_y = if flip_y {
        (height - 1.0).max(0.0) - hotspot_y
    } else {
        hotspot_y
    };

    (hotspot_x as u16, hotspot_y as u16)
}

#[cfg(test)]
mod tests {
    use bevy_app::App;
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

    macro_rules! test_calculate_effective_rect {
        ($name:ident, $use_texture_atlas:expr, $rect:expr, $expected_rect:expr, $expected_needs_sub_image:expr) => {
            #[test]
            fn $name() {
                let mut app = App::new();
                let mut texture_atlas_layout_assets = Assets::<TextureAtlasLayout>::default();

                // Create a simple 3x3 texture atlas layout for the test cases
                // that use a texture atlas. In the future we could adjust the
                // test cases to use different texture atlas layouts.
                let layout = TextureAtlasLayout::from_grid(UVec2::new(3, 3), 1, 1, None, None);
                let layout_handle = texture_atlas_layout_assets.add(layout);

                app.insert_resource(texture_atlas_layout_assets);

                let texture_atlases = app
                    .world()
                    .get_resource::<Assets<TextureAtlasLayout>>()
                    .unwrap();

                let image = create_image_rgba8(&[0; 3 * 3 * 4]); // 3x3 image

                let texture_atlas = if $use_texture_atlas {
                    Some(TextureAtlas::from(layout_handle))
                } else {
                    None
                };

                let rect = $rect;

                let (result_rect, needs_sub_image) =
                    calculate_effective_rect(&texture_atlases, &image, &texture_atlas, &rect);

                assert_eq!(result_rect, $expected_rect);
                assert_eq!(needs_sub_image, $expected_needs_sub_image);
            }
        };
    }

    test_calculate_effective_rect!(
        no_texture_atlas_no_rect,
        false,
        None,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        false
    );

    test_calculate_effective_rect!(
        no_texture_atlas_with_partial_rect,
        false,
        Some(URect {
            min: UVec2::new(1, 1),
            max: UVec2::new(3, 3)
        }),
        Rect {
            min: Vec2::new(1.0, 1.0),
            max: Vec2::new(3.0, 3.0)
        },
        true
    );

    test_calculate_effective_rect!(
        no_texture_atlas_with_full_rect,
        false,
        Some(URect {
            min: UVec2::ZERO,
            max: UVec2::new(3, 3)
        }),
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        false
    );

    test_calculate_effective_rect!(
        texture_atlas_no_rect,
        true,
        None,
        Rect {
            min: Vec2::ZERO,
            max: Vec2::new(3.0, 3.0)
        },
        true // always needs sub-image to avoid comparing Rect against URect
    );

    test_calculate_effective_rect!(
        texture_atlas_rect,
        true,
        Some(URect {
            min: UVec2::ZERO,
            max: UVec2::new(3, 3)
        }),
        Rect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(3.0, 3.0)
        },
        true // always needs sub-image to avoid comparing Rect against URect
    );

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

    macro_rules! test_extract_and_transform_rgba_pixels {
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
                    let result = extract_and_transform_rgba_pixels(&image, $flip_x, $flip_y, rect);
                    assert_eq!(result, Some($expected.to_vec()));
                }

                // RGBA32Float test
                {
                    let image = create_image_rgba32float(image_data);
                    let rect = $rect;
                    let result = extract_and_transform_rgba_pixels(&image, $flip_x, $flip_y, rect);
                    assert_eq!(result, Some($expected.to_vec()));
                }
            }
        };
    }

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    test_extract_and_transform_rgba_pixels!(
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

    #[test]
    fn test_transform_hotspot() {
        fn test(hotspot: (u16, u16), flip_x: bool, flip_y: bool, rect: Rect, expected: (u16, u16)) {
            let transformed = transform_hotspot(hotspot, flip_x, flip_y, rect);
            assert_eq!(transformed, expected);

            // Round-trip test: Applying the same transformation again should
            // reverse it.
            let transformed = transform_hotspot(transformed, flip_x, flip_y, rect);
            assert_eq!(transformed, hotspot);
        }

        let rect = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(100.0, 200.0),
        };

        test((10, 20), false, false, rect, (10, 20)); // no flip
        test((10, 20), true, false, rect, (89, 20)); // flip X
        test((10, 20), false, true, rect, (10, 179)); // flip Y
        test((10, 20), true, true, rect, (89, 179)); // flip both
        test((0, 0), true, true, rect, (99, 199)); // flip both (bounds check)
    }
}
