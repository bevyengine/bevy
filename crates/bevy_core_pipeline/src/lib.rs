#![allow(clippy::type_complexity)]

pub mod blit;
pub mod bloom;
pub mod clear_color;
pub mod contrast_adaptive_sharpening;
pub mod core_2d;
pub mod core_3d;
pub mod fullscreen_vertex_shader;
pub mod fxaa;
pub mod msaa_writeback;
pub mod prepass;
mod skybox;
mod taa;
pub mod tonemapping;
pub mod upscaling;

pub use skybox::Skybox;

/// Experimental features that are not yet finished. Please report any issues you encounter!
pub mod experimental {
    pub mod taa {
        pub use crate::taa::*;
    }
}

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        clear_color::ClearColor,
        core_2d::{Camera2d, Camera2dBundle},
        core_3d::{Camera3d, Camera3dBundle},
    };
}

use crate::{
    blit::BlitPlugin,
    bloom::BloomPlugin,
    clear_color::{ClearColor, ClearColorConfig},
    contrast_adaptive_sharpening::CASPlugin,
    core_2d::Core2dPlugin,
    core_3d::Core3dPlugin,
    fullscreen_vertex_shader::FULLSCREEN_SHADER_HANDLE,
    fxaa::FxaaPlugin,
    msaa_writeback::MsaaWritebackPlugin,
    prepass::{DepthPrepass, NormalPrepass},
    tonemapping::TonemappingPlugin,
    upscaling::UpscalingPlugin,
};
use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_render::{extract_resource::ExtractResourcePlugin, prelude::Shader};

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

        app.register_type::<ClearColor>()
            .register_type::<ClearColorConfig>()
            .register_type::<DepthPrepass>()
            .register_type::<NormalPrepass>()
            .init_resource::<ClearColor>()
            .add_plugins((
                ExtractResourcePlugin::<ClearColor>::default(),
                Core2dPlugin,
                Core3dPlugin,
                BlitPlugin,
                MsaaWritebackPlugin,
                TonemappingPlugin,
                UpscalingPlugin,
                BloomPlugin,
                FxaaPlugin,
                CASPlugin,
            ));
    }
}
