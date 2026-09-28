//! CPU conversion of an equirectangular (lat-long) panorama into a cubemap [`Image`].

use crate::{ctt_format::wgpu_to_ctt_texture_format, Image, TextureFormatPixelInfo};
use bevy_math::{ops, Vec3};
use core::f32::consts::{PI, TAU};
use ctt::{
    split_cubemap, AlphaMode, CubemapInput, EquirectangularFront, EquirectangularOrientation,
    FormatDesc, FormatExt, SurfaceRef,
};
use thiserror::Error;
use wgpu_types::{
    Extent3d, TextureDimension, TextureFormat, TextureViewDescriptor, TextureViewDimension,
};

const F16_MAX: f32 = half::f16::MAX.to_f32_const();
/// The largest face size ctt projects.
const MAX_FACE_SIZE: u32 = 1 << 15;

/// An error from [`Image::equirectangular_to_cubemap`].
#[non_exhaustive]
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EquirectangularToCubemapError {
    /// The source is empty or is not a single-layer 2D image with one mip level.
    #[error(
        "the equirectangular source must be a nonempty, single-layer 2D image with one mip level"
    )]
    WrongDimension,
    /// The image data is missing or incomplete.
    #[error("image data is missing or incomplete")]
    Uninitialized,
    /// The source format cannot be decoded by the converter.
    #[error("unsupported source texture format {0:?}; use an uncompressed, non-integer format")]
    UnsupportedSourceFormat(TextureFormat),
    /// `face_size` was zero or above the maximum.
    #[error("cubemap face size must be between 1 and {MAX_FACE_SIZE}")]
    InvalidFaceSize,
    /// The projection failed.
    #[error("cubemap projection failed: {0}")]
    ProjectionFailed(String),
}

impl Image {
    /// Converts an equirectangular (lat-long) image to a cubemap.
    ///
    /// Returns an image with six `face_size` × `face_size` faces, a cube texture
    /// view, and one mip level in [`TextureFormat::Rgba16Float`]. The source's
    /// sampler and asset usage are preserved. The face size is independent of the
    /// source resolution and does not need to be a power of two.
    ///
    /// The source can be any uncompressed, non-integer format. sRGB colors are
    /// converted to linear before filtering; alpha stays linear. Colors are
    /// weighted by alpha while filtering, so transparent texels do not bleed
    /// into their neighbors.
    ///
    /// Each output texel is filtered from a mip pyramid of the panorama with
    /// anisotropic taps, so faces smaller than the panorama and texels near the
    /// poles do not alias. Output channels are clamped to ±65504; non-finite
    /// results become zero.
    ///
    /// # Orientation
    ///
    /// For a unit world direction, the panorama coordinates are:
    ///
    /// ```text
    /// u = 0.5 + atan2(dir.z, dir.x) / (2 * pi)
    /// v = acos(dir.y) / pi
    /// ```
    ///
    /// The center column faces +X, the horizontal seam faces -X, and the top row
    /// faces +Y. +Z is at `u = 0.75` and -Z at `u = 0.25`.
    ///
    /// Faces use wgpu's order: +X, -X, +Y, -Y, +Z, -Z. World Z is flipped to
    /// match Bevy's cubemap sampling, so the last two faces hold world -Z and +Z.
    ///
    /// # Errors
    ///
    /// Returns [`EquirectangularToCubemapError`] if `face_size` is zero or too
    /// large, the source is not a nonempty, single-layer 2D image with one mip
    /// level, its format is unsupported, or its [`Image::data`] is missing or
    /// incomplete.
    pub fn equirectangular_to_cubemap(
        &self,
        face_size: u32,
    ) -> Result<Image, EquirectangularToCubemapError> {
        if !(1..=MAX_FACE_SIZE).contains(&face_size) {
            return Err(EquirectangularToCubemapError::InvalidFaceSize);
        }
        let width = self.width();
        let height = self.height();
        if self.texture_descriptor.dimension != TextureDimension::D2
            || self.texture_descriptor.size.depth_or_array_layers != 1
            || self.texture_descriptor.mip_level_count != 1
            || width == 0
            || height == 0
        {
            return Err(EquirectangularToCubemapError::WrongDimension);
        }
        let Some(data) = self.data.as_deref() else {
            return Err(EquirectangularToCubemapError::Uninitialized);
        };
        let source_format = self.texture_descriptor.format;
        let unsupported = || EquirectangularToCubemapError::UnsupportedSourceFormat(source_format);
        let pixel_size = source_format.pixel_size().map_err(|_| unsupported())?;
        let format = wgpu_to_ctt_texture_format(source_format).ok_or_else(unsupported)?;
        let bytes = data
            .get(..width as usize * height as usize * pixel_size)
            .ok_or(EquirectangularToCubemapError::Uninitialized)?;

        let faces = split_cubemap(CubemapInput::Equirectangular {
            surface: SurfaceRef {
                data: bytes,
                width,
                height,
                depth: 1,
                stride: width * pixel_size as u32,
                slice_stride: 0,
            },
            desc: FormatDesc {
                format,
                color_space: format.normalize().1,
                alpha: AlphaMode::Straight,
            },
            face_size: Some(face_size),
            // ctt's lat-long center faces +X in cube space, which with Bevy's Z
            // flip gives the orientation documented above.
            orientation: EquirectangularOrientation {
                front: EquirectangularFront::PosX,
                mirror: false,
            },
        })
        .map_err(|error| match error {
            ctt::Error::UnsupportedFormat(_) | ctt::Error::UnsupportedConversion(_) => {
                unsupported()
            }
            error => EquirectangularToCubemapError::ProjectionFailed(error.to_string()),
        })?;

        // ctt returns `Rgba32Float` faces.
        let mut halves = Vec::with_capacity(6 * face_size as usize * face_size as usize * 8);
        for face in faces.surfaces {
            for &bytes in face[0].data.as_chunks::<4>().0 {
                let channel = f32::from_le_bytes(bytes);
                let channel = if channel.is_finite() {
                    channel.clamp(-F16_MAX, F16_MAX)
                } else {
                    0.0
                };
                halves.extend_from_slice(&half::f16::from_f32(channel).to_le_bytes());
            }
        }

        let mut cubemap = Image::new(
            Extent3d {
                width: face_size,
                height: face_size,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            halves,
            TextureFormat::Rgba16Float,
            self.asset_usage,
        );
        cubemap.sampler = self.sampler.clone();
        cubemap.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });
        Ok(cubemap)
    }
}

