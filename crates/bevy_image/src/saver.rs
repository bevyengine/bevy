use std::io::Cursor;

use bevy_asset::{saver::AssetSaver, AssetPath, AsyncWriteExt};
use bevy_reflect::TypePath;
use image::{write_buffer_with_format, ExtendedColorType};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wgpu_types::TextureFormat;

use crate::{Image, ImageFormat, ImageFormatSetting, ImageLoader, ImageLoaderSettings};

/// [`AssetSaver`] for images that can be saved by the `image` crate.
///
/// Unlike `CompressedImageSaver`, this does not attempt to do any "texture optimization", like
/// compression (though some file formats intrinsically perform some compression, e.g., JPEG).
///
/// Some file formats do not support all texture formats (e.g., PNG does not support
/// [`TextureFormat::Rg8Unorm`]). In some cases, [`ImageSaver`] will convert the image to allow
/// writing as the requested file format.
#[derive(Clone, TypePath)]
pub struct ImageSaver;

impl AssetSaver for ImageSaver {
    type Asset = Image;
    type Error = SaveImageError;
    type OutputLoader = ImageLoader;
    type Settings = ImageSaverSettings;

    async fn save(
        &self,
        _writer: &mut bevy_asset::io::Writer,
        asset: bevy_asset::saver::SavedAsset<'_, '_, Self::Asset>,
        settings: &Self::Settings,
        asset_path: AssetPath<'_>,
    ) -> Result<ImageLoaderSettings, Self::Error> {
        let format = match settings.format {
            SaveImageFormatSetting::Format(format) => format,
            SaveImageFormatSetting::FromExtension => match asset_path.get_extension() {
                None => return Err(SaveImageError::MissingExtension(asset_path.into_owned())),
                Some(extension) => ImageFormat::from_extension(extension)
                    .ok_or_else(|| SaveImageError::UnknownExtension(extension.to_owned()))?,
            },
        };

        let Some(_asset_data) = asset.data.as_ref() else {
            return Err(SaveImageError::ImageMissingData);
        };

        // TODO: Consider supporting more formats here!
        let (image_crate_format, color_type, is_srgb): (_, ExtendedColorType, _) = match format {
            #[cfg(feature = "png")]
            ImageFormat::Png => match asset.texture_descriptor.format {
                TextureFormat::R8Unorm => (image::ImageFormat::Png, ExtendedColorType::L8, false),
                TextureFormat::Rgba8Unorm => {
                    (image::ImageFormat::Png, ExtendedColorType::Rgba8, false)
                }
                TextureFormat::Rgba8UnormSrgb => {
                    (image::ImageFormat::Png, ExtendedColorType::Rgba8, true)
                }
                _ => {
                    return Err(SaveImageError::UnsupportedSaveColorTypeForFormat(
                        ImageFormat::Png,
                        asset.texture_descriptor.format,
                    ))
                }
            },
            // FIXME: https://github.com/rust-lang/rust/issues/129031
            #[expect(
                clippy::allow_attributes,
                reason = "`unreachable_patterns` may not always lint"
            )]
            #[allow(
                unreachable_patterns,
                reason = "The wildcard pattern will be unreachable if only save-able formats are enabled"
            )]
            _ => return Err(SaveImageError::UnsupportedFormat(format)),
        };

        #[expect(clippy::allow_attributes, reason = "this lint only sometimes lints")]
        #[allow(
            unreachable_code,
            reason = "this code is unreachable if none of the supported save formats are enabled"
        )]
        let mut bytes = vec![];
        write_buffer_with_format(
            &mut Cursor::new(&mut bytes),
            _asset_data,
            asset.width(),
            asset.height(),
            color_type,
            image_crate_format,
        )?;

        _writer.write_all(&bytes).await?;

        Ok(ImageLoaderSettings {
            format: ImageFormatSetting::Format(format),
            // Passing in the original texture format breaks things. For example, PNG will save R8
            // data as RGBA8 data: if we later try to load as R8, we get 4 times as many pixels!
            texture_format: None,
            is_srgb,
            sampler: asset.sampler.clone(),
            asset_usage: asset.asset_usage,
            array_layout: None,
        })
    }
}

