mod bundle;
mod light;
mod material;
mod render;

pub use bundle::*;
pub use light::*;
pub use material::*;
pub use render::*;

use bevy_app::prelude::*;
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_core_pipeline::Transparent3d;
use bevy_ecs::prelude::*;
use bevy_reflect::TypeUuid;
use bevy_render2::{
    render_component::{ExtractComponentPlugin, UniformComponentPlugin},
    render_graph::RenderGraph,
    render_phase::{sort_phase_system, AddRenderCommand, DrawFunctions},
    render_resource::{Shader, SpecializedPipelines},
    view::VisibilitySystems,
    RenderApp, RenderStage,
};
use bevy_transform::TransformSystem;

pub mod draw_3d_graph {
    pub mod node {
        pub const SHADOW_PASS: &str = "shadow_pass";
    }
}

pub const PBR_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4805239651767701046);
pub const SHADOW_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 1836745567947005696);

#[derive(Default)]
pub struct PbrPlugin;

impl Plugin for PbrPlugin {
    fn build(&self, app: &mut App) {
        let mut shaders = app.world.get_resource_mut::<Assets<Shader>>().unwrap();
        let pbr_shader = Shader::from_wgsl(include_str!("render/pbr.wgsl"));
        shaders.set_untracked(PBR_SHADER_HANDLE, pbr_shader);
        let shadow_shader = Shader::from_wgsl(include_str!("render/depth.wgsl"));
        shaders.set_untracked(SHADOW_SHADER_HANDLE, shadow_shader);

        app.add_plugin(StandardMaterialPlugin)
            .add_plugin(ExtractComponentPlugin::<Handle<StandardMaterial>>::default())
            .add_plugin(UniformComponentPlugin::<MeshUniform>::default())
            .init_resource::<AmbientLight>()
            .init_resource::<DirectionalLightShadowMap>()
            .init_resource::<PointLightShadowMap>()
            .init_resource::<AmbientLight>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_directional_light_frusta
                    .label(SimulationLightSystems::UpdateDirectionalLightFrusta)
                    .after(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                update_point_light_frusta
                    .label(SimulationLightSystems::UpdatePointLightFrusta)
                    .after(TransformSystem::TransformPropagate),
            )
            .add_system_to_stage(
                CoreStage::PostUpdate,
                check_light_visibility
                    .label(SimulationLightSystems::CheckLightVisibility)
                    .after(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::CalculateBounds)
                    .after(SimulationLightSystems::UpdateDirectionalLightFrusta)
                    .after(SimulationLightSystems::UpdatePointLightFrusta)
                    // NOTE: This MUST be scheduled AFTER the core renderer visibility check
                    // because that resets entity ComputedVisibility for the first view
                    // which would override any results from this otherwise
                    .after(VisibilitySystems::CheckVisibility),
            );

        let render_app = app.sub_app(RenderApp);
        render_app
            .add_system_to_stage(RenderStage::Extract, render::extract_meshes)
            .add_system_to_stage(
                RenderStage::Extract,
                render::extract_lights.label(RenderLightSystems::ExtractLights),
            )
            .add_system_to_stage(
                RenderStage::Prepare,
                // this is added as an exclusive system because it contributes new views. it must run (and have Commands applied)
                // _before_ the `prepare_views()` system is run. ideally this becomes a normal system when "stageless" features come out
                render::prepare_lights
                    .exclusive_system()
                    .label(RenderLightSystems::PrepareLights),
            )
            .add_system_to_stage(RenderStage::Queue, render::queue_meshes)
            .add_system_to_stage(
                RenderStage::Queue,
                render::queue_shadows.label(RenderLightSystems::QueueShadows),
            )
            .add_system_to_stage(RenderStage::Queue, render::queue_shadow_view_bind_group)
            .add_system_to_stage(RenderStage::Queue, render::queue_transform_bind_group)
            .add_system_to_stage(RenderStage::PhaseSort, sort_phase_system::<Shadow>)
            .init_resource::<PbrPipeline>()
            .init_resource::<ShadowPipeline>()
            .init_resource::<DrawFunctions<Shadow>>()
            .init_resource::<LightMeta>()
            .init_resource::<SpecializedPipelines<PbrPipeline>>()
            .init_resource::<SpecializedPipelines<ShadowPipeline>>();

        let shadow_pass_node = ShadowPassNode::new(&mut render_app.world);
        render_app.add_render_command::<Transparent3d, DrawPbr>();
        render_app.add_render_command::<Shadow, DrawShadowMesh>();
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::draw_3d_graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::SHADOW_PASS, shadow_pass_node);
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::SHADOW_PASS,
                bevy_core_pipeline::draw_3d_graph::node::MAIN_PASS,
            )
            .unwrap();
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::draw_3d_graph::input::VIEW_ENTITY,
                draw_3d_graph::node::SHADOW_PASS,
                ShadowPassNode::IN_VIEW,
            )
            .unwrap();
    }
}
