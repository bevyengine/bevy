pub mod camera;
pub mod color;
pub mod colorspace;
pub mod draw;
pub mod entity;
pub mod mesh;
pub mod pass;
pub mod pipeline;
pub mod render_graph;
pub mod renderer;
pub mod shader;
pub mod texture;

use bevy_ecs::{IntoSystem, SystemStage};
use bevy_reflect::RegisterTypeBuilder;
use draw::Visible;
pub use once_cell;

pub mod prelude {
    pub use crate::{
        base::Msaa,
        color::Color,
        draw::{Draw, Visible},
        entity::*,
        mesh::{shape, Mesh},
        pass::ClearColor,
        pipeline::RenderPipelines,
        shader::Shader,
        texture::Texture,
    };
}

use crate::prelude::*;
use base::Msaa;
use bevy_app::prelude::*;
use bevy_asset::AddAsset;
use camera::{
    ActiveCameras, Camera, OrthographicProjection, PerspectiveProjection, VisibleEntities,
};
use pipeline::{
    IndexFormat, PipelineCompiler, PipelineDescriptor, PipelineSpecialization, PrimitiveTopology,
    ShaderSpecialization,
};
use render_graph::{
    base::{self, BaseRenderGraphBuilder, BaseRenderGraphConfig, MainPass},
    RenderGraph,
};
use renderer::{AssetRenderResourceBindings, RenderResourceBindings};
use shader::ShaderLoader;
#[cfg(feature = "hdr")]
use texture::HdrTextureLoader;
#[cfg(feature = "png")]
use texture::ImageTextureLoader;
use texture::TextureResourceSystemState;

/// The names of "render" App stages
pub mod stage {
    /// Stage where render resources are set up
    pub const RENDER_RESOURCE: &str = "render_resource";
    /// Stage where Render Graph systems are run. In general you shouldn't add systems to this stage manually.
    pub const RENDER_GRAPH_SYSTEMS: &str = "render_graph_systems";
    // Stage where draw systems are executed. This is generally where Draw components are setup
    pub const DRAW: &str = "draw";
    pub const RENDER: &str = "render";
    pub const POST_RENDER: &str = "post_render";
}

/// Adds core render types and systems to an App
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

impl Plugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        #[cfg(feature = "png")]
        {
            app.init_asset_loader::<ImageTextureLoader>();
        }
        #[cfg(feature = "hdr")]
        {
            app.init_asset_loader::<HdrTextureLoader>();
        }

        app.init_asset_loader::<ShaderLoader>();

        if app.resources().get::<ClearColor>().is_none() {
            app.resources_mut().insert(ClearColor::default());
        }

        app.add_stage_after(
            bevy_asset::stage::ASSET_EVENTS,
            stage::RENDER_RESOURCE,
            SystemStage::parallel(),
        )
        .add_stage_after(
            stage::RENDER_RESOURCE,
            stage::RENDER_GRAPH_SYSTEMS,
            SystemStage::parallel(),
        )
        .add_stage_after(
            stage::RENDER_GRAPH_SYSTEMS,
            stage::DRAW,
            SystemStage::parallel(),
        )
        .add_stage_after(stage::DRAW, stage::RENDER, SystemStage::parallel())
        .add_stage_after(stage::RENDER, stage::POST_RENDER, SystemStage::parallel())
        .add_asset::<Mesh>()
        .add_asset::<Texture>()
        .add_asset::<Shader>()
        .add_asset::<PipelineDescriptor>()
        .register_type::<Camera>()
        .register_type::<Draw>()
        .register_type::<Visible>()
        .register_type::<RenderPipelines>()
        .register_type::<OrthographicProjection>()
        .register_type::<PerspectiveProjection>()
        .register_type::<MainPass>()
        .register_type::<VisibleEntities>()
        .register_type::<Color>()
        .register_type::<ShaderSpecialization>()
        .register_type::<PrimitiveTopology>()
        .register_type::<IndexFormat>()
        .register_type::<PipelineSpecialization>()
        .init_resource::<RenderGraph>()
        .init_resource::<PipelineCompiler>()
        .init_resource::<RenderResourceBindings>()
        .init_resource::<TextureResourceSystemState>()
        .init_resource::<AssetRenderResourceBindings>()
        .init_resource::<ActiveCameras>()
        .add_system_to_stage(
            bevy_app::stage::PRE_UPDATE,
            draw::clear_draw_system.system(),
        )
        .add_system_to_stage(
            bevy_app::stage::POST_UPDATE,
            camera::active_cameras_system.system(),
        )
        .add_system_to_stage(
            bevy_app::stage::POST_UPDATE,
            camera::camera_system::<OrthographicProjection>.system(),
        )
        .add_system_to_stage(
            bevy_app::stage::POST_UPDATE,
            camera::camera_system::<PerspectiveProjection>.system(),
        )
        // registration order matters here. this must come after all camera_system::<T> systems
        .add_system_to_stage(
            bevy_app::stage::POST_UPDATE,
            camera::visible_entities_system.system(),
        )
        .add_system_to_stage(
            stage::RENDER_RESOURCE,
            shader::shader_update_system.system(),
        )
        .add_system_to_stage(
            stage::RENDER_RESOURCE,
            mesh::mesh_resource_provider_system.system(),
        )
        .add_system_to_stage(
            stage::RENDER_RESOURCE,
            Texture::texture_resource_system.system(),
        )
        .add_system_to_stage(
            stage::RENDER_GRAPH_SYSTEMS,
            render_graph::render_graph_schedule_executor_system.system(),
        )
        .add_system_to_stage(stage::DRAW, pipeline::draw_render_pipelines_system.system())
        .add_system_to_stage(
            stage::POST_RENDER,
            shader::clear_shader_defs_system.system(),
        );

        if app.resources().get::<Msaa>().is_none() {
            app.init_resource::<Msaa>();
        }

        if let Some(ref config) = self.base_render_graph_config {
            let resources = app.resources();
            let mut render_graph = resources.get_mut::<RenderGraph>().unwrap();
            let msaa = resources.get::<Msaa>().unwrap();
            render_graph.add_base_graph(config, &msaa);
            let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
            if config.add_3d_camera {
                active_cameras.add(base::camera::CAMERA_3D);
            }

            if config.add_2d_camera {
                active_cameras.add(base::camera::CAMERA_2D);
            }
        }
    }
}
