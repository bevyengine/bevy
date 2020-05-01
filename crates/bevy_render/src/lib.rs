#![feature(min_specialization)]
pub mod batch;
mod camera;
pub mod entity;
pub mod mesh;
pub mod render_graph;
pub mod renderer;
pub mod shader;
pub mod vertex;

mod color;

pub use camera::*;
pub use color::*;
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
    mesh::Mesh,
    pipeline::{
        PipelineAssignments, PipelineCompiler, PipelineDescriptor, VertexBufferDescriptors,
    },
    render_resource::{
        entity_render_resource_assignments_system, EntityRenderResourceAssignments,
        RenderResourceAssignments,
    },
    shader::Shader,
    texture::Texture,
};

use batch::AssetBatchers;
use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_asset::AssetStorage;
use mesh::mesh_resource_provider_system;
use render_graph::RenderGraph;

pub static RENDER_RESOURCE_STAGE: &str = "render_resource";
pub static RENDER_STAGE: &str = "render";

#[derive(Default)]
pub struct RenderPlugin;

impl RenderPlugin {}

impl AppPlugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(stage::POST_UPDATE, RENDER_RESOURCE_STAGE)
            .add_stage_after(RENDER_RESOURCE_STAGE, RENDER_STAGE)
            // resources
            .add_resource(RenderGraph::default())
            .add_resource(AssetStorage::<Mesh>::new())
            .add_resource(AssetStorage::<Texture>::new())
            .add_resource(AssetStorage::<Shader>::new())
            .add_resource(AssetStorage::<PipelineDescriptor>::new())
            .add_resource(PipelineAssignments::new())
            .add_resource(PipelineCompiler::new())
            .add_resource(RenderResourceAssignments::default())
            .add_resource(VertexBufferDescriptors::default())
            .add_resource(EntityRenderResourceAssignments::default())
            .add_resource(AssetBatchers::default())
            // core systems
            .add_system(entity_render_resource_assignments_system())
            .init_system_to_stage(stage::POST_UPDATE, camera::camera_update_system)
            .add_system_to_stage(stage::POST_UPDATE, mesh::mesh_specializer_system())
            .add_system_to_stage(stage::POST_UPDATE, mesh::mesh_batcher_system())
            // render resource provider systems
            .init_system_to_stage(RENDER_RESOURCE_STAGE, mesh_resource_provider_system);
    }
}
