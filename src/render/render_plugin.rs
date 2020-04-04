use super::{
    draw_target::draw_targets::{
        AssignedBatchesDrawTarget, AssignedMeshesDrawTarget, MeshesDrawTarget, UiDrawTarget,
    },
    pass::passes::ForwardPassBuilder,
    pipeline::{pipelines::ForwardPipelineBuilder, PipelineCompiler, ShaderPipelineAssignments},
    render_graph::RenderGraph,
    render_resource::{
        build_entity_render_resource_assignments_system,
        resource_providers::{
            Camera2dResourceProvider, CameraResourceProvider, LightResourceProvider,
            MeshResourceProvider, UiResourceProvider,
        },
        AssetBatchers, EntityRenderResourceAssignments, RenderResourceAssignments,
    },
};
use crate::{prelude::*, window::WindowResized};

#[derive(Default)]
pub struct RenderPlugin;

impl RenderPlugin {
    pub fn setup_render_graph_defaults(app: &mut AppBuilder) {
        let mut pipelines = app
            .resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut shaders = app.resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut render_graph = app.resources.get_mut::<RenderGraph>().unwrap();
        render_graph
            .build(&mut pipelines, &mut shaders)
            .add_draw_target(MeshesDrawTarget::default())
            .add_draw_target(AssignedBatchesDrawTarget::default())
            .add_draw_target(AssignedMeshesDrawTarget::default())
            .add_draw_target(UiDrawTarget::default())
            .add_resource_provider(CameraResourceProvider::new(
                app.resources.get_event_reader::<WindowResized>(),
            ))
            .add_resource_provider(Camera2dResourceProvider::new(
                app.resources.get_event_reader::<WindowResized>(),
            ))
            .add_resource_provider(LightResourceProvider::new(10))
            .add_resource_provider(UiResourceProvider::new())
            .add_resource_provider(MeshResourceProvider::new())
            .add_resource_provider(UniformResourceProvider::<StandardMaterial>::new(true))
            .add_resource_provider(UniformResourceProvider::<LocalToWorld>::new(true))
            .add_forward_pass()
            .add_forward_pipeline();
    }
}

impl AppPlugin for RenderPlugin {
    fn build(&self, mut app: AppBuilder) -> AppBuilder {
        let mut asset_batchers = AssetBatchers::default();
        asset_batchers.batch_types2::<Mesh, StandardMaterial>();
        app = app
            .add_system(build_entity_render_resource_assignments_system())
            .add_resource(RenderGraph::default())
            .add_resource(AssetStorage::<Mesh>::new())
            .add_resource(AssetStorage::<Texture>::new())
            .add_resource(AssetStorage::<Shader>::new())
            .add_resource(AssetStorage::<StandardMaterial>::new())
            .add_resource(AssetStorage::<PipelineDescriptor>::new())
            .add_resource(ShaderPipelineAssignments::new())
            .add_resource(PipelineCompiler::new())
            .add_resource(RenderResourceAssignments::default())
            .add_resource(EntityRenderResourceAssignments::default())
            .add_resource(asset_batchers);
        RenderPlugin::setup_render_graph_defaults(&mut app);
        app
    }

    fn name(&self) -> &'static str {
        "Render"
    }
}
