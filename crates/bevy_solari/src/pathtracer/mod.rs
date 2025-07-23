mod extract;
mod node;
mod prepare;

use crate::SolariPlugins;
use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{component::Component, reflect::ReflectComponent, schedule::IntoScheduleConfigs};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    renderer::RenderDevice,
    view::Hdr,
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use extract::extract_pathtracer;
use node::PathtracerNode;
use prepare::prepare_pathtracer_accumulation_texture;
use tracing::warn;

/// Non-realtime pathtracing.
///
/// This plugin is meant to generate reference screenshots to compare against,
/// and is not intended to be used by games.
pub struct PathtracingPlugin;

impl Plugin for PathtracingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "pathtracer.wgsl");

        app.register_type::<Pathtracer>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugins::required_wgpu_features()) {
            warn!(
                "PathtracingPlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugins::required_wgpu_features().difference(features)
            );
            return;
        }

        render_app
            .add_systems(ExtractSchedule, extract_pathtracer)
            .add_systems(
                Render,
                prepare_pathtracer_accumulation_texture.in_set(RenderSystems::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<PathtracerNode>>(
                Core3d,
                node::graph::PathtracerNode,
            )
            .add_render_graph_edges(Core3d, (Node3d::EndMainPass, node::graph::PathtracerNode));
    }
}

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Default, Clone)]
#[require(Hdr)]
pub struct Pathtracer {
    pub reset: bool,
}
