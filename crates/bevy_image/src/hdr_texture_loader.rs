use crate::{Image, SourceColorPrimaries, TextureFormatPixelInfo};
use bevy_asset::RenderAssetUsages;
use bevy_asset::{io::Reader, AssetLoader, LoadContext};
use bevy_color::{Chromaticity, RgbPrimaries};
use bevy_reflect::TypePath;
use image::DynamicImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};
use {bevy_utils::once, tracing::warn};

/// Loads HDR textures as Texture assets
#[derive(Clone, Default, TypePath)]
pub struct HdrTextureLoader;

/// Settings for [`HdrTextureLoader`].
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HdrTextureLoaderSettings {
    /// Where the asset will be used - see the docs on [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
    /// Overrides [`Image::source_color_primaries`]. With the default `None`, the loader
    /// reads the file's `PRIMARIES=` header line. See [`SourceColorPrimaries`] for the
    /// resolution order.
    #[serde(default)]
    pub source_color_primaries: Option<SourceColorPrimaries>,
}

/// Possible errors that can be produced by [`HdrTextureLoader`]
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum HdrTextureLoaderError {
    /// I/O Error.
    #[error("Could load texture: {0}")]
    Io(#[from] std::io::Error),
    /// Failed to decode the texture.
    #[error("Could not extract image: {0}")]
    Image(#[from] image::ImageError),
}

impl AssetLoader for HdrTextureLoader {
    type Asset = Image;
    type Settings = HdrTextureLoaderSettings;
    type Error = HdrTextureLoaderError;
    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let format = TextureFormat::Rgba32Float;
        // `Rgba32Float` will always return a valid pixel size
        let pixel_size = format.pixel_size().unwrap();
        debug_assert_eq!(pixel_size, 4 * 4, "Format should have 32bit x 4 size");

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let decoder = image::codecs::hdr::HdrDecoder::new(bytes.as_slice())?;
        let info = decoder.metadata();
        let dynamic_image = DynamicImage::from_decoder(decoder)?;
        let image_buffer = dynamic_image
            .as_rgb32f()
            .expect("HDR Image format should be Rgb32F");
        let mut rgba_data = Vec::with_capacity(image_buffer.pixels().len() * pixel_size);

        for rgb in image_buffer.pixels() {
            let alpha = 1.0f32;

            rgba_data.extend_from_slice(&rgb.0[0].to_le_bytes());
            rgba_data.extend_from_slice(&rgb.0[1].to_le_bytes());
            rgba_data.extend_from_slice(&rgb.0[2].to_le_bytes());
            rgba_data.extend_from_slice(&alpha.to_le_bytes());
        }

        let mut image = Image::new(
            Extent3d {
                width: info.width,
                height: info.height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            rgba_data,
            format,
            settings.asset_usage,
        );
        image.source_color_primaries =
            SourceColorPrimaries::resolve(settings.source_color_primaries, || {
                parse_radiance_primaries(&info.custom_attributes)
            });
        Ok(image)
    }

    fn extensions(&self) -> &[&str] {
        &["hdr"]
    }
}

/// Parses a Radiance `PRIMARIES=` header line and matches it against the supported
/// [`SourceColorPrimaries`].
///
/// Returns `None` when the line is absent, malformed, or names primaries Bevy does not
/// support. Unsupported primaries log a warning once.
fn parse_radiance_primaries(
    custom_attributes: &[(String, String)],
) -> Option<SourceColorPrimaries> {
    let (_, value) = custom_attributes
        .iter()
        .find(|(key, _)| key.eq_ignore_ascii_case("PRIMARIES"))?;
    let mut coordinates = [0.0f32; 8];
    let mut values = value.split_whitespace();
    for coordinate in &mut coordinates {
        *coordinate = values.next()?.parse().ok()?;
    }
    if values.next().is_some() {
        // A `PRIMARIES=` line carries exactly eight coordinates.
        return None;
    }
    let source_color_primaries = SourceColorPrimaries::from_chromaticities(RgbPrimaries {
        red: Chromaticity::new(coordinates[0], coordinates[1]),
        green: Chromaticity::new(coordinates[2], coordinates[3]),
        blue: Chromaticity::new(coordinates[4], coordinates[5]),
        white: Chromaticity::new(coordinates[6], coordinates[7]),
    });
    if source_color_primaries.is_none() {
        once!(warn!(
            "Radiance HDR file declares PRIMARIES \"{value}\", which Bevy does not support. \
            Assuming BT.709 primaries.",
        ));
    }
    source_color_primaries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attributes(value: &str) -> Vec<(String, String)> {
        vec![("PRIMARIES".to_owned(), value.to_owned())]
    }

    #[test]
    fn radiance_primaries_match_known_sets() {
        for (value, expected) in [
            (
                "0.640 0.330 0.300 0.600 0.150 0.060 0.3127 0.3290",
                SourceColorPrimaries::Bt709,
            ),
            (
                "0.708 0.292 0.170 0.797 0.131 0.046 0.3127 0.3290",
                SourceColorPrimaries::Bt2020,
            ),
            (
                "0.680 0.320 0.265 0.690 0.150 0.060 0.3127 0.3290",
                SourceColorPrimaries::DisplayP3,
            ),
        ] {
            assert_eq!(parse_radiance_primaries(&attributes(value)), Some(expected));
        }
    }

    #[test]
    fn radiance_primaries_reject_unknown_or_malformed() {
        assert_eq!(parse_radiance_primaries(&[]), None);
        // Radiance's own default primaries are not BT.709. Green sits at 0.290, 0.600.
        assert_eq!(
            parse_radiance_primaries(&attributes(
                "0.640 0.330 0.290 0.600 0.150 0.060 0.333 0.333"
            )),
            None
        );
        assert_eq!(parse_radiance_primaries(&attributes("0.640 0.330")), None);
        assert_eq!(
            parse_radiance_primaries(&attributes("a b c d e f g h")),
            None
        );
    }
}
