//! Module containing logic for the frame time graph

use bevy_app::{Plugin, Update};
use bevy_asset::{load_internal_asset, uuid_handle, Asset, Assets, Handle};
use bevy_diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy_ecs::system::{Res, ResMut};
use bevy_math::ops::log2;
use bevy_reflect::TypePath;
use bevy_render::{
    render_resource::{AsBindGroup, Shader, ShaderRef, ShaderType},
    storage::ShaderStorageBuffer,
};
use bevy_ui_render::prelude::{UiMaterial, UiMaterialPlugin};

use crate::fps_overlay::FpsOverlayConfig;

const FRAME_TIME_GRAPH_SHADER_HANDLE: Handle<Shader> =
    uuid_handle!("4e38163a-5782-47a5-af52-d9161472ab59");

/// Plugin that sets up everything to render the frame time graph material
pub struct FrameTimeGraphPlugin;

impl Plugin for FrameTimeGraphPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        load_internal_asset!(
            app,
            FRAME_TIME_GRAPH_SHADER_HANDLE,
            "frame_time_graph.wgsl",
            Shader::from_wgsl
        );

        // TODO: Use plugin dependencies, see https://github.com/bevyengine/bevy/issues/69
        if !app.is_plugin_added::<FrameTimeDiagnosticsPlugin>() {
            panic!("Requires FrameTimeDiagnosticsPlugin");
            // app.add_plugins(FrameTimeDiagnosticsPlugin);
        }

        app.add_plugins(UiMaterialPlugin::<FrametimeGraphMaterial>::default())
            .add_systems(Update, update_frame_time_values);
    }
}

/// The config values sent to the frame time graph shader
#[derive(Debug, Clone, Copy, ShaderType)]
pub struct FrameTimeGraphConfigUniform {
    // minimum expected delta time
    dt_min: f32,
    // maximum expected delta time
    dt_max: f32,
    dt_min_log2: f32,
    dt_max_log2: f32,
    // controls whether or not the bars width are proportional to their delta time
    proportional_width: u32,
}

impl FrameTimeGraphConfigUniform {
    /// `proportional_width`: controls whether or not the bars width are proportional to their delta time
    pub fn new(target_fps: f32, min_fps: f32, proportional_width: bool) -> Self {
        // we want an upper limit that is above the target otherwise the bars will disappear
        let dt_min = 1. / (target_fps * 1.2);
        let dt_max = 1. / min_fps;
        Self {
            dt_min,
            dt_max,
            dt_min_log2: log2(dt_min),
            dt_max_log2: log2(dt_max),
            proportional_width: u32::from(proportional_width),
        }
    }
}

/// The material used to render the frame time graph ui node
#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
pub struct FrametimeGraphMaterial {
    /// The history of the previous frame times value.
    ///
    /// This should be updated every frame to match the frame time history from the [`DiagnosticsStore`]
    #[storage(0, read_only)]
    pub values: Handle<ShaderStorageBuffer>, // Vec<f32>,
    /// The configuration values used by the shader to control how the graph is rendered
    #[uniform(1)]
    pub config: FrameTimeGraphConfigUniform,
}

impl UiMaterial for FrametimeGraphMaterial {
    fn fragment_shader() -> ShaderRef {
        FRAME_TIME_GRAPH_SHADER_HANDLE.into()
    }
}

/// A system that updates the frame time values sent to the frame time graph
fn update_frame_time_values(
    mut frame_time_graph_materials: ResMut<Assets<FrametimeGraphMaterial>>,
    mut buffers: ResMut<Assets<ShaderStorageBuffer>>,
    diagnostics_store: Res<DiagnosticsStore>,
    config: Option<Res<FpsOverlayConfig>>,
) {
    if !config.is_none_or(|c| c.frame_time_graph_config.enabled) {
        return;
    }
    let Some(frame_time) = diagnostics_store.get(&FrameTimeDiagnosticsPlugin::FRAME_TIME) else {
        return;
    };
    let frame_times = frame_time
        .values()
        // convert to millis
        .map(|x| *x as f32 / 1000.0)
        .collect::<Vec<_>>();
    for (_, material) in frame_time_graph_materials.iter_mut() {
        let buffer = buffers.get_mut(&material.values).unwrap();

        buffer.set_data(frame_times.clone().as_slice());
    }
}
