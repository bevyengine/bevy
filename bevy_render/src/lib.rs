#![feature(min_specialization)]
mod camera;
pub mod entity;
pub mod mesh;
pub mod render_graph;
pub mod render_graph_2;
pub mod renderer_2;
pub mod shader;
pub mod vertex;

mod color;
mod light;

pub use camera::*;
pub use color::*;
pub use light::*;
pub use renderable::*;

pub use vertex::Vertex;

pub mod draw_target;
pub mod pass;
pub mod pipeline;
pub mod render_resource;
mod renderable;
pub mod texture;

pub use once_cell;

use self::{
    draw_target::draw_targets::{
        AssignedBatchesDrawTarget, AssignedMeshesDrawTarget, MeshesDrawTarget, UiDrawTarget,
    },
    mesh::Mesh,
    pass::passes::ForwardPassBuilder,
    pipeline::{
        pipelines::ForwardPipelineBuilder, PipelineAssignments, PipelineCompiler,
        PipelineDescriptor, VertexBufferDescriptors,
    },
    render_graph::RenderGraph,
    render_resource::{
        entity_render_resource_assignments_system,
        resource_providers::{
            Camera2dResourceProvider, CameraResourceProvider, LightResourceProvider,
            UniformResourceProvider,
        },
        AssetBatchers, EntityRenderResourceAssignments, RenderResourceAssignments,
    },
    shader::{uniforms::StandardMaterial, Shader},
    texture::Texture,
};

use bevy_app::{stage, AppBuilder, AppPlugin, GetEventReader};
use bevy_asset::AssetStorage;
use bevy_transform::prelude::LocalToWorld;
use bevy_window::WindowResized;
use render_resource::resource_providers::{CameraNode, mesh_resource_provider_system};
use render_graph_2::RenderGraph2;

pub static RENDER_RESOURCE_STAGE: &str = "render_resource";
pub static RENDER_STAGE: &str = "render";

#[derive(Default)]
pub struct RenderPlugin;

impl RenderPlugin {
    pub fn setup_render_graph_defaults(app: &mut AppBuilder) {
        let resources = app.resources();
        let mut pipelines = app
            .resources()
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
        render_graph
            .build(&mut pipelines, &mut shaders)
            .add_draw_target(MeshesDrawTarget::default())
            .add_draw_target(AssignedBatchesDrawTarget::default())
            .add_draw_target(AssignedMeshesDrawTarget::default())
            .add_draw_target(UiDrawTarget::default())
            .add_resource_provider(Camera2dResourceProvider::new(
                resources.get_event_reader::<WindowResized>(),
            ))
            .add_resource_provider(LightResourceProvider::new(10))
            .add_resource_provider(UniformResourceProvider::<StandardMaterial>::new(true))
            .add_resource_provider(UniformResourceProvider::<LocalToWorld>::new(true))
            .add_forward_pass()
            .add_forward_pipeline();
    }
}

impl AppPlugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut render_graph = RenderGraph2::default();
        render_graph.add_system_node(CameraNode::default(), app.resources_mut());
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        app.add_stage_after(stage::POST_UPDATE, RENDER_RESOURCE_STAGE)
            .add_stage_after(RENDER_RESOURCE_STAGE, RENDER_STAGE)
            // resources
            .add_resource(RenderGraph::default())
            .add_resource(render_graph)
            .add_resource(AssetStorage::<Mesh>::new())
            .add_resource(AssetStorage::<Texture>::new())
            .add_resource(AssetStorage::<Shader>::new())
            .add_resource(AssetStorage::<StandardMaterial>::new())
            .add_resource(AssetStorage::<PipelineDescriptor>::new())
            .add_resource(PipelineAssignments::new())
            .add_resource(PipelineCompiler::new())
            .add_resource(RenderResourceAssignments::default())
            .add_resource(VertexBufferDescriptors::default())
            .add_resource(EntityRenderResourceAssignments::default())
            .add_resource(asset_batchers)
            // core systems
            .add_system(entity_render_resource_assignments_system())
            .add_system_to_stage_init(stage::POST_UPDATE, camera::camera_update_system)
            .add_system_to_stage(
                stage::POST_UPDATE,
                mesh::mesh_specializer_system(),
            )
            .add_system_to_stage(stage::POST_UPDATE, mesh::mesh_batcher_system())
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_handle_shader_def_system::<StandardMaterial>(),
            )
            .add_system_to_stage(
                stage::POST_UPDATE,
                shader::asset_handle_batcher_system::<StandardMaterial>(),
            )
            // render resource provider systems
            .add_system_to_stage_init(RENDER_RESOURCE_STAGE, mesh_resource_provider_system);
        RenderPlugin::setup_render_graph_defaults(app);
    }
}
