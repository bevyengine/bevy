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

pub mod base_render_graph;
pub mod draw_target;
pub mod pass;
pub mod pipeline;
pub mod render_resource;
mod renderable;
pub mod texture;

pub use once_cell;
pub use bevy_derive::{Uniform, Uniforms};

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

use base_render_graph::{BaseRenderGraphBuilder, BaseRenderGraphConfig};
use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;
use bevy_type_registry::RegisterType;
use legion::prelude::IntoSystem;
use mesh::mesh_resource_provider_system;
use render_graph::RenderGraph;
use render_resource::EntitiesWaitingForAssets;
use std::ops::Range;
use texture::{PngTextureLoader, TextureResourceSystemState};

pub mod stage {
    pub static RENDER_RESOURCE: &str = "render_resource";
    pub static RENDER: &str = "render";
}

pub struct RenderPlugin {
    /// configures the "base render graph". If this is not `None`, the "base render graph" will be added  
    pub base_render_graph_config: Option<BaseRenderGraphConfig>,
}

impl Default for RenderPlugin {
    fn default() -> Self {
        RenderPlugin {
            base_render_graph_config: Some(BaseRenderGraphConfig::default()),
        }
    }
}

impl AppPlugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_stage_after(bevy_asset::stage::ASSET_EVENTS, stage::RENDER_RESOURCE)
            .add_stage_after(stage::RENDER_RESOURCE, stage::RENDER)
            .add_asset::<Mesh>()
            .add_asset::<Texture>()
            .add_asset::<Shader>()
            .add_asset::<PipelineDescriptor>()
            .add_asset_loader::<Texture, PngTextureLoader>()
            .register_component::<Camera>()
            .register_component::<OrthographicProjection>()
            .register_component::<PerspectiveProjection>()
            .register_component::<Renderable>()
            .register_property_type::<Color>()
            .register_property_type::<Range<f32>>()
            .init_resource::<RenderGraph>()
            .init_resource::<PipelineAssignments>()
            .init_resource::<PipelineCompiler>()
            .init_resource::<RenderResourceAssignments>()
            .init_resource::<VertexBufferDescriptors>()
            .init_resource::<EntityRenderResourceAssignments>()
            .init_resource::<EntitiesWaitingForAssets>()
            .init_resource::<TextureResourceSystemState>()
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                entity_render_resource_assignments_system(),
            )
            .init_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<OrthographicProjection>,
            )
            .init_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<PerspectiveProjection>,
            )
            .add_system_to_stage(
                bevy_app::stage::PRE_UPDATE,
                EntitiesWaitingForAssets::clear_system.system(),
            )
            .init_system_to_stage(stage::RENDER_RESOURCE, mesh_resource_provider_system)
            .add_system_to_stage(
                stage::RENDER_RESOURCE,
                Texture::texture_resource_system.system(),
            );

        if let Some(ref config) = self.base_render_graph_config {
            let resources = app.resources();
            let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
            render_graph.add_base_graph(config);
        }
    }
}
