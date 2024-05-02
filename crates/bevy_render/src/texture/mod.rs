#[cfg(feature = "basis-universal")]
mod basis;
#[cfg(feature = "basis-universal")]
mod compressed_image_saver;
#[cfg(feature = "dds")]
mod dds;
#[cfg(feature = "exr")]
mod exr_texture_loader;
mod fallback_image;
#[cfg(feature = "hdr")]
mod hdr_texture_loader;
#[allow(clippy::module_inception)]
mod image;
mod image_loader;
#[cfg(feature = "ktx2")]
mod ktx2;
mod texture_attachment;
mod texture_cache;

pub(crate) mod image_texture_conversion;

use std::fmt::Display;

pub use self::image::*;
#[cfg(feature = "ktx2")]
pub use self::ktx2::*;
use bevy_reflect::Reflect;
#[cfg(feature = "dds")]
pub use dds::*;
#[cfg(feature = "exr")]
pub use exr_texture_loader::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;

#[cfg(feature = "basis-universal")]
pub use compressed_image_saver::*;
pub use fallback_image::*;
pub use image_loader::*;
pub use texture_attachment::*;
pub use texture_cache::*;
use wgpu::TextureFormat;

use crate::{
    render_asset::RenderAssetPlugin, renderer::RenderDevice, Render, RenderApp, RenderSet,
};
use bevy_app::{App, Plugin};
use bevy_asset::{AssetApp, Assets, Handle};
use bevy_ecs::prelude::*;

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

        app.world_mut()
            .resource_mut::<Assets<Image>>()
            .insert(&Handle::default(), Image::default());
        #[cfg(feature = "basis-universal")]
        if let Some(processor) = app
            .world()
            .get_resource::<bevy_asset::processor::AssetProcessor>()
        {
            processor.register_processor::<bevy_asset::processor::LoadAndSave<ImageLoader, CompressedImageSaver>>(
                CompressedImageSaver.into(),
            );
            processor
                .set_default_processor::<bevy_asset::processor::LoadAndSave<ImageLoader, CompressedImageSaver>>("png");
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

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for TextureFormat {
    fn bevy_default() -> Self {
        TextureFormat::Rgba8UnormSrgb
    }
}

/// Defines the subset of [`TextureFormat`](TextureFormat)s suitable for rendering
///
/// Not all of these formats may be available on all devices. See [`ViewTargetFormat::required_features`].
#[derive(Copy, Clone, Debug, Default, Hash, Eq, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
pub enum ViewTargetFormat {
    // BEFORE_MERGE: What about depth or stencil formats

    // NOTE: The formats are pulled from https://gpuweb.github.io/gpuweb/#plain-color-formats
    //       If you want to add more of the to this enum they'll have to, support being a `RENDER_ATTACHMENT`, be `blendable`,
    //       allow multisampling and have their `GPUTextureSampleType` be float.
    //       These requirements may be relaxed in the future.
    /// Red channel only. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    R8Unorm,

    /// Red and green channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rg8Unorm,

    /// Red, green, blue, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Rgba8Unorm,

    /// The default texture format used by bevy. It being clamped means view targets using it won't support effects like bloom.
    /// Red, green, blue, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    #[default]
    Rgba8UnormSrgb,

    /// Blue, green, red, and alpha channels. 8 bit integer per channel. [0, 255] converted to/from float [0, 1] in shader.
    Bgra8Unorm,

    /// Blue, green, red, and alpha channels. 8 bit integer per channel. Srgb-color [0, 255] converted to/from linear-color float [0, 1] in shader.
    Bgra8UnormSrgb,

    /// Red channel only. 16 bit float per channel. Float in shader.
    R16Float,

    /// Red and green channels. 16 bit float per channel. Float in shader.
    Rg16Float,

    // BEFORE_MERGE: Figure these out. Since they're native only they won't be in the table.
    //
    // R16Unorm
    // R16Snorm
    // Rg16Unorm
    // Rg16Snorm
    // Rgba16Unorm
    // Rgba16Snorm
    /// A common texture format for color in HDR textures and the default texture format used by bevy when it needs unclamped texture format.
    ///
    /// Red, green, blue, and alpha channels. 16 bit float per channel. Float in shader.
    Rgba16Float,

    /// A cheaper to render unclamped texture format than [`ViewTargetFormat::UNCLAMPED_DEFAULT`]. Might encounter to precision issues.
    ///
    /// Can only be used if the render device supports the [`RG11B10UFLOAT_RENDERABLE`](wgpu::Features::RG11B10UFLOAT_RENDERABLE)
    /// feature.
    ///
    /// Red, green, and blue channels. 11 bit float with no sign bit for RG channels. 10 bit float with no sign bit for blue channel. Float in shader.
    Rb11b10Float,

    /// Red, green, blue, and alpha channels. 10 bit integer for RGB channels, 2 bit integer for alpha channel. [0, 1023] ([0, 3] for alpha) converted to/from float [0, 1] in shader.
    Rgb10a2Unorm,
    // TODO: These may be supported in the future. See https://github.com/gpuweb/gpuweb/issues/3556.
    //
    // If/When this happen they'll require the `float32-blendable` (pressumed name) + `float32-filterable`
    //
    // /// Red channel only. 32 bit float per channel. Float in shader.
    // R32Float,
    // /// Red and green channels. 32 bit float per channel. Float in shader.
    // Rg32Float,
    // /// Red, green, blue, and alpha channels. 32 bit float per channel. Float in shader.
    // Rgba32Float,
}

impl ViewTargetFormat {
    pub const DEFAULT: Self = Self::Rgba8UnormSrgb;
    pub const UNCLAMPED_DEFAULT: Self = Self::Rgba16Float;

    /// Defines what [`features`](wgpu::Features) the render device needs to support for the format to
    /// be available.
    ///
    /// Currently the only [`ViewTargetFormat`] that requires any features is [`Rb11b10Float`](ViewTargetFormat::Rb11b10Float).
    pub const fn required_features(&self) -> wgpu::Features {
        match self {
            ViewTargetFormat::R8Unorm
            | ViewTargetFormat::Rg8Unorm
            | ViewTargetFormat::Bgra8Unorm
            | ViewTargetFormat::Rgba8Unorm
            | ViewTargetFormat::Rgba8UnormSrgb
            | ViewTargetFormat::Bgra8UnormSrgb
            | ViewTargetFormat::Rgba16Float
            | ViewTargetFormat::R16Float
            | ViewTargetFormat::Rg16Float
            | ViewTargetFormat::Rgb10a2Unorm => wgpu::Features::empty(),
            ViewTargetFormat::Rb11b10Float => wgpu::Features::RG11B10UFLOAT_RENDERABLE,
        }
    }

    /// Unclamped [`ViewTargetFormat`]s are ones whose float values in shader won't be clamped between 0.0 and 1.0.
    ///
    /// Bloom and other such effects only work with these formats.
    pub const fn is_unclamped(&self) -> bool {
        // BEFORE_MERGE: Heavily bikeshed the name

        match self {
            ViewTargetFormat::R16Float
            | ViewTargetFormat::Rg16Float
            | ViewTargetFormat::Rgba16Float
            | ViewTargetFormat::Rb11b10Float => true,
            ViewTargetFormat::R8Unorm
            | ViewTargetFormat::Rg8Unorm
            | ViewTargetFormat::Rgba8Unorm
            | ViewTargetFormat::Bgra8Unorm
            | ViewTargetFormat::Rgba8UnormSrgb
            | ViewTargetFormat::Bgra8UnormSrgb
            | ViewTargetFormat::Rgb10a2Unorm => false,
        }
    }
}

impl From<ViewTargetFormat> for TextureFormat {
    fn from(value: ViewTargetFormat) -> Self {
        match value {
            ViewTargetFormat::Rgba16Float => TextureFormat::Rgba16Float,
            ViewTargetFormat::Rb11b10Float => TextureFormat::Rg11b10Float,
            ViewTargetFormat::Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
            ViewTargetFormat::Rgba8Unorm => TextureFormat::Rgba8Unorm,
            ViewTargetFormat::R8Unorm => TextureFormat::R8Unorm,
            ViewTargetFormat::Rg8Unorm => TextureFormat::Rg8Unorm,
            ViewTargetFormat::Bgra8Unorm => TextureFormat::Bgra8Unorm,
            ViewTargetFormat::Bgra8UnormSrgb => TextureFormat::Bgra8UnormSrgb,
            ViewTargetFormat::R16Float => TextureFormat::R16Float,
            ViewTargetFormat::Rg16Float => TextureFormat::Rg16Float,
            ViewTargetFormat::Rgb10a2Unorm => TextureFormat::Rgb10a2Unorm,
        }
    }
}

/// The error type returned when converting a [`TextureFormat`] into [`ViewTargetFormat`] fails.
#[derive(Debug, Clone, Copy)]
pub struct TryFromTextureFormatError {}

impl Display for TryFromTextureFormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("could not convert texture format into view target format")
    }
}

