use crate::{
    pipeline::{PipelineCompiler, PipelineDescriptor, RenderPipelines},
    render_resource::{BindGroupStatus, RenderResourceBindings},
    renderer::RenderResourceContext,
};
use bevy_asset::Assets;
use legion::prelude::*;

fn update_bind_groups(
    pipeline: &PipelineDescriptor,
    render_resource_bindings: &mut RenderResourceBindings,
    render_resource_context: &dyn RenderResourceContext,
) {
    let layout = pipeline.get_layout().unwrap();
    for bind_group_descriptor in layout.bind_groups.iter() {
        match render_resource_bindings.update_bind_group(bind_group_descriptor) {
            BindGroupStatus::Changed(id) => {
                let bind_group = render_resource_bindings
                    .get_bind_group(id)
                    .expect("RenderResourceSet was just changed, so it should exist");
                render_resource_context.create_bind_group(bind_group_descriptor.id, bind_group);
            }
            // TODO: Don't re-create bind groups if they havent changed. this will require cleanup of orphan bind groups and
            // removal of global context.clear_bind_groups()
            // PERF: see above
            BindGroupStatus::Unchanged(id) => {
                let bind_group = render_resource_bindings
                    .get_bind_group(id)
                    .expect("RenderResourceSet was just changed, so it should exist");
                render_resource_context.create_bind_group(bind_group_descriptor.id, bind_group);
            }
            BindGroupStatus::NoMatch => {
                // ignore unchanged / unmatched render resource sets
            }
        }
    }
}

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
        update_bind_groups(
            pipeline,
            &mut render_resource_bindings,
            render_resource_context,
        )
    }
    for mut render_pipelines in query.iter_mut(world) {
        let render_pipelines = render_pipelines.as_mut();
        for render_pipeline in render_pipelines.pipelines.iter_mut() {
            let pipeline = pipelines.get(&render_pipeline.specialized_pipeline.unwrap()).unwrap();
            update_bind_groups(
                pipeline,
                &mut render_pipelines.bindings,
                render_resource_context,
            )
        }
    }
}
