#![feature(min_specialization)]
mod camera;
pub mod entity;
pub mod mesh;
pub mod render_graph;
pub mod render_graph_2;
pub mod renderer_2;
pub mod shader;

mod color;
mod light;
mod vertex;

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
pub mod renderer;
pub mod texture;

pub use once_cell;

use self::{
    draw_target::draw_targets::{
        AssignedBatchesDrawTarget, AssignedMeshesDrawTarget, MeshesDrawTarget, UiDrawTarget,
    },
    mesh::Mesh,
    pass::passes::ForwardPassBuilder,
    pipeline::{
        pipelines::ForwardPipelineBuilder, PipelineCompiler, PipelineDescriptor,
        ShaderPipelineAssignments, VertexBufferDescriptors,
    },
    render_graph::RenderGraph,
    render_resource::{
        build_entity_render_resource_assignments_system,
        resource_providers::{
            Camera2dResourceProvider, CameraResourceProvider, LightResourceProvider,
            MeshResourceProvider, UniformResourceProvider,
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
            .add_resource_provider(CameraResourceProvider::new(
                app.resources().get_event_reader::<WindowResized>(),
            ))
            .add_resource_provider(Camera2dResourceProvider::new(
                resources.get_event_reader::<WindowResized>(),
            ))
            .add_resource_provider(LightResourceProvider::new(10))
            .add_resource_provider(MeshResourceProvider::new())
            .add_resource_provider(UniformResourceProvider::<StandardMaterial>::new(true))
            .add_resource_provider(UniformResourceProvider::<LocalToWorld>::new(true))
            .add_forward_pass()
            .add_forward_pipeline();
    }
}

impl AppPlugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        app.add_system(build_entity_render_resource_assignments_system())
            .build_system_on_stage(stage::POST_UPDATE, camera::camera_update_system)
            .add_system_to_stage(stage::POST_UPDATE, mesh::mesh_batcher_system())
            .add_stage_after(stage::POST_UPDATE, RENDER_STAGE)
            .add_resource(RenderGraph::default())
            .add_resource(AssetStorage::<Mesh>::new())
            .add_resource(AssetStorage::<Texture>::new())
            .add_resource(AssetStorage::<Shader>::new())
            .add_resource(AssetStorage::<StandardMaterial>::new())
            .add_resource(AssetStorage::<PipelineDescriptor>::new())
            .add_resource(ShaderPipelineAssignments::new())
            .add_resource(VertexBufferDescriptors::default())
            .add_resource(PipelineCompiler::new())
            .add_resource(RenderResourceAssignments::default())
            .add_resource(EntityRenderResourceAssignments::default())
            .add_resource(asset_batchers);
        RenderPlugin::setup_render_graph_defaults(app);
    }
}
