use crate::{
    pipeline::{PipelineCompiler, PipelineDescriptor, RenderPipelines},
    render_resource::RenderResourceBindings,
    renderer::RenderResourceContext,
};
use bevy_asset::Assets;
use legion::prelude::*;

pub fn bind_groups_system(
    world: &mut SubWorld,
    pipelines: Res<Assets<PipelineDescriptor>>,
    pipeline_compiler: Res<PipelineCompiler>,
    render_resource_context: Res<Box<dyn RenderResourceContext>>,
    mut render_resource_bindings: ResMut<RenderResourceBindings>,
    query: &mut Query<Write<RenderPipelines>>,
) {
    let render_resource_context = &**render_resource_context;
    for compiled_pipeline_handle in pipeline_compiler.iter_all_compiled_pipelines() {
        let pipeline = pipelines.get(compiled_pipeline_handle).unwrap();
        render_resource_bindings.update_bind_groups(pipeline, render_resource_context);
    }
    for mut render_pipelines in query.iter_mut(world) {
        let render_pipelines = render_pipelines.as_mut();
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            let pipeline = pipelines
                .get(&render_pipeline.specialized_pipeline.unwrap())
                .unwrap();
            render_pipelines.bindings.update_bind_groups(pipeline, render_resource_context);
        }
    }
}
