pub mod bloom;
pub mod clear_color;
pub mod core_2d;
pub mod core_3d;
pub mod fullscreen_vertex_shader;
pub mod fxaa;
pub mod tonemapping;
pub mod upscaling;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        clear_color::ClearColor,
        core_2d::{Camera2d, Camera2dBundle},
        core_3d::{Camera3d, Camera3dBundle},
    };
}

use crate::{
    bloom::BloomPlugin,
    clear_color::{ClearColor, ClearColorConfig},
    core_2d::Core2dPlugin,
    core_3d::Core3dPlugin,
    fullscreen_vertex_shader::FULLSCREEN_SHADER_HANDLE,
    fxaa::FxaaPlugin,
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
            .init_resource::<ClearColor>()
            .add_plugin(ExtractResourcePlugin::<ClearColor>::default())
            .add_plugin(Core2dPlugin)
            .add_plugin(Core3dPlugin)
            .add_plugin(TonemappingPlugin)
            .add_plugin(UpscalingPlugin)
            .add_plugin(BloomPlugin)
            .add_plugin(FxaaPlugin);
    }
}