impl TryFrom<TextureFormat> for ViewTargetFormat {
    type Error = TryFromTextureFormatError;

    fn try_from(value: TextureFormat) -> Result<Self, Self::Error> {
        match value {
            TextureFormat::Rgba16Float => Ok(ViewTargetFormat::Rgba16Float),
            TextureFormat::Rg11b10Float => Ok(ViewTargetFormat::Rb11b10Float),
            TextureFormat::Rgba8UnormSrgb => Ok(ViewTargetFormat::Rgba8UnormSrgb),
            TextureFormat::Rgba8Unorm => Ok(ViewTargetFormat::Rgba8Unorm),
            TextureFormat::R8Unorm => Ok(ViewTargetFormat::R8Unorm),
            TextureFormat::Rg8Unorm => Ok(ViewTargetFormat::Rg8Unorm),
            TextureFormat::Bgra8Unorm => Ok(ViewTargetFormat::Bgra8Unorm),
            TextureFormat::Bgra8UnormSrgb => Ok(ViewTargetFormat::Bgra8UnormSrgb),
            TextureFormat::R16Float => Ok(ViewTargetFormat::R16Float),
            TextureFormat::Rg16Float => Ok(ViewTargetFormat::Rg16Float),
            TextureFormat::Rgb10a2Unorm => Ok(ViewTargetFormat::Rgb10a2Unorm),
            _ => Err(TryFromTextureFormatError {}),
        }
    }
}
