use bevy_asset::Assets;
use crate::{
    draw::RenderPipelines,
    pipeline::{PipelineCompiler, PipelineDescriptor},
    render_resource::{RenderResourceAssignments, RenderResourceSetStatus},
    renderer::{RenderResourceContext, RenderResources},
};
use legion::prelude::*;

fn update_bind_groups(
    pipeline: &PipelineDescriptor,
    render_resource_assignments: &mut RenderResourceAssignments,
    render_resource_context: &dyn RenderResourceContext,
) {
    let layout = pipeline.get_layout().unwrap();
    for bind_group in layout.bind_groups.iter() {
        match render_resource_assignments.update_bind_group(bind_group) {
            RenderResourceSetStatus::Changed(id) => {
                let render_resource_set = render_resource_assignments
                    .get_render_resource_set(id)
                    .expect("RenderResourceSet was just changed, so it should exist");
                render_resource_context.create_bind_group(bind_group.id, render_resource_set);
            },
            // TODO: Don't re-create bind groups if they havent changed. this will require cleanup of orphan bind groups and
            // removal of global context.clear_bind_groups()
            // PERF: see above
            RenderResourceSetStatus::Unchanged(id) => {
                let render_resource_set = render_resource_assignments
                    .get_render_resource_set(id)
                    .expect("RenderResourceSet was just changed, so it should exist");
                render_resource_context.create_bind_group(bind_group.id, render_resource_set);
            },
            RenderResourceSetStatus::NoMatch => {
                // ignore unchanged / unmatched render resource sets
            }
        }
    }
}

pub fn render_resource_sets_system(
    world: &mut SubWorld,
    pipelines: Res<Assets<PipelineDescriptor>>,
    pipeline_compiler: Res<PipelineCompiler>,
    render_resources: Res<RenderResources>,
    mut render_resource_assignments: ResMut<RenderResourceAssignments>,
    query: &mut Query<Write<RenderPipelines>>,
) {
    let render_resource_context = &*render_resources.context;
    for compiled_pipeline_handle in pipeline_compiler.iter_all_compiled_pipelines() {
        let pipeline = pipelines.get(compiled_pipeline_handle).unwrap();
        update_bind_groups(
            pipeline,
            &mut render_resource_assignments,
            render_resource_context,
        )
    }
    for mut render_pipelines in query.iter_mut(world) {
        let render_pipelines = render_pipelines.as_mut();
        for pipeline in render_pipelines.compiled_pipelines.iter() {
            let pipeline = pipelines.get(pipeline).unwrap();
            update_bind_groups(
                pipeline,
                &mut render_pipelines.render_resource_assignments,
                render_resource_context,
            )
        }
    }
}
