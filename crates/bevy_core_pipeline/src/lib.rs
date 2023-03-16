pub mod blit;
pub mod bloom;
pub mod clear_color;
pub mod core_2d;
pub mod core_3d;
pub mod fullscreen_vertex_shader;
pub mod fxaa;
pub mod msaa_writeback;
pub mod prepass;
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
    blit::BlitPlugin,
    bloom::BloomPlugin,
    clear_color::{ClearColor, ClearColorConfig},
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
use bevy_ecs::world::FromWorld;
use bevy_render::{
    extract_resource::ExtractResourcePlugin,
    prelude::Shader,
    render_graph::{Node, RenderGraph},
};

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
            .add_plugin(ExtractResourcePlugin::<ClearColor>::default())
            .add_plugin(Core2dPlugin)
            .add_plugin(Core3dPlugin)
            .add_plugin(BlitPlugin)
            .add_plugin(MsaaWritebackPlugin)
            .add_plugin(TonemappingPlugin)
            .add_plugin(UpscalingPlugin)
            .add_plugin(BloomPlugin)
            .add_plugin(FxaaPlugin);
    }
}

/// Utility function to add a [`Node`] to the [`RenderGraph`]
/// * Create the [`Node`] using the [`FromWorld`] implementation
/// * Add it to the graph
/// * Automatically add the required node edges based on the given ordering
pub fn add_node<T: Node + FromWorld>(
    render_app: &mut App,
    sub_graph_name: &'static str,
    node_name: &'static str,
    edges: &[&'static str],
) {
    let node = T::from_world(&mut render_app.world);
    let mut render_graph = render_app.world.resource_mut::<RenderGraph>();

    let graph = render_graph.sub_graph_mut(sub_graph_name);
    graph.add_node_with_edges(node_name, node, edges);
}