/// Lat-long texture coordinates in `0..1` of a unit world direction.
pub fn equirectangular_uv(dir: Vec3) -> (f32, f32) {
    let u = 0.5 + ops::atan2(dir.z, dir.x) / TAU;
    let v = ops::acos(dir.y.clamp(-1.0, 1.0)) / PI;
    (u, v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_asset::RenderAssetUsages;
    use bevy_color::{ColorToComponents, Srgba};

    fn equirect_rgba32(width: u32, height: u32, texel: impl Fn(u32, u32) -> [f32; 4]) -> Image {
        let mut data = Vec::with_capacity((width * height * 16) as usize);
        for y in 0..height {
            for x in 0..width {
                for channel in texel(x, y) {
                    data.extend_from_slice(&channel.to_le_bytes());
                }
            }
        }
        Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba32Float,
            RenderAssetUsages::all(),
        )
    }

    fn texel(cubemap: &Image, face: u32, x: u32, y: u32) -> [f32; 4] {
        cubemap
            .get_color_at_3d(x, y, face)
            .unwrap()
            .to_linear()
            .to_f32_array()
    }

    fn assert_close(actual: [f32; 4], expected: [f32; 4], tolerance: f32) {
        for i in 0..4 {
            assert!(
                (actual[i] - expected[i]).abs() <= tolerance,
                "channel {i}: {actual:?} != {expected:?}"
            );
        }
    }

    #[test]
    fn output_is_a_cube_view_over_six_layers() {
        let mut image = equirect_rgba32(8, 4, |_, _| [0.0; 4]);
        image.sampler = crate::ImageSampler::nearest();
        image.asset_usage = RenderAssetUsages::MAIN_WORLD;
        let cubemap = image.equirectangular_to_cubemap(4).unwrap();
        assert_eq!(cubemap.sampler, image.sampler);
        assert_eq!(cubemap.asset_usage, image.asset_usage);
        assert_eq!(cubemap.texture_descriptor.mip_level_count, 1);
        assert_eq!(cubemap.texture_descriptor.size.depth_or_array_layers, 6);
        assert_eq!(cubemap.texture_descriptor.dimension, TextureDimension::D2);
        assert_eq!(
            cubemap.texture_descriptor.format,
            TextureFormat::Rgba16Float
        );
        assert_eq!(
            cubemap
                .texture_view_descriptor
                .as_ref()
                .and_then(|view| view.dimension),
            Some(TextureViewDimension::Cube)
        );
        assert_eq!(cubemap.data.as_ref().unwrap().len(), 6 * 4 * 4 * 8);
    }

    #[test]
    fn lat_long_u_matches_the_documented_landmarks() {
        let u = |dir| equirectangular_uv(dir).0;
        assert!((u(Vec3::X) - 0.5).abs() < 1e-6, "+X: {}", u(Vec3::X));
        assert!((u(Vec3::Z) - 0.75).abs() < 1e-6, "+Z: {}", u(Vec3::Z));
        assert!(
            (u(Vec3::NEG_Z) - 0.25).abs() < 1e-6,
            "-Z: {}",
            u(Vec3::NEG_Z)
        );
        let minus_x = u(Vec3::NEG_X);
        assert!(
            minus_x.abs() < 1e-6 || (minus_x - 1.0).abs() < 1e-6,
            "-X: {minus_x}"
        );
        assert!(equirectangular_uv(Vec3::Y).1.abs() < 1e-6);
        assert!((equirectangular_uv(Vec3::NEG_Y).1 - 1.0).abs() < 1e-6);
    }

    #[test]
    fn rgba16_output_stays_finite() {
        let hot = [1.0e6, f32::INFINITY, f32::NAN, 1.0];
        let cubemap = equirect_rgba32(16, 8, |_, _| hot)
            .equirectangular_to_cubemap(4)
            .unwrap();
        for face in 0..6 {
            for y in 0..4 {
                for x in 0..4 {
                    let t = texel(&cubemap, face, x, y);
                    assert!(t.iter().all(|c| c.is_finite()), "{t:?}");
                    assert_eq!(t[0], F16_MAX);
                    assert_eq!(t[1], 0.0);
                    assert_eq!(t[2], 0.0);
                    assert_eq!(t[3], 1.0);
                }
            }
        }
    }

    #[test]
    fn constant_panorama_gives_constant_faces() {
        let color = [0.25, 0.5, 2.0, 1.0];
        for (width, height, face_size) in [(16, 8, 8), (7, 3, 5), (1, 1, 1)] {
            let cubemap = equirect_rgba32(width, height, |_, _| color)
                .equirectangular_to_cubemap(face_size)
                .unwrap();
            for face in 0..6 {
                for y in 0..face_size {
                    for x in 0..face_size {
                        assert_close(texel(&cubemap, face, x, y), color, 1e-3);
                    }
                }
            }
        }
    }

    #[test]
    fn srgb_source_is_linearized() {
        let mut data = Vec::new();
        for _ in 0..(8 * 4) {
            data.extend_from_slice(&[128, 255, 0, 255]);
        }
        let image = Image::new(
            Extent3d {
                width: 8,
                height: 4,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        let cubemap = image.equirectangular_to_cubemap(2).unwrap();
        let expected = [Srgba::gamma_function(128.0 / 255.0), 1.0, 0.0, 1.0];
        assert_close(texel(&cubemap, 0, 0, 0), expected, 2e-3);
    }

    #[test]
    fn filtering_wraps_at_the_seam_and_clamps_at_the_poles() {
        let cubemap = equirect_rgba32(4, 2, |x, y| [1.0 + 2.0 * x as f32, y as f32, 0.0, 1.0])
            .equirectangular_to_cubemap(1)
            .unwrap();
        // -X blends the last and first columns equally.
        assert_close(texel(&cubemap, 1, 0, 0), [4.0, 0.5, 0.0, 1.0], 1e-3);
        assert_close(texel(&cubemap, 2, 0, 0), [4.0, 0.0, 0.0, 1.0], 1e-3);
        assert_close(texel(&cubemap, 3, 0, 0), [4.0, 1.0, 0.0, 1.0], 1e-3);
    }

    #[test]
    fn srgb_filtering_uses_linear_colors_weighted_by_alpha() {
        for (transparent_alpha, expected) in
            [(255, [0.5, 0.5, 0.5, 1.0]), (0, [1.0, 1.0, 1.0, 0.5])]
        {
            let image = Image::new(
                Extent3d {
                    width: 2,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                vec![0, 0, 0, transparent_alpha, 255, 255, 255, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::all(),
            );
            let cubemap = image.equirectangular_to_cubemap(1).unwrap();
            assert_close(texel(&cubemap, 0, 0, 0), expected, 1e-3);
        }
    }

    #[test]
    fn half_float_and_unorm_sources_preserve_single_pixel_colors() {
        for (format, data, expected) in [
            (
                TextureFormat::Rgba16Float,
                [0.25, -0.5, 2.0, 1.0]
                    .into_iter()
                    .flat_map(|value| half::f16::from_f32(value).to_le_bytes())
                    .collect(),
                [0.25, -0.5, 2.0, 1.0],
            ),
            (
                TextureFormat::Rgba8Unorm,
                vec![128, 255, 0, 64],
                [128.0 / 255.0, 1.0, 0.0, 64.0 / 255.0],
            ),
        ] {
            let image = Image::new(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                data,
                format,
                RenderAssetUsages::all(),
            );
            let cubemap = image.equirectangular_to_cubemap(3).unwrap();
            for face in 0..6 {
                for y in 0..3 {
                    for x in 0..3 {
                        assert_close(texel(&cubemap, face, x, y), expected, 1e-3);
                    }
                }
            }
        }
    }

    #[test]
    fn face_interiors_have_the_expected_orientation() {
        let panorama = equirect_rgba32(256, 128, |x, y| {
            let longitude = ((x as f32 + 0.5) / 256.0 - 0.5) * TAU;
            let latitude = (0.5 - (y as f32 + 0.5) / 128.0) * PI;
            let (sin_lon, cos_lon) = ops::sin_cos(longitude);
            let (sin_lat, cos_lat) = ops::sin_cos(latitude);
            [
                (cos_lat * cos_lon + 1.0) * 0.5,
                (sin_lat + 1.0) * 0.5,
                (cos_lat * sin_lon + 1.0) * 0.5,
                1.0,
            ]
        });
        let size = 8;
        let cubemap = panorama.equirectangular_to_cubemap(size).unwrap();
        for face in 0..6 {
            for y in 0..size {
                for x in 0..size {
                    let u = (x as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                    // World direction through the texel, with `v = -1` at the top.
                    let expected = match face {
                        0 => Vec3::new(1.0, -v, u),
                        1 => Vec3::new(-1.0, -v, -u),
                        2 => Vec3::new(u, 1.0, -v),
                        3 => Vec3::new(u, -1.0, v),
                        4 => Vec3::new(u, -v, -1.0),
                        _ => Vec3::new(-u, -v, 1.0),
                    }
                    .normalize();
                    // Filtering averages directions over the texel's footprint,
                    // which shortens them, so compare only where they point.
                    let [r, g, b, _] = texel(&cubemap, face, x, y);
                    let actual = (Vec3::new(r, g, b) * 2.0 - Vec3::ONE).normalize();
                    assert!(
                        actual.dot(expected) > 0.999,
                        "face {face} texel ({x}, {y}): {actual} != {expected}"
                    );
                }
            }
        }
    }

    #[test]
    fn rejects_invalid_layouts_and_incomplete_data() {
        let image = equirect_rgba32(8, 4, |_, _| [0.0; 4]);
        for invalid in 0..5 {
            let mut image = image.clone();
            match invalid {
                0 => image.texture_descriptor.dimension = TextureDimension::D3,
                1 => image.texture_descriptor.size.depth_or_array_layers = 6,
                2 => image.texture_descriptor.mip_level_count = 2,
                3 => image.texture_descriptor.size.width = 0,
                _ => image.texture_descriptor.size.height = 0,
            }
            assert_eq!(
                image.equirectangular_to_cubemap(2),
                Err(EquirectangularToCubemapError::WrongDimension)
            );
        }
        let mut image = image;
        image.data.as_mut().unwrap().pop();
        assert_eq!(
            image.equirectangular_to_cubemap(2),
            Err(EquirectangularToCubemapError::Uninitialized)
        );
    }

    #[test]
    fn rejects_bad_inputs() {
        let mut image = equirect_rgba32(8, 4, |_, _| [0.0; 4]);
        assert_eq!(
            image.equirectangular_to_cubemap(0),
            Err(EquirectangularToCubemapError::InvalidFaceSize)
        );
        assert_eq!(
            image.equirectangular_to_cubemap(MAX_FACE_SIZE + 1),
            Err(EquirectangularToCubemapError::InvalidFaceSize)
        );
        for format in [TextureFormat::Rgba32Uint, TextureFormat::Bc6hRgbUfloat] {
            image.texture_descriptor.format = format;
            assert_eq!(
                image.equirectangular_to_cubemap(2),
                Err(EquirectangularToCubemapError::UnsupportedSourceFormat(
                    format
                ))
            );
        }
        image.texture_descriptor.format = TextureFormat::Rgba32Float;
        image.data = None;
        assert_eq!(
            image.equirectangular_to_cubemap(2),
            Err(EquirectangularToCubemapError::Uninitialized)
        );
    }
}