/// Settings for how to save an image.
#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct ImageSaverSettings {
    /// Defines the file format that the image will be saved as.
    pub format: SaveImageFormatSetting,
}

/// The setting for how to choose which file-format to use.
#[derive(Serialize, Deserialize, Default, Clone, Copy, Debug)]
pub enum SaveImageFormatSetting {
    /// The file format to write will be deduced from the file path being written to.
    #[default]
    FromExtension,
    /// This is the explicit file format being written.
    Format(ImageFormat),
}

/// An error while saving an image.
#[derive(Error, Debug)]
pub enum SaveImageError {
    /// Cannot deduce file format from extension because there is no extension.
    #[error("SaveImageFormatSetting::FromExtension was set, but the asset path \"{0}\" has no extension")]
    MissingExtension(AssetPath<'static>),
    /// Cannot deduce file format from extension since this extension is unknown. Holds the
    /// extension that could not be matched.
    #[error("could not determine asset format for extension \"{0}\"")]
    UnknownExtension(String),
    /// [`Image::data`] is [`None`], so there is no data to save. See
    /// [`RenderAssetUsages`](bevy_asset::RenderAssetUsages) for more.
    #[error("the provided image does not contain any pixel data. Its data may live on the GPU (which we can't save out) due to `RenderAssetUsages`")]
    ImageMissingData,
    /// The image saver doesn't support the file format being requested.
    #[error("the requested file format {0:?} is not supported for saving")]
    UnsupportedFormat(ImageFormat),
    /// The image saver doesn't support the texture format of the image data for the image format.
    #[error("the image uses a texture format \"{1:?}\" that is not supported for saving by the image format \"{0:?}\"")]
    UnsupportedSaveColorTypeForFormat(ImageFormat, TextureFormat),
    /// The [`image`] crate returned an error.
    #[error(transparent)]
    ImageError(#[from] image::ImageError),
    /// Writing the bytes returned an error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use bevy_app::{App, TaskPoolPlugin};
    use bevy_asset::{
        io::{
            memory::{Dir, MemoryAssetReader, MemoryAssetWriter},
            AssetSourceBuilder, AssetSourceId,
        },
        saver::{save_using_saver, SavedAsset},
        AssetApp, AssetPath, AssetPlugin, AssetServer, DirectAssetAccessExt, RenderAssetUsages,
    };
    use bevy_color::Srgba;
    use bevy_ecs::world::World;
    use bevy_math::UVec2;
    use bevy_platform::future::block_on;
    use wgpu_types::TextureFormat;

    use crate::{
        CompressedImageFormats, Image, ImageLoader, ImageSaver, ImageSaverSettings,
        TextureFormatPixelInfo,
    };

    fn create_app() -> (App, Dir) {
        let mut app = App::new();
        let dir = Dir::default();
        let dir_clone_1 = dir.clone();
        let dir_clone_2 = dir.clone();
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader {
                    root: dir_clone_1.clone(),
                })
            })
            .with_writer(move |_| {
                Some(Box::new(MemoryAssetWriter {
                    root: dir_clone_2.clone(),
                }))
            }),
        )
        .add_plugins((
            TaskPoolPlugin::default(),
            AssetPlugin {
                watch_for_changes_override: Some(false),
                use_asset_processor_override: Some(false),
                ..Default::default()
            },
        ))
        .init_asset::<Image>()
        .register_asset_loader(ImageLoader::new(CompressedImageFormats::empty()));

        (app, dir)
    }

    fn run_app_until(app: &mut App, mut predicate: impl FnMut(&mut World) -> Option<()>) {
        const LARGE_ITERATION_COUNT: usize = 10000;
        for _ in 0..LARGE_ITERATION_COUNT {
            app.update();
            if predicate(app.world_mut()).is_some() {
                return;
            }
        }

        panic!("Ran out of loops to return `Some` from `predicate`");
    }

    #[expect(clippy::allow_attributes, reason = "only occasionally unused")]
    #[allow(unused, reason = "only used for feature-flagged image formats")]
    fn roundtrip_for_type(file_name: &str, color_type: TextureFormat) {
        let (mut app, dir) = create_app();
        let asset_server = app.world().resource::<AssetServer>().clone();

        let asset_path = AssetPath::from_path(Path::new(file_name));

        const WIDTH: u32 = 5;
        let mut image = Image::new(
            wgpu_types::Extent3d {
                width: WIDTH,
                height: WIDTH,
                depth_or_array_layers: 1,
            },
            wgpu_types::TextureDimension::D2,
            vec![0; color_type.pixel_size().unwrap() * WIDTH as usize * WIDTH as usize],
            color_type,
            RenderAssetUsages::all(),
        );
        for y in 0..WIDTH {
            for x in 0..WIDTH {
                image
                    .set_color_at(
                        x,
                        y,
                        Srgba::new(
                            (x + 1) as f32 / WIDTH as f32,
                            (y + 1) as f32 / WIDTH as f32,
                            (x + y + 2) as f32 / (2 * WIDTH) as f32,
                            1.0,
                        )
                        .into(),
                    )
                    .unwrap();
            }
        }

        {
            let asset_server = asset_server.clone();
            let image = image.clone();
            let asset_path = asset_path.clone_owned();
            block_on(async move {
                let saved_asset = SavedAsset::from_asset(&image);
                save_using_saver(
                    asset_server,
                    &ImageSaver,
                    &asset_path,
                    saved_asset,
                    &ImageSaverSettings::default(),
                )
                .await
            })
            .unwrap();
        }

        assert!(dir.get_asset(asset_path.path()).is_some());

        let handle = asset_server.load::<Image>(asset_path);
        run_app_until(&mut app, |_| asset_server.is_loaded(&handle).then_some(()));

        let loaded_image = app.world().get_asset(handle.id()).unwrap();

        assert_eq!(loaded_image.size(), UVec2::new(WIDTH, WIDTH));
        let compare_images = 'compare_images: {
            for y in 0..WIDTH {
                for x in 0..WIDTH {
                    if image.get_color_at(x, y).unwrap() != loaded_image.get_color_at(x, y).unwrap()
                    {
                        break 'compare_images Err((x, y));
                    }
                }
            }
            Ok(())
        };

        if let Err((x, y)) = compare_images {
            fn image_to_string(image: &Image) -> String {
                (0..WIDTH)
                    .map(|y| {
                        (0..WIDTH)
                            .map(|x| {
                                let color = image.get_color_at(x, y).unwrap().to_srgba();
                                format!(
                                    "({},{},{})",
                                    (color.red * 255.0) as u32,
                                    (color.green * 255.0) as u32,
                                    (color.blue * 255.0) as u32,
                                )
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            panic!(
                "Mismatch in color at ({x}, {y})\nleft:\n{}\nright:\n{}",
                image_to_string(loaded_image),
                image_to_string(&image)
            );
        }
    }

    #[cfg(feature = "png")]
    mod png_tests {
        use super::*;

        #[test]
        fn roundtrip_png_r8_unorm() {
            roundtrip_for_type("image.png", TextureFormat::R8Unorm);
        }
        #[test]
        fn roundtrip_png_rgba8_unorm_srgb() {
            roundtrip_for_type("image.png", TextureFormat::Rgba8UnormSrgb);
        }
        #[test]
        fn roundtrip_png_rgba8_unorm() {
            roundtrip_for_type("image.png", TextureFormat::Rgba8Unorm);
        }
    }
}
