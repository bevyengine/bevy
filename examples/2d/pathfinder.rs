use bevy::prelude::*;
use bevy::render::base_render_graph::BaseRenderGraphConfig;
use bevy_diagnostic::{PrintDiagnosticsPlugin, FrameTimeDiagnosticsPlugin};
use bevy_wgpu::diagnostic::WgpuResourceDiagnosticsPlugin;

fn main() {
    App::build()
        .add_plugin(bevy::core::CorePlugin::default())
        .add_plugin(bevy::diagnostic::DiagnosticsPlugin::default())
        .add_plugin(bevy::input::InputPlugin::default())
        .add_plugin(bevy::window::WindowPlugin::default())
        .add_plugin(bevy::render::RenderPlugin {
            base_render_graph_config: Some(BaseRenderGraphConfig {
                add_2d_camera: true,
                add_3d_camera: false,
                add_main_pass: false,
                add_main_depth_texture:true,
                connect_main_pass_to_swapchain: false,
                connect_main_pass_to_main_depth_texture: false,
            })
        })
        .add_plugin(bevy::pathfinder::PathfinderPlugin::default())
        .add_plugin(bevy::winit::WinitPlugin::default())
        .add_plugin(bevy::wgpu::WgpuPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(WgpuResourceDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .run();
}
