mod extract;
mod node;
mod prepare;

use crate::SolariPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass},
};
use bevy_ecs::{component::Component, reflect::ReflectComponent, schedule::IntoScheduleConfigs};
use bevy_pbr::DefaultOpaqueRendererMethod;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    load_shader_library,
    render_graph::{RenderGraphApp, ViewNodeRunner},
    renderer::RenderDevice,
    view::Hdr,
    ExtractSchedule, Render, RenderApp, RenderSystems,
};
use extract::extract_solari_lighting;
use node::SolariLightingNode;
use prepare::prepare_solari_lighting_resources;
use tracing::warn;

pub struct SolariLightingPlugin;

impl Plugin for SolariLightingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "direct.wgsl");
        load_shader_library!(app, "reservoir.wgsl");

        app.register_type::<SolariLighting>()
            .insert_resource(DefaultOpaqueRendererMethod::deferred());
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugin::required_wgpu_features()) {
            warn!(
                "SolariLightingPlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugin::required_wgpu_features().difference(features)
            );
            return;
        }
        render_app
            .add_systems(ExtractSchedule, extract_solari_lighting)
            .add_systems(
                Render,
                prepare_solari_lighting_resources.in_set(RenderSystems::PrepareResources),
            )
            .add_render_graph_node::<ViewNodeRunner<SolariLightingNode>>(
                Core3d,
                node::graph::SolariLightingNode,
            )
            .add_render_graph_edges(
                Core3d,
                (Node3d::EndMainPass, node::graph::SolariLightingNode),
            );
    }
}

#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
#[require(Hdr, DeferredPrepass, DepthPrepass, MotionVectorPrepass)]
pub struct SolariLighting {
    pub reset: bool,
}

impl Default for SolariLighting {
    fn default() -> Self {
        Self {
            reset: true, // No temporal history on the first frame
        }
    }
}
