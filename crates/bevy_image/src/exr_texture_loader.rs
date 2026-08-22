use crate::{Image, SourceColorPrimaries, TextureFormatPixelInfo};
use bevy_asset::{io::Reader, AssetLoader, LoadContext, RenderAssetUsages};
use bevy_color::{Chromaticity, RgbPrimaries};
use bevy_reflect::TypePath;
use image::ImageDecoder;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::{Extent3d, TextureDimension, TextureFormat};
use {bevy_utils::once, tracing::warn};

/// Loads EXR textures as Texture assets
#[derive(Clone, Default, TypePath)]
#[cfg(feature = "exr")]
pub struct ExrTextureLoader;

/// Settings for [`ExrTextureLoader`].
#[derive(Serialize, Deserialize, Default, Debug)]
#[cfg(feature = "exr")]
pub struct ExrTextureLoaderSettings {
    /// Where the asset will be used - see the docs on [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
    /// Overrides the color primaries stamped on [`Image::source_color_primaries`]. With
    /// the default `None`, the loader reads the file's `chromaticities` attribute. See
    /// [`SourceColorPrimaries`] for the resolution order.
    #[serde(default)]
    pub source_color_primaries: Option<SourceColorPrimaries>,
}

/// Possible errors that can be produced by [`ExrTextureLoader`]
#[non_exhaustive]
#[derive(Debug, Error, TypePath)]
#[cfg(feature = "exr")]
pub enum ExrTextureLoaderError {
    /// I/O Error.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Failed to decode the texture.
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
}

impl AssetLoader for ExrTextureLoader {
    type Asset = Image;
    type Settings = ExrTextureLoaderSettings;
    type Error = ExrTextureLoaderError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> Result<Image, Self::Error> {
        let format = TextureFormat::Rgba32Float;
        debug_assert_eq!(
            // `Rgba32Float` will always return a valid pixel size
            format.pixel_size().unwrap(),
            4 * 4,
            "Format should have 32bit x 4 size"
        );

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;

        let decoder = image::codecs::openexr::OpenExrDecoder::with_alpha_preference(
            std::io::Cursor::new(bytes.as_slice()),
            Some(true),
        )?;
        let (width, height) = decoder.dimensions();

        let total_bytes = decoder.total_bytes() as usize;

        let mut buf = vec![0u8; total_bytes];
        decoder.read_image(buf.as_mut_slice())?;

        let mut image = Image::new(
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            buf,
            format,
            settings.asset_usage,
        );
        // The `image` crate's OpenEXR decoder drops the header color metadata, so read
        // the header again with the decoder's own `exr` crate.
        image.source_color_primaries =
            SourceColorPrimaries::resolve(settings.source_color_primaries, || {
                read_exr_chromaticities(&bytes)
            });
        Ok(image)
    }

    fn extensions(&self) -> &[&str] {
        &["exr"]
    }
}

/// Reads the `chromaticities` header attribute and matches it against the supported
/// [`SourceColorPrimaries`].
///
/// Returns `None` when the attribute is absent, the header cannot be parsed, or the
/// primaries are not supported. Unsupported primaries log a warning once.
fn read_exr_chromaticities(bytes: &[u8]) -> Option<SourceColorPrimaries> {
    // Errors are swallowed. This is best-effort metadata, and any structural problem
    // with the file surfaces in the decode instead.
    let metadata =
        exr::meta::MetaData::read_from_buffered(std::io::Cursor::new(bytes), false).ok()?;
    let chromaticities = metadata.headers.first()?.shared_attributes.chromaticities?;
    let source_color_primaries = SourceColorPrimaries::from_chromaticities(RgbPrimaries {
        red: Chromaticity::new(chromaticities.red.0, chromaticities.red.1),
        green: Chromaticity::new(chromaticities.green.0, chromaticities.green.1),
        blue: Chromaticity::new(chromaticities.blue.0, chromaticities.blue.1),
        white: Chromaticity::new(chromaticities.white.0, chromaticities.white.1),
    });
    if source_color_primaries.is_none() {
        once!(warn!(
            "OpenEXR file declares chromaticities {chromaticities:?}, which Bevy does not \
            support. Assuming BT.709 primaries.",
        ));
    }
    source_color_primaries
}

#[cfg(test)]
mod tests {
    use super::*;
    use exr::image::write::WritableImage;

    /// Writes a 1x1 RGBA EXR to memory, optionally with a `chromaticities` attribute.
    fn write_test_exr(chromaticities: Option<exr::meta::attribute::Chromaticities>) -> Vec<u8> {
        let mut image = exr::image::Image::from_channels(
            (1, 1),
            exr::image::SpecificChannels::rgba(|_: exr::math::Vec2<usize>| {
                (0.5f32, 0.5f32, 0.5f32, 1.0f32)
            }),
        );
        image.attributes.chromaticities = chromaticities;
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write().to_buffered(&mut bytes).unwrap();
        bytes.into_inner()
    }

    fn chromaticities(primaries: RgbPrimaries) -> exr::meta::attribute::Chromaticities {
        exr::meta::attribute::Chromaticities {
            red: exr::math::Vec2(primaries.red.x, primaries.red.y),
            green: exr::math::Vec2(primaries.green.x, primaries.green.y),
            blue: exr::math::Vec2(primaries.blue.x, primaries.blue.y),
            white: exr::math::Vec2(primaries.white.x, primaries.white.y),
        }
    }

    #[test]
    fn exr_chromaticities_are_read_from_the_header() {
        let bytes = write_test_exr(Some(chromaticities(RgbPrimaries::BT2020)));
        assert_eq!(
            read_exr_chromaticities(&bytes),
            Some(SourceColorPrimaries::Bt2020)
        );
    }

    #[test]
    fn exr_without_chromaticities_yields_none() {
        let bytes = write_test_exr(None);
        assert_eq!(read_exr_chromaticities(&bytes), None);
    }

    #[test]
    fn exr_with_unknown_chromaticities_yields_none() {
        // ACEScg primaries are a valid file value but not a supported variant.
        let bytes = write_test_exr(Some(chromaticities(RgbPrimaries::ACES_CG)));
        assert_eq!(read_exr_chromaticities(&bytes), None);
    }
}
