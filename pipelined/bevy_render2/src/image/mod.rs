pub use self::image::*;
use bevy_app::{App, Plugin};
use bevy_asset::AddAsset;
#[cfg(feature = "hdr")]
pub use hdr_image_loader::*;
pub use image_loader::*;

use crate::render_asset::RenderAssetPlugin;

#[cfg(feature = "hdr")]
mod hdr_image_loader;
#[allow(clippy::module_inception)]
mod image;
mod image_loader;

pub(crate) mod image_conversion;

pub struct ImagePlugin;

impl Plugin for ImagePlugin {
    fn build(&self, app: &mut App) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageLoader>();
        }

        app.add_plugin(RenderAssetPlugin::<Image>::default())
            .add_asset::<Image>();
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
