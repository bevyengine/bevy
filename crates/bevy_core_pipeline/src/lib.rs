#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

pub mod auto_exposure;
pub mod blit;
pub mod bloom;
pub mod core_2d;
pub mod core_3d;
pub mod deferred;
pub mod dof;
pub mod experimental;
pub mod motion_blur;
pub mod msaa_writeback;
pub mod oit;
pub mod post_process;
pub mod prepass;
pub mod tonemapping;
pub mod upscaling;

pub use fullscreen_vertex_shader::FullscreenShader;
pub use skybox::Skybox;

mod fullscreen_vertex_shader;
mod skybox;

use crate::{
    blit::BlitPlugin, bloom::BloomPlugin, core_2d::Core2dPlugin, core_3d::Core3dPlugin,
    deferred::copy_lighting_id::CopyDeferredLightingIdPlugin, dof::DepthOfFieldPlugin,
    experimental::mip_generation::MipGenerationPlugin, motion_blur::MotionBlurPlugin,
    msaa_writeback::MsaaWritebackPlugin, post_process::PostProcessingPlugin,
    tonemapping::TonemappingPlugin, upscaling::UpscalingPlugin,
};
use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_render::RenderApp;
use oit::OrderIndependentTransparencyPlugin;

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fullscreen_vertex_shader/fullscreen.wgsl");

        app.add_plugins((Core2dPlugin, Core3dPlugin, CopyDeferredLightingIdPlugin))
            .add_plugins((
                BlitPlugin,
                MsaaWritebackPlugin,
                TonemappingPlugin,
                UpscalingPlugin,
                BloomPlugin,
                MotionBlurPlugin,
                DepthOfFieldPlugin,
                PostProcessingPlugin,
                OrderIndependentTransparencyPlugin,
                MipGenerationPlugin,
            ));
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<FullscreenShader>();
    }
}
