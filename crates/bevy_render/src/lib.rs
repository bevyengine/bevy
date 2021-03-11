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
pub mod wireframe;

use bevy_ecs::{
    schedule::{ParallelSystemDescriptorCoercion, SystemStage},
    system::{IntoExclusiveSystem, IntoSystem},
};
use bevy_transform::TransformSystem;
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
use bevy_asset::{AddAsset, AssetStage};
use bevy_ecs::schedule::{StageLabel, SystemLabel};
use camera::{
    ActiveCameras, Camera, DepthCalculation, OrthographicProjection, PerspectiveProjection,
    RenderLayers, ScalingMode, VisibleEntities, WindowOrigin,
};
use pipeline::{
    IndexFormat, PipelineCompiler, PipelineDescriptor, PipelineSpecialization, PrimitiveTopology,
    ShaderSpecialization, VertexBufferLayout,
};
use render_graph::{
    base::{self, BaseRenderGraphConfig, MainPass},
    RenderGraph,
};
use renderer::{AssetRenderResourceBindings, RenderResourceBindings};
use shader::ShaderLoader;
#[cfg(feature = "hdr")]
use texture::HdrTextureLoader;
#[cfg(feature = "png")]
use texture::ImageTextureLoader;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum RenderSystem {
    VisibleEntities,
}

/// The names of "render" App stages
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderStage {
    /// Stage where render resources are set up
    RenderResource,
    /// Stage where Render Graph systems are run. In general you shouldn't add systems to this
    /// stage manually.
    RenderGraphSystems,
    // Stage where draw systems are executed. This is generally where Draw components are setup
    Draw,
    Render,
    PostRender,
}

/// Adds core render types and systems to an App
pub struct RenderPlugin {
    /// configures the "base render graph". If this is not `None`, the "base render graph" will be
    /// added
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

        app.add_stage_after(
            AssetStage::AssetEvents,
            RenderStage::RenderResource,
            SystemStage::parallel(),
        )
        .add_stage_after(
            RenderStage::RenderResource,
            RenderStage::RenderGraphSystems,
            SystemStage::parallel(),
        )
        .add_stage_after(
            RenderStage::RenderGraphSystems,
            RenderStage::Draw,
            SystemStage::parallel(),
        )
        .add_stage_after(
            RenderStage::Draw,
            RenderStage::Render,
            SystemStage::parallel(),
        )
        .add_stage_after(
            RenderStage::Render,
            RenderStage::PostRender,
            SystemStage::parallel(),
        )
        .init_asset_loader::<ShaderLoader>()
        .add_asset::<Mesh>()
        .add_asset::<Texture>()
        .add_asset::<Shader>()
        .add_asset::<PipelineDescriptor>()
        .register_type::<Camera>()
        .register_type::<DepthCalculation>()
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
        .register_type::<RenderLayers>()
        .register_type::<ScalingMode>()
        .register_type::<VertexBufferLayout>()
        .register_type::<WindowOrigin>()
        .init_resource::<ClearColor>()
        .init_resource::<RenderGraph>()
        .init_resource::<PipelineCompiler>()
        .init_resource::<Msaa>()
        .init_resource::<RenderResourceBindings>()
        .init_resource::<AssetRenderResourceBindings>()
        .init_resource::<ActiveCameras>()
        .add_system_to_stage(CoreStage::PreUpdate, draw::clear_draw_system.system())
        .add_system_to_stage(
            CoreStage::PostUpdate,
            camera::active_cameras_system.system(),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            camera::camera_system::<OrthographicProjection>
                .system()
                .before(RenderSystem::VisibleEntities),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            camera::camera_system::<PerspectiveProjection>
                .system()
                .before(RenderSystem::VisibleEntities),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            camera::visible_entities_system
                .system()
                .label(RenderSystem::VisibleEntities)
                .after(TransformSystem::TransformPropagate),
        )
        .add_system_to_stage(
            RenderStage::RenderResource,
            shader::shader_update_system.system(),
        )
        .add_system_to_stage(
            RenderStage::RenderResource,
            mesh::mesh_resource_provider_system.system(),
        )
        .add_system_to_stage(
            RenderStage::RenderResource,
            Texture::texture_resource_system.system(),
        )
        .add_system_to_stage(
            RenderStage::RenderGraphSystems,
            render_graph::render_graph_schedule_executor_system.exclusive_system(),
        )
        .add_system_to_stage(
            RenderStage::Draw,
            pipeline::draw_render_pipelines_system.system(),
        )
        .add_system_to_stage(
            RenderStage::PostRender,
            shader::clear_shader_defs_system.system(),
        );

        if let Some(ref config) = self.base_render_graph_config {
            crate::base::add_base_graph(config, app.world_mut());
            let mut active_cameras = app.world_mut().get_resource_mut::<ActiveCameras>().unwrap();
            if config.add_3d_camera {
                active_cameras.add(base::camera::CAMERA_3D);
            }

            if config.add_2d_camera {
                active_cameras.add(base::camera::CAMERA_2D);
            }
        }
    }
}
