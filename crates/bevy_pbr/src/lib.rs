pub mod wireframe;

mod alpha;
mod bundle;
mod light;
mod material;
mod pbr_material;
mod render;

pub use alpha::*;
pub use bundle::*;
pub use light::*;
pub use material::*;
pub use pbr_material::*;
pub use render::*;

use bevy_window::ModifiesWindows;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        alpha::AlphaMode,
        bundle::{
            DirectionalLightBundle, MaterialMeshBundle, PbrBundle, PointLightBundle,
            SpotLightBundle,
        },
        light::{AmbientLight, DirectionalLight, PointLight, SpotLight},
        material::{Material, MaterialPlugin},
        pbr_material::StandardMaterial,
    };
}

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the shadow pass node.
        pub const SHADOW_PASS: &str = "shadow_pass";
    }
}

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, AddAsset, Assets, Handle, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::CameraUpdateSystem,
    extract_resource::ExtractResourcePlugin,
    prelude::Color,
    render_graph::RenderGraph,
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions},
    render_resource::{Shader, SpecializedMeshPipelines},
    view::VisibilitySystems,
    RenderApp, RenderStage,
};
use bevy_transform::TransformSystem;

pub const PBR_TYPES_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1708015359337029744);
pub const PBR_BINDINGS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 5635987986427308186);
pub const UTILS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1900548483293416725);
pub const CLUSTERED_FORWARD_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 166852093121196815);
pub const PBR_LIGHTING_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 14170772752254856967);
pub const SHADOWS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 11350275143789590502);
pub const PBR_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767701046);
pub const PBR_FUNCTIONS_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 16550102964439850292);
pub const SHADOW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745567947005696);

/// Sets up the entire PBR infrastructure of bevy.
#[derive(Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PBR_TYPES_SHADER_HANDLE,
            "render/pbr_types.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_BINDINGS_SHADER_HANDLE,
            "render/pbr_bindings.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, UTILS_HANDLE, "render/utils.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            CLUSTERED_FORWARD_HANDLE,
            "render/clustered_forward.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_LIGHTING_HANDLE,
            "render/pbr_lighting.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SHADOWS_HANDLE,
            "render/shadows.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            PBR_FUNCTIONS_HANDLE,
            "render/pbr_functions.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, PBR_SHADER_HANDLE, "render/pbr.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            SHADOW_SHADER_HANDLE,
            "render/depth.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<CubemapVisibleEntities>()
            .register_type::<DirectionalLight>()
            .register_type::<PointLight>()
            .register_type::<SpotLight>()
            .register_asset_reflect::<StandardMaterial>()
            .register_type::<AmbientLight>()
            .register_type::<DirectionalLightShadowMap>()
            .register_type::<ClusterConfig>()
            .register_type::<ClusterZConfig>()
            .register_type::<ClusterFarZMode>()
            .register_type::<PointLightShadowMap>()
            .add_plugin(MeshRenderPlugin)
            .add_plugin(MaterialPlugin::<StandardMaterial>::default())
            .init_resource::<AmbientLight>()
            .init_resource::<GlobalVisiblePointLights>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .add_plugin(ExtractResourcePlugin::<AmbientLight>::default())
            .add_system_to_stage(
                CoreStage::PostUpdate,
                // NOTE: Clusters need to have been added before update_clusters is run so
                // add as an exclusive system
                add_clusters
                    .at_start()
                    .label(SimulationLightSystems::AddClusters),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                assign_lights_to_clusters
                    .label(SimulationLightSystems::AssignLightsToClusters)
                    .after(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::CheckVisibility)
                    .after(CameraUpdateSystem)
                    .after(ModifiesWindows),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_directional_light_frusta
                    .label(SimulationLightSystems::UpdateLightFrusta)
                    // This must run after CheckVisibility because it relies on ComputedVisibility::is_visible()
                    .after(VisibilitySystems::CheckVisibility)
                    .after(TransformSystem::TransformPropagate)
                    // We assume that no entity will be both a directional light and a spot light,
                    // so these systems will run indepdently of one another.
                    // FIXME: Add an archetype invariant for this https://github.com/bevyengine/bevy/issues/1481.
                    .ambiguous_with(update_spot_light_frusta),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_point_light_frusta
                    .label(SimulationLightSystems::UpdateLightFrusta)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::AssignLightsToClusters),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_spot_light_frusta
                    .label(SimulationLightSystems::UpdateLightFrusta)
                    .after(TransformSystem::TransformPropagate)
                    .after(SimulationLightSystems::AssignLightsToClusters),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                check_light_mesh_visibility
                    .label(SimulationLightSystems::CheckLightVisibility)
                    .after(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .after(SimulationLightSystems::UpdateLightFrusta)
                    // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                    // because that resets entity ComputedVisibility for the first view
                    // which would override any results from this otherwise
                    .after(VisibilitySystems::CheckVisibility),
            );

        app.world
            .resource_mut::<Assets<StandardMaterial>>()
            .set_untracked(
                Handle::<StandardMaterial>::default(),
                StandardMaterial {
                    base_color: Color::rgb(1.0, 0.0, 0.5),
                    unlit: true,
                    ..Default::default()
                },
            );

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .add_system_to_stage(
                RenderStage::Extract,
                render::extract_clusters.label(RenderLightSystems::ExtractClusters),
            )
            .add_system_to_stage(
                RenderStage::Extract,
                render::extract_lights.label(RenderLightSystems::ExtractLights),
            )
            .add_system_to_stage(
                RenderStage::Prepare,
                // this is added as an exclusive system because it contributes new views. it must run (and have Commands applied)
                // _before_ the `prepare_views()` system is run. ideally this becomes a normal system when "stageless" features come out
                render::prepare_lights
                    .at_start()
                    .label(RenderLightSystems::PrepareLights),
            )
            .add_system_to_stage(
                RenderStage::Prepare,
                // NOTE: This needs to run after prepare_lights. As prepare_lights is an exclusive system,
                // just adding it to the non-exclusive systems in the Prepare stage means it runs after
                // prepare_lights.
                render::prepare_clusters.label(RenderLightSystems::PrepareClusters),
            )
            .add_system_to_stage(
                RenderStage::Queue,
                render::queue_shadows.label(RenderLightSystems::QueueShadows),
            )
            .add_system_to_stage(RenderStage::Queue, render::queue_shadow_view_bind_group)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Shadow>)
            .init_resource::<ShadowPipeline>()
            .init_resource::<DrawFunctions<Shadow>>()
            .init_resource::<LightMeta>()
            .init_resource::<GlobalLightMeta>()
            .init_resource::<SpecializedMeshPipelines<ShadowPipeline>>();

        let shadow_pass_node = ShadowPassNode::new(&mut render_app.world);
        render_app.add_render_command::<Shadow, DrawShadowMesh>();
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::SHADOW_PASS, shadow_pass_node);
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::SHADOW_PASS,
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            )
            .unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::SHADOW_PASS,
                ShadowPassNode::IN_VIEW,
            )
            .unwrap();
    }
}
