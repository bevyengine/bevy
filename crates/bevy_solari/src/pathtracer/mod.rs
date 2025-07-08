mod extract;
mod node;
mod prepare;

use crate::{scene::init_raytracing_scene_bindings, SolariSystems};
use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    component::Component, reflect::ReflectComponent, schedule::IntoScheduleConfigs, world::World,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    render_graph::{RenderGraphExt, ViewNodeRunner},
    view::Hdr,
    ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};
use extract::extract_pathtracer;
use node::PathtracerNode;
use prepare::prepare_pathtracer_accumulation_texture;

/// Non-realtime pathtracing.
///
/// This plugin is meant to generate reference screenshots to compare against,
/// and is not intended to be used by games.
pub struct PathtracingPlugin;

impl Plugin for PathtracingPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "pathtracer.wgsl");

        app.register_type::<Pathtracer>();

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_systems(
                RenderStartup,
                add_solari_pathtracing_render_graph_nodes
                    .after(init_raytracing_scene_bindings)
                    .in_set(SolariSystems),
            )
            .add_systems(ExtractSchedule, extract_pathtracer.in_set(SolariSystems))
            .add_systems(
                Render,
                prepare_pathtracer_accumulation_texture
                    .in_set(RenderSystems::PrepareResources)
                    .in_set(SolariSystems),
            );
    }
}

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component, Default, Clone)]
#[require(Hdr)]
pub struct Pathtracer {
    pub reset: bool,
}

// We only want to add these render graph nodes and edges if Solari required features are present.
// Making this a system that runs at RenderStartup allows a run condition to check for required
// features first.
fn add_solari_pathtracing_render_graph_nodes(world: &mut World) {
    world
        .add_render_graph_node::<ViewNodeRunner<PathtracerNode>>(
            Core3d,
            node::graph::PathtracerNode,
        )
        .add_render_graph_edges(Core3d, (Node3d::EndMainPass, node::graph::PathtracerNode));
}
