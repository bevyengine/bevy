pub mod extract;
pub mod node;
pub mod prepare;

use crate::SolariPlugin;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{component::Component, reflect::ReflectComponent, schedule::IntoScheduleConfigs};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::Shader,
    renderer::RenderDevice,
    ExtractSchedule, Render, RenderApp, RenderSet,
};
use extract::extract_pathtracer;
use node::PathtracerNode;
use prepare::prepare_pathtracer_accumulation_texture;
use tracing::warn;

const PATHTRACER_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("87a5940d-b1ba-4cae-b8ce-be20c931e0c7");

pub struct PathtracingPlugin;

impl Plugin for PathtracingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PATHTRACER_SHADER_HANDLE,
            "pathtracer.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<Pathtracer>();
    }

    fn finish(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);

        let render_device = render_app.world().resource::<RenderDevice>();
        let features = render_device.features();
        if !features.contains(SolariPlugin::required_wgpu_features()) {
            warn!(
                "PathtracingPlugin not loaded. GPU lacks support for required features: {:?}.",
                SolariPlugin::required_wgpu_features().difference(features)
            );
            return;
        }

        render_app
            .add_systems(ExtractSchedule, extract_pathtracer)
            .add_systems(
                Render,
                prepare_pathtracer_accumulation_texture.in_set(RenderSet::PrepareResources),
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
pub struct Pathtracer;
