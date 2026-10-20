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

/// Loads Radiance HDR images, optionally converting lat-long panoramas to cubemaps.
#[derive(Clone, Default, TypePath)]
pub struct HdrTextureLoader;

/// Settings for [`HdrTextureLoader`].
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct HdrTextureLoaderSettings {
    /// Where the asset will be used - see the docs on [`RenderAssetUsages`] for details.
    pub asset_usage: RenderAssetUsages,
    /// Overrides [`Image::source_color_primaries`]. By default, the loader reads the
    /// file's `PRIMARIES=` header line. See [`SourceColorPrimaries`] for the
    /// resolution order.
    #[serde(default)]
    pub source_color_primaries: Option<SourceColorPrimaries>,
    /// Converts a lat-long panorama to a cubemap with faces of this size.
    ///
    /// `None` keeps the image as a 2D panorama. `Some(size)` uses
    /// `Image::equirectangular_to_cubemap` and returns a cube texture view. It
    /// requires the `equirectangular_cubemap` feature; without it, loading fails.
    #[serde(default)]
    pub cubemap_face_size: Option<u32>,
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
    /// Failed to convert the panorama to a cubemap.
    #[cfg(feature = "equirectangular_cubemap")]
    #[error(transparent)]
    Cubemap(#[from] crate::EquirectangularToCubemapError),
    /// `cubemap_face_size` was set without the `equirectangular_cubemap` feature.
    #[error("converting to a cubemap requires the `equirectangular_cubemap` feature")]
    CubemapUnsupported,
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
        match settings.cubemap_face_size {
            #[cfg(feature = "equirectangular_cubemap")]
            Some(face_size) => Ok(image.equirectangular_to_cubemap(face_size)?),
            #[cfg(not(feature = "equirectangular_cubemap"))]
            Some(_) => Err(HdrTextureLoaderError::CubemapUnsupported),
            None => Ok(image),
        }
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
    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{
        io::{
            memory::{Dir, MemoryAssetReader},
            AssetSourceBuilder, AssetSourceId,
        },
        AssetApp, AssetPlugin, AssetServer, Assets, LoadState,
    };
    use std::{
        path::Path,
        time::{Duration, Instant},
    };
    #[cfg(feature = "equirectangular_cubemap")]
    use wgpu_types::TextureViewDimension;

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

    fn hdr_bytes() -> Vec<u8> {
        let mut bytes = Vec::new();
        image::codecs::hdr::HdrEncoder::new(&mut bytes)
            .encode(&[image::Rgb([2.0, 1.0, 0.5]); 16 * 8], 16, 8)
            .unwrap();
        bytes
    }

    fn source_with_meta(meta: &str) -> Dir {
        let source = Dir::default();
        let path = Path::new("environment.hdr");
        source.insert_asset(path, hdr_bytes());
        source.insert_meta_text(path, meta);
        source
    }

    fn load_image(app: &mut App) -> Result<Image, String> {
        let server = app.world().resource::<AssetServer>().clone();
        let handle = server.load::<Image>("environment.hdr");
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(30) {
            app.update();
            match server.load_state(&handle) {
                LoadState::Loaded => {
                    return Ok(app
                        .world()
                        .resource::<Assets<Image>>()
                        .get(&handle)
                        .unwrap()
                        .clone());
                }
                LoadState::Failed(error) => return Err(error.to_string()),
                _ => std::thread::yield_now(),
            }
        }
        panic!("HDR asset did not finish loading");
    }

    fn load_with_settings(settings: &str) -> Result<Image, String> {
        let source = source_with_meta(&format!(
            r#"(
            meta_format_version: "1.0",
            asset: Load(
                loader: "bevy_image::hdr_texture_loader::HdrTextureLoader",
                settings: ({settings}),
            ),
        )"#
        ));
        let mut app = App::new();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: source.clone(),
                })
            }),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin {
                watch_for_changes_override: Some(false),
                use_asset_processor_override: Some(false),
                ..Default::default()
            },
            crate::ImagePlugin::default(),
        ));
        load_image(&mut app)
    }

    #[test]
    fn existing_metadata_keeps_the_panorama() {
        let image =
            load_with_settings(r#"asset_usage: RenderAssetUsages("RENDER_WORLD"),"#).unwrap();
        assert_eq!(
            image.texture_descriptor.size,
            Extent3d {
                width: 16,
                height: 8,
                depth_or_array_layers: 1,
            }
        );
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba32Float);
        assert_eq!(image.texture_view_descriptor, None);
    }

    #[cfg(feature = "equirectangular_cubemap")]
    #[test]
    fn metadata_loads_a_cubemap() {
        let image = load_with_settings(
            r#"
            asset_usage: RenderAssetUsages("RENDER_WORLD"),
            cubemap_face_size: Some(8),
        "#,
        )
        .unwrap();
        assert_eq!(
            image.texture_descriptor.size,
            Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 6,
            }
        );
        assert_eq!(image.texture_descriptor.format, TextureFormat::Rgba16Float);
        assert_eq!(image.texture_descriptor.mip_level_count, 1);
        assert_eq!(
            image.texture_view_descriptor.unwrap().dimension,
            Some(TextureViewDimension::Cube)
        );
        assert_eq!(image.asset_usage, RenderAssetUsages::RENDER_WORLD);
        assert_eq!(image.data.unwrap().len(), 6 * 8 * 8 * 8);
    }

    #[cfg(feature = "equirectangular_cubemap")]
    #[test]
    fn invalid_face_size_fails_loading() {
        let error = load_with_settings(
            r#"
            asset_usage: RenderAssetUsages("RENDER_WORLD"),
            cubemap_face_size: Some(0),
        "#,
        )
        .unwrap_err();
        assert!(
            error.contains("cubemap face size must be between 1 and"),
            "{error}"
        );
    }

    #[cfg(not(feature = "equirectangular_cubemap"))]
    #[test]
    fn cubemap_without_the_feature_fails_loading() {
        let error = load_with_settings(
            r#"
            asset_usage: RenderAssetUsages("RENDER_WORLD"),
            cubemap_face_size: Some(8),
        "#,
        )
        .unwrap_err();
        assert!(
            error.contains("requires the `equirectangular_cubemap` feature"),
            "{error}"
        );
    }

    #[cfg(all(
        feature = "compressed_image_saver",
        feature = "equirectangular_cubemap"
    ))]
    #[test]
    fn metadata_processes_and_reloads_a_compressed_cubemap() {
        use crate::{CompressedImageFormats, ImageLoader};
        use bevy_asset::{
            io::memory::MemoryAssetWriter,
            processor::{AssetProcessor, FileTransactionLogFactory},
            AssetMode,
        };

        let source = source_with_meta(
            r#"(
            meta_format_version: "1.0",
            asset: Process(
                processor: "LoadTransformAndSave<HdrTextureLoader, IdentityAssetTransformer<Image>, CompressedImageSaver>",
                settings: (
                    loader_settings: (
                        asset_usage: RenderAssetUsages("RENDER_WORLD"),
                        cubemap_face_size: Some(8),
                    ),
                    transformer_settings: (),
                    saver_settings: (),
                ),
            ),
        )"#,
        );
        let processed = Dir::default();
        let source_writer = source.clone();
        let processed_reader = processed.clone();
        let processed_writer = processed.clone();
        let mut app = App::new();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: source.clone(),
                })
            })
            .with_writer(move |_| {
                Some(Box::new(MemoryAssetWriter {
                    root: source_writer.clone(),
                }))
            })
            .with_processed_reader(move || {
                Box::new(MemoryAssetReader {
                    root: processed_reader.clone(),
                })
            })
            .with_processed_writer(move |_| {
                Some(Box::new(MemoryAssetWriter {
                    root: processed_writer.clone(),
                }))
            }),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin {
                mode: AssetMode::Processed,
                use_asset_processor_override: Some(true),
                watch_for_changes_override: Some(false),
                ..Default::default()
            },
            crate::ImagePlugin::default(),
        ))
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::all()));

        let log_path = std::env::temp_dir().join(format!(
            "bevy-hdr-cubemap-{}-{}.log",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        app.world()
            .resource::<AssetProcessor>()
            .data()
            .set_log_factory(Box::new(FileTransactionLogFactory {
                file_path: log_path.clone(),
            }))
            .unwrap();

        let image = load_image(&mut app).unwrap();
        assert_eq!(
            image.texture_descriptor.size,
            Extent3d {
                width: 8,
                height: 8,
                depth_or_array_layers: 6,
            }
        );
        assert!(matches!(
            image.texture_descriptor.format,
            TextureFormat::Bc6hRgbUfloat
                | TextureFormat::Astc {
                    channel: wgpu_types::AstcChannel::Hdr,
                    ..
                }
        ));
        assert_eq!(image.texture_descriptor.mip_level_count, 4);
        assert_eq!(
            image.texture_view_descriptor.unwrap().dimension,
            Some(TextureViewDimension::Cube)
        );
        assert_eq!(image.asset_usage, RenderAssetUsages::RENDER_WORLD);
        let file = processed.get_asset(Path::new("environment.hdr")).unwrap();
        let reader = ktx2::Reader::new(file.value()).unwrap();
        assert_eq!(reader.header().face_count, 6);
        assert_eq!(reader.header().level_count, 4);
        assert_eq!(reader.header().pixel_width, 8);
        let meta = processed
            .get_metadata(Path::new("environment.hdr"))
            .unwrap();
        let meta = str::from_utf8(meta.value()).unwrap();
        assert!(
            meta.contains("bevy_image::image_loader::ImageLoader"),
            "{meta}"
        );
        assert!(meta.contains("Ktx2"), "{meta}");
        let _ = std::fs::remove_file(log_path);
    }
}
