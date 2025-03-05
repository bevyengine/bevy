#![expect(missing_docs, reason = "Not all docs are written yet, see #3492.")]
#![forbid(unsafe_code)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub mod auto_exposure;
pub mod blit;
pub mod bloom;
pub mod contrast_adaptive_sharpening;
pub mod core_2d;
pub mod core_3d;
pub mod deferred;
pub mod dof;
pub mod experimental;
pub mod fullscreen_vertex_shader;
pub mod fxaa;
pub mod motion_blur;
pub mod msaa_writeback;
pub mod oit;
pub mod post_process;
pub mod prepass;
mod skybox;
pub mod smaa;
mod taa;
pub mod tonemapping;
pub mod upscaling;

pub use skybox::Skybox;

/// The core pipeline prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{core_2d::Camera2d, core_3d::Camera3d};
}

use crate::{
    blit::BlitPlugin,
    bloom::BloomPlugin,
    contrast_adaptive_sharpening::CasPlugin,
    core_2d::Core2dPlugin,
    core_3d::Core3dPlugin,
    deferred::copy_lighting_id::CopyDeferredLightingIdPlugin,
    dof::DepthOfFieldPlugin,
    experimental::mip_generation::MipGenerationPlugin,
    fullscreen_vertex_shader::FULLSCREEN_SHADER_HANDLE,
    fxaa::FxaaPlugin,
    motion_blur::MotionBlurPlugin,
    msaa_writeback::MsaaWritebackPlugin,
    post_process::PostProcessingPlugin,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    smaa::SmaaPlugin,
    tonemapping::TonemappingPlugin,
    upscaling::UpscalingPlugin,
};
use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_render::prelude::Shader;
use oit::OrderIndependentTransparencyPlugin;

#[derive(Default)]
pub struct CorePipelinePlugin;

impl Plugin for CorePipelinePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            FULLSCREEN_SHADER_HANDLE,
            "fullscreen_vertex_shader/fullscreen.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<DepthPrepass>()
            .register_type::<NormalPrepass>()
            .register_type::<MotionVectorPrepass>()
            .register_type::<DeferredPrepass>()
            .add_plugins((Core2dPlugin, Core3dPlugin, CopyDeferredLightingIdPlugin))
            .add_plugins((
                BlitPlugin,
                MsaaWritebackPlugin,
                TonemappingPlugin,
                UpscalingPlugin,
                BloomPlugin,
                FxaaPlugin,
                CasPlugin,
                MotionBlurPlugin,
                DepthOfFieldPlugin,
                SmaaPlugin,
                PostProcessingPlugin,
                OrderIndependentTransparencyPlugin,
                MipGenerationPlugin,
            ));
    }
}
