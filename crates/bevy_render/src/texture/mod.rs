mod fallback_image;
mod gpu_image;
mod image_loader;
mod texture_attachment;
mod texture_cache;

use bevy_image::HdrTextureLoader;
pub use gpu_image::*;

pub use crate::render_resource::DefaultImageSampler;
pub use bevy_image::{
    BevyDefault, CompressedImageFormats, Image, ImageAddressMode, ImageFilterMode, ImageFormat,
    ImageSampler, ImageSamplerDescriptor, ImageType, IntoDynamicImageError, TextureError,
    TextureFormatPixelInfo,
};
pub use fallback_image::*;
pub use image_loader::*;
pub use texture_attachment::*;
pub use texture_cache::*;

use crate::{
    render_asset::RenderAssetPlugin, renderer::RenderDevice, Render, RenderApp, RenderSet,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AssetApp, Assets, Handle};
use bevy_ecs::prelude::*;

/// A handle to a 1 x 1 transparent white image.
///
/// Like [`Handle<Image>::default`], this is a handle to a fallback image asset.
/// While that handle points to an opaque white 1 x 1 image, this handle points to a transparent 1 x 1 white image.
// Number randomly selected by fair WolframAlpha query. Totally arbitrary.
pub const TRANSPARENT_IMAGE_HANDLE: Handle<Image> =
    Handle::weak_from_u128(154728948001857810431816125397303024160);

// TODO: replace Texture names with Image names?
/// Adds the [`Image`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct ImagePlugin {
    /// The default image sampler to use when [`ImageSampler`] is set to `Default`.
    pub default_sampler: ImageSamplerDescriptor,
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
            default_sampler: ImageSamplerDescriptor::linear(),
        }
    }

    /// Creates image settings with nearest sampling by default.
    pub fn default_nearest() -> ImagePlugin {
        ImagePlugin {
            default_sampler: ImageSamplerDescriptor::nearest(),
        }
    }
}

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "exr")]
        {
            app.init_asset_loader::<ExrTextureLoader>();
        }

        #[cfg(feature = "hdr")]
        {
            app.init_asset_loader::<HdrTextureLoader>();
        }

        app.add_plugins(RenderAssetPlugin::<GpuImage>::default())
            .register_type::<Image>()
            .init_asset::<Image>()
            .register_asset_reflect::<Image>();

        let mut image_assets = app.world_mut().resource_mut::<Assets<Image>>();

        image_assets.insert(&Handle::default(), Image::default());
        image_assets.insert(&TRANSPARENT_IMAGE_HANDLE, Image::transparent());

        #[cfg(feature = "basis-universal")]
        if let Some(processor) = app
            .world()
            .get_resource::<bevy_asset::processor::AssetProcessor>()
        {
            processor.register_processor::<bevy_asset::processor::LoadTransformAndSave<
                ImageLoader,
                bevy_asset::transformer::IdentityAssetTransformer<Image>,
                CompressedImageSaver,
            >>(CompressedImageSaver.into());
            processor.set_default_processor::<bevy_asset::processor::LoadTransformAndSave<
                ImageLoader,
                bevy_asset::transformer::IdentityAssetTransformer<Image>,
                CompressedImageSaver,
            >>("png");
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TextureCache>().add_systems(
                Render,
                update_texture_cache_system.in_set(RenderSet::Cleanup),
            );
        }

        #[cfg(any(
            feature = "png",
            feature = "dds",
            feature = "tga",
            feature = "jpeg",
            feature = "bmp",
            feature = "basis-universal",
            feature = "ktx2",
            feature = "webp",
            feature = "pnm"
        ))]
        app.preregister_asset_loader::<ImageLoader>(IMG_FILE_EXTENSIONS);
    }

    fn finish(&self, app: &mut App) {
        #[cfg(any(
            feature = "png",
            feature = "dds",
            feature = "tga",
            feature = "jpeg",
            feature = "bmp",
            feature = "basis-universal",
            feature = "ktx2",
            feature = "webp",
            feature = "pnm"
        ))]
        {
            app.init_asset_loader::<ImageLoader>();
        }

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let default_sampler = {
                let device = render_app.world().resource::<RenderDevice>();
                device.create_sampler(&self.default_sampler.as_wgpu())
            };
            render_app
                .insert_resource(DefaultImageSampler(default_sampler))
                .init_resource::<FallbackImage>()
                .init_resource::<FallbackImageZero>()
                .init_resource::<FallbackImageCubemap>()
                .init_resource::<FallbackImageFormatMsaaCache>();
        }
    }
}
