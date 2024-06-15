// FIXME(3492): remove once docs are ready
#![allow(missing_docs)]
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
pub mod fullscreen_vertex_shader;
pub mod fxaa;
pub mod motion_blur;
pub mod msaa_writeback;
pub mod prepass;
mod skybox;
pub mod smaa;
mod taa;
pub mod tonemapping;
pub mod upscaling;

pub use skybox::Skybox;

/// Experimental features that are not yet finished. Please report any issues you encounter!
///
/// Expect bugs, missing features, compatibility issues, low performance, and/or future breaking changes.
pub mod experimental {
    pub mod taa {
        pub use crate::taa::{
            TemporalAntiAliasBundle, TemporalAntiAliasNode, TemporalAntiAliasPlugin,
            TemporalAntiAliasSettings,
        };
    }
}

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        core_2d::{Camera2d, Camera2dBundle},
        core_3d::{Camera3d, Camera3dBundle},
    };
}

use crate::{
    blit::BlitPlugin,
    bloom::BloomPlugin,
    contrast_adaptive_sharpening::CASPlugin,
    core_2d::Core2dPlugin,
    core_3d::Core3dPlugin,
    deferred::copy_lighting_id::CopyDeferredLightingIdPlugin,
    dof::DepthOfFieldPlugin,
    fullscreen_vertex_shader::FULLSCREEN_SHADER_HANDLE,
    fxaa::FxaaPlugin,
    motion_blur::MotionBlurPlugin,
    msaa_writeback::MsaaWritebackPlugin,
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    smaa::SmaaPlugin,
    tonemapping::TonemappingPlugin,
    upscaling::UpscalingPlugin,
};
use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_render::prelude::Shader;

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
            .add_plugins((
                Core2dPlugin,
                Core3dPlugin,
                CopyDeferredLightingIdPlugin,
                BlitPlugin,
                MsaaWritebackPlugin,
                TonemappingPlugin,
                UpscalingPlugin,
                BloomPlugin,
                FxaaPlugin,
                CASPlugin,
                MotionBlurPlugin,
                DepthOfFieldPlugin,
                SmaaPlugin,
            ));
    }
}
