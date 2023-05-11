#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "dds")]
mod dds;
#[cfg(feature = "exr")]
mod exr_texture_loader;
mod fallback_image;
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
#[cfg(feature = "exr")]
pub use exr_texture_loader::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;

pub use fallback_image::*;
pub use image_texture_loader::*;
pub use texture_cache::*;

use crate::{
    render_asset::{PrepareAssetSet, RenderAssetPlugin},
    renderer::RenderDevice,
    Render, RenderApp, RenderSet,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Assets};
use bevy_ecs::prelude::*;

// TODO: replace Texture names with Image names?
/// Adds the [`Image`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct ImagePlugin {
    /// The default image sampler to use when [`ImageSampler`] is set to `Default`.
    pub default_sampler: wgpu::SamplerDescriptor<'static>,
}

impl Default for ImagePlugin {
    fn default() -> Self {
        ImagePlugin::default_linear()
    }
}

impl ImagePlugin {
    /// Creates image settings with linear sampling by default.
    pub fn default_linear() -> ImagePlugin {
        ImagePlugin {
            default_sampler: ImageSampler::linear_descriptor(),
        }
    }

    /// Creates image settings with nearest sampling by default.
    pub fn default_nearest() -> ImagePlugin {
        ImagePlugin {
            default_sampler: ImageSampler::nearest_descriptor(),
        }
    }
}

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

        #[cfg(feature = "exr")]
        {
            app.init_asset_loader::<ExrTextureLoader>();
        }

        #[cfg(feature = "hdr")]
        {
            app.init_asset_loader::<HdrTextureLoader>();
        }

        app.add_plugin(RenderAssetPlugin::<Image>::with_prepare_asset_set(
            PrepareAssetSet::PreAssetPrepare,
        ))
        .register_type::<Image>()
        .add_asset::<Image>()
        .register_asset_reflect::<Image>();
        app.world
            .resource_mut::<Assets<Image>>()
            .set_untracked(DEFAULT_IMAGE_HANDLE, Image::default());

        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TextureCache>().add_systems(
                Render,
                update_texture_cache_system.in_set(RenderSet::Cleanup),
            );
        }
    }

    fn finish(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            let default_sampler = {
                let device = render_app.world.resource::<RenderDevice>();
                device.create_sampler(&self.default_sampler.clone())
            };
            render_app
                .insert_resource(DefaultImageSampler(default_sampler))
                .init_resource::<FallbackImage>()
                .init_resource::<FallbackImageCubemap>()
                .init_resource::<FallbackImageMsaaCache>()
                .init_resource::<FallbackImageDepthCache>();
        }
    }
}

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for wgpu::TextureFormat {
    fn bevy_default() -> Self {
        wgpu::TextureFormat::Rgba8UnormSrgb
    }
}
