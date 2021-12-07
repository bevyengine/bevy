#[cfg(feature = "hdr")]
mod hdr_texture_loader;
#[allow(clippy::module_inception)]
mod image;
mod image_texture_loader;
mod texture_cache;

pub(crate) mod image_texture_conversion;

pub use self::image::*;
#[cfg(feature = "hdr")]
pub use hdr_texture_loader::*;
pub use image_texture_loader::*;
pub use texture_cache::*;

use crate::{render_asset::RenderAssetPlugin, RenderApp, RenderStage};
use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Assets};

// TODO: replace Texture names with Image names?
/// Adds the [`Image`] as an asset and makes sure that they are extracted and prepared for the GPU.
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }

        app.add_plugin(RenderAssetPlugin::<Image>::default())
            .add_asset::<Image>();
        app.world
            .get_resource_mut::<Assets<Image>>()
            .unwrap()
            .set_untracked(DEFAULT_IMAGE_HANDLE, Image::default());

        app.sub_app(RenderApp)
            .init_resource::<TextureCache>()
            .add_system_to_stage(RenderStage::Cleanup, update_texture_cache_system);
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
