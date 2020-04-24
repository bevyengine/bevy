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
    draw_target::draw_targets::AssignedMeshesDrawTarget,
    mesh::Mesh,
    pass::{
        LoadOp, RenderPassColorAttachmentDescriptor, RenderPassDepthStencilAttachmentDescriptor,
        StoreOp, TextureAttachment,
    },
    pipeline::{
        PipelineAssignments, PipelineCompiler, PipelineDescriptor, VertexBufferDescriptors,
    },
    render_graph::RenderGraph,
    render_resource::{
        entity_render_resource_assignments_system,
        resource_providers::{LightResourceProvider, UniformResourceProvider},
        AssetBatchers, EntityRenderResourceAssignments, RenderResourceAssignments,
    },
    shader::{uniforms::StandardMaterial, Shader},
    texture::Texture,
};

use bevy_app::{stage, AppBuilder, AppPlugin, GetEventReader};
use bevy_asset::AssetStorage;
use bevy_transform::prelude::LocalToWorld;
use bevy_window::{WindowCreated, WindowReference, WindowResized};
use pass::PassDescriptor;
use pipeline::pipelines::build_forward_pipeline;
use render_graph_2::{
    nodes::{Camera2dNode, CameraNode, PassNode, WindowSwapChainNode, WindowTextureNode},
    RenderGraph2,
};
use render_resource::resource_providers::mesh_resource_provider_system;
use texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage};

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
            .add_resource_provider(LightResourceProvider::new(10))
            .add_resource_provider(UniformResourceProvider::<StandardMaterial>::new(true))
            .add_resource_provider(UniformResourceProvider::<LocalToWorld>::new(true));
    }
}

impl AppPlugin for RenderPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        app.add_stage_after(stage::POST_UPDATE, RENDER_RESOURCE_STAGE)
            .add_stage_after(RENDER_RESOURCE_STAGE, RENDER_STAGE)
            // resources
            .add_resource(RenderGraph::default())
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
            .add_system_to_stage(stage::POST_UPDATE, mesh::mesh_specializer_system())
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
        let mut render_graph = RenderGraph2::default();
        // begin render graph 2
        {
            let resources = app.resources_mut();
            render_graph.add_system_node_named("camera", CameraNode::default(), resources);
            render_graph.add_system_node_named("camera2d", Camera2dNode::default(), resources);
            render_graph.add_node_named(
                "swapchain",
                WindowSwapChainNode::new(
                    WindowReference::Primary,
                    resources.get_event_reader::<WindowCreated>(),
                    resources.get_event_reader::<WindowResized>(),
                ),
            );
            render_graph.add_node_named(
                "main_pass_depth_texture",
                WindowTextureNode::new(
                    WindowReference::Primary,
                    TextureDescriptor {
                        size: Extent3d {
                            depth: 1,
                            width: 1,
                            height: 1,
                        },
                        array_layer_count: 1,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: TextureDimension::D2,
                        format: TextureFormat::Depth32Float, // PERF: vulkan recommends using 24 bit depth for better performance
                        usage: TextureUsage::OUTPUT_ATTACHMENT,
                    },
                    resources.get_event_reader::<WindowCreated>(),
                    resources.get_event_reader::<WindowResized>(),
                ),
            );
            let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();
            let mut pipelines = resources
                .get_mut::<AssetStorage<PipelineDescriptor>>()
                .unwrap();
            let mut main_pass = PassNode::new(PassDescriptor {
                color_attachments: vec![RenderPassColorAttachmentDescriptor {
                    attachment: TextureAttachment::Input("color".to_string()),
                    resolve_target: None,
                    load_op: LoadOp::Clear,
                    store_op: StoreOp::Store,
                    clear_color: Color::rgb(0.1, 0.1, 0.1),
                }],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachmentDescriptor {
                    attachment: TextureAttachment::Input("depth".to_string()),
                    depth_load_op: LoadOp::Clear,
                    depth_store_op: StoreOp::Store,
                    stencil_load_op: LoadOp::Clear,
                    stencil_store_op: StoreOp::Store,
                    clear_depth: 1.0,
                    clear_stencil: 0,
                }),
                sample_count: 1,
            });
            main_pass.add_pipeline(
                pipelines.add_default(build_forward_pipeline(&mut shaders)),
                vec![Box::new(AssignedMeshesDrawTarget)],
            );
            render_graph.add_node_named("main_pass", main_pass);

            // TODO: replace these with "autowire" groups
            render_graph.add_node_edge("camera", "main_pass").unwrap();
            render_graph.add_node_edge("camera2d", "main_pass").unwrap();
            render_graph
                .add_slot_edge(
                    "swapchain",
                    WindowSwapChainNode::OUT_TEXTURE,
                    "main_pass",
                    "color",
                )
                .unwrap();
            render_graph
                .add_slot_edge(
                    "main_pass_depth_texture",
                    WindowTextureNode::OUT_TEXTURE,
                    "main_pass",
                    "depth",
                )
                .unwrap();
        }
        app.add_resource(render_graph);
        // end render graph 2
    }
}
