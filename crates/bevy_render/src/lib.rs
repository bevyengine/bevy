pub mod batch;
mod camera;
pub mod draw;
pub mod entity;
pub mod mesh;
pub mod render_graph;
pub mod renderer;
pub mod shader;
pub mod vertex;

mod color;

pub use camera::*;
pub use color::*;

pub use vertex::Vertex;

pub mod base_render_graph;
pub mod pass;
pub mod pipeline;
pub mod render_resource;
pub mod texture;

pub use once_cell;

use self::{
    mesh::Mesh,
    pipeline::{PipelineCompiler, PipelineDescriptor, VertexBufferDescriptors},
    render_resource::RenderResourceBindings,
    shader::Shader,
    texture::Texture,
};

use base_render_graph::{BaseRenderGraphBuilder, BaseRenderGraphConfig};
use bevy_app::{AppBuilder, AppPlugin};
use bevy_asset::AddAsset;
use bevy_type_registry::RegisterType;
use draw::{clear_draw_system, Draw};
use legion::prelude::IntoSystem;
use mesh::mesh_resource_provider_system;
use pipeline::{draw_render_pipelines_system, RenderPipelines};
use render_graph::{system::render_graph_schedule_executor_system, RenderGraph};
use render_resource::AssetRenderResourceBindings;
use shader::clear_shader_defs_system;
use std::ops::Range;
use texture::{PngTextureLoader, TextureResourceSystemState};

pub mod stage {
    /// Stage where render resources are set up
    pub static RENDER_RESOURCE: &str = "render_resource";
    /// Stage where Render Graph systems are run. In general you shouldn't add systems to this stage manually.
    pub static RENDER_GRAPH_SYSTEMS: &str = "render_graph_systems";
    // Stage where draw systems are executed. This is generally where Draw components are setup
    pub static DRAW: &str = "draw";
    pub static RENDER: &str = "render";
    pub static POST_RENDER: &str = "post_render";
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
            .add_stage_after(stage::RENDER_RESOURCE, stage::RENDER_GRAPH_SYSTEMS)
            .add_stage_after(stage::RENDER_GRAPH_SYSTEMS, stage::DRAW)
            .add_stage_after(stage::DRAW, stage::RENDER)
            .add_stage_after(stage::RENDER, stage::POST_RENDER)
            .add_asset::<Mesh>()
            .add_asset::<Texture>()
            .add_asset::<Shader>()
            .add_asset::<PipelineDescriptor>()
            .add_asset_loader::<Texture, PngTextureLoader>()
            .register_component::<Camera>()
            .register_component::<Draw>()
            .register_component::<RenderPipelines>()
            .register_component::<OrthographicProjection>()
            .register_component::<PerspectiveProjection>()
            .register_property_type::<Color>()
            .register_property_type::<Range<f32>>()
            .init_resource::<RenderGraph>()
            .init_resource::<PipelineCompiler>()
            .init_resource::<RenderResourceBindings>()
            .init_resource::<VertexBufferDescriptors>()
            .init_resource::<TextureResourceSystemState>()
            .init_resource::<AssetRenderResourceBindings>()
            .init_resource::<ActiveCameras>()
            .add_system_to_stage(bevy_app::stage::PRE_UPDATE, clear_draw_system.system())
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::active_cameras_system.system(),
            )
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<OrthographicProjection>(),
            )
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                camera::camera_system::<PerspectiveProjection>(),
            )
            // registration order matters here. this must come after all camera_system::<T> systems
            .add_system_to_stage(
                bevy_app::stage::POST_UPDATE,
                visible_entities_system.system(),
            )
            // TODO: turn these "resource systems" into graph nodes and remove the RENDER_RESOURCE stage
            .init_system_to_stage(stage::RENDER_RESOURCE, mesh_resource_provider_system)
            .add_system_to_stage(
                stage::RENDER_RESOURCE,
                Texture::texture_resource_system.system(),
            )
            .add_system_to_stage(
                stage::RENDER_GRAPH_SYSTEMS,
                render_graph_schedule_executor_system,
            )
            .add_system_to_stage(stage::DRAW, draw_render_pipelines_system.system())
            .add_system_to_stage(stage::POST_RENDER, clear_shader_defs_system.system());

        if let Some(ref config) = self.base_render_graph_config {
            let resources = app.resources();
            let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
            render_graph.add_base_graph(config);
            let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
            if config.add_3d_camera {
                active_cameras.add(base_render_graph::camera::CAMERA);
            }

            if config.add_2d_camera {
                active_cameras.add(base_render_graph::camera::CAMERA2D);
            }
        }
    }
}
