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

use crate::{render_asset::RenderAssetPlugin, RenderStage};
use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;
use bevy_ecs::prelude::*;

// TODO: replace Texture names with Image names?
pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }

        app.add_plugin(RenderAssetPlugin::<Image>::default())
            .add_asset::<Image>();

        let render_app = app.sub_app_mut(0);
        render_app
            .init_resource::<TextureCache>()
            .add_system_to_stage(RenderStage::Cleanup, update_texture_cache_system.system());
    }
}

pub trait BevyDefault {
    fn bevy_default() -> Self;
}

impl BevyDefault for wgpu::TextureFormat {
    fn bevy_default() -> Self {
        if cfg!(target_os = "android") {
            // Bgra8UnormSrgb texture missing on some Android devices
            wgpu::TextureFormat::Rgba8UnormSrgb
        } else {
            wgpu::TextureFormat::Bgra8UnormSrgb
        }
    }
}
