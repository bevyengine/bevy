#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "dds")]
mod dds;
#[cfg(feature = "hdr")]
mod hdr_texture_loader;
#[allow(clippy::module_inception)]
mod image;
mod image_texture_loader;
#[cfg(feature = "ktx2")]
mod ktx2;
mod texture_cache;

pub(crate) mod image_texture_conversion;

pub use self::image::*;
#[cfg(feature = "ktx2")]
pub use self::ktx2::*;
#[cfg(feature = "dds")]
pub use dds::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;

pub use image_texture_loader::*;
pub use texture_cache::*;

use crate::{
    render_asset::{PrepareAssetLabel, RenderAssetPlugin},
    renderer::RenderDevice,
    RenderApp, RenderStage,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Assets};

// TODO: replace Texture names with Image names?
/// Adds the [`Image`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(any(
            feature = "png",
            feature = "dds",
            feature = "tga",
            feature = "jpeg",
            feature = "bmp",
            feature = "basis-universal",
            feature = "ktx2",
        ))]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }

        #[cfg(feature = "hdr")]
        {
            app.init_asset_loader::<HdrTextureLoader>();
        }

        app.add_plugin(RenderAssetPlugin::<Image>::with_prepare_asset_label(
            PrepareAssetLabel::PreAssetPrepare,
        ))
        .add_asset::<Image>();
        app.world
            .resource_mut::<Assets<Image>>()
            .set_untracked(DEFAULT_IMAGE_HANDLE, Image::default());

        let default_sampler = app
            .world
            .get_resource_or_insert_with(ImageSettings::default)
            .default_sampler
            .clone();
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let default_sampler = {
                let device = render_app.world.resource::<RenderDevice>();
                device.create_sampler(&default_sampler)
            };
            render_app
                .insert_resource(DefaultImageSampler(default_sampler))
                .init_resource::<TextureCache>()
                .add_system_to_stage(RenderStage::Cleanup, update_texture_cache_system);
        }
    }
}

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for wgpu::TextureFormat {
    fn bevy_default() -> Self {
        if cfg!(target_os = "android") || cfg!(target_arch = "wasm32") {
            // Bgra8UnormSrgb texture missing on some Android devices
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        }
    }
}
