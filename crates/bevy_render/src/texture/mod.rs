mod fallback_image;
mod gpu_image;
mod manual_texture_view;
mod texture_attachment;
mod texture_cache;

pub use crate::render_resource::DefaultImageSampler;
use bevy_image::{CompressedImageFormatSupport, CompressedImageFormats, ImageLoader, ImagePlugin};
pub use fallback_image::*;
pub use gpu_image::*;
pub use manual_texture_view::*;
pub use texture_attachment::*;
pub use texture_cache::*;

use crate::{
    extract_resource::ExtractResourcePlugin, render_asset::RenderAssetPlugin,
    renderer::RenderDevice, Render, RenderApp, RenderSystems,
};
use bevy_app::{App, Plugin};
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;
use tracing::warn;

#[derive(Default)]
pub struct TexturePlugin;

impl Plugin for TexturePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            RenderAssetPlugin::<GpuImage>::default(),
            ExtractResourcePlugin::<ManualTextureViews>::default(),
        ))
        .init_resource::<ManualTextureViews>();
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<TextureCache>().add_systems(
                Render,
                update_texture_cache_system.in_set(RenderSystems::Cleanup),
            );
        }
    }

    fn finish(&self, app: &mut App) {
        if !ImageLoader::SUPPORTED_FORMATS.is_empty() {
            let supported_compressed_formats = if let Some(resource) =
                app.world().get_resource::<CompressedImageFormatSupport>()
            {
                resource.0
            } else {
                warn!("CompressedImageFormatSupport resource not found. It should either be initialized in finish() of \
                       RenderPlugin, or manually if not using the RenderPlugin or the WGPU backend.");
                CompressedImageFormats::NONE
            };

            app.register_asset_loader(ImageLoader::new(supported_compressed_formats));
        }
        let default_sampler = app.get_added_plugins::<ImagePlugin>()[0]
            .default_sampler
            .clone();

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let default_sampler = {
                let device = render_app.world().resource::<RenderDevice>();
                device.create_sampler(&default_sampler.as_wgpu())
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
