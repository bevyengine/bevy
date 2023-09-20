pub use self::node::SolariGlobalIlluminationNode;
use self::{
    pipelines::{prepare_pipelines, SolariGlobalIlluminationPipelines},
    view_resources::{prepare_bind_groups, prepare_resources},
};
use super::SolariEnabled;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::core_3d::CORE_3D;
use bevy_ecs::{component::Component, prelude::resource_exists, schedule::IntoSystemConfigs};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphApp, ViewNodeRunner},
    render_resource::{Shader, SpecializedComputePipelines},
    Render, RenderApp, RenderSet,
};
pub(crate) use view_resources::SolariGlobalIlluminationViewResources;

mod node;
mod pipelines;
mod view_resources;

const WORLD_CACHE_SIZE: u64 = 1048576;

const SOLARI_VIEW_BINDINGS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 0717171717171755);
const SOLARI_WORLD_CACHE_QUERY_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1717171717171755);
const SOLARI_WORLD_CACHE_COMPACT_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2717171717171755);
const SOLARI_WORLD_CACHE_UPDATE_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 3717171717171755);
const SOLARI_SCREEN_PROBES_UPDATE_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4717171717171755);
const SOLARI_SCREEN_PROBES_MERGE_CASCADES_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5717171717171755);
const SOLARI_SCREEN_PROBES_INTEPOLATE_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 6717171717171755);
const SOLARI_UTILS_SHADER: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7717171717171755);

pub struct SolariGlobalIlluminationPlugin;

impl Plugin for SolariGlobalIlluminationPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            SOLARI_VIEW_BINDINGS_SHADER,
            "view_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_WORLD_CACHE_QUERY_SHADER,
            "world_cache_query.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_WORLD_CACHE_COMPACT_SHADER,
            "world_cache_compact.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_WORLD_CACHE_UPDATE_SHADER,
            "world_cache_update.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_SCREEN_PROBES_UPDATE_SHADER,
            "screen_probes_update.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_SCREEN_PROBES_MERGE_CASCADES_SHADER,
            "screen_probes_merge_casades.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SOLARI_SCREEN_PROBES_INTEPOLATE_SHADER,
            "screen_probes_interpolate.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, SOLARI_UTILS_SHADER, "utils.wgsl", Shader::from_wgsl);

        app.add_plugins(ExtractComponentPlugin::<SolariGlobalIlluminationSettings>::default());

        app.sub_app_mut(RenderApp)
            .add_render_graph_node::<ViewNodeRunner<SolariGlobalIlluminationNode>>(
                CORE_3D,
                "solari_global_illumination",
            )
            .add_render_graph_edges(
                CORE_3D,
                &[
                    // PREPASS -> SOLARI_GLOBAL_ILLUMINATION -> MAIN_PASS
                    bevy_core_pipeline::core_3d::graph::node::PREPASS,
                    "solari_global_illumination",
                    bevy_core_pipeline::core_3d::graph::node::START_MAIN_PASS,
                ],
            )
            .init_resource::<SolariGlobalIlluminationPipelines>()
            .init_resource::<SpecializedComputePipelines<SolariGlobalIlluminationPipelines>>()
            .add_systems(
                Render,
                (
                    prepare_pipelines.in_set(RenderSet::PrepareResources),
                    prepare_resources.in_set(RenderSet::PrepareResources),
                    prepare_bind_groups.in_set(RenderSet::PrepareBindGroups),
                )
                    .run_if(resource_exists::<SolariEnabled>()),
            );
    }
}

#[derive(Component, ExtractComponent, Clone, Default)]
pub struct SolariGlobalIlluminationSettings {}
