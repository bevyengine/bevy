use bevy_asset::AssetStorage;
use bevy_render::{
    pipeline::{PipelineAssignments, PipelineCompiler, PipelineDescriptor},
    render_graph::RenderGraph,
    render_resource::{EntityRenderResourceAssignments, RenderResourceAssignments},
    Renderable,
};
use legion::prelude::*;

pub fn render_resource_sets_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_render_resource_sets")
        .read_resource::<RenderGraph>()
        .read_resource::<AssetStorage<PipelineDescriptor>>()
        .read_resource::<PipelineCompiler>()
        .read_resource::<PipelineAssignments>()
        .read_resource::<EntityRenderResourceAssignments>()
        .write_resource::<RenderResourceAssignments>()
        .write_component::<Renderable>()
        .build(
            |_,
             world,
             (
                render_graph,
                pipelines,
                pipeline_compiler,
                pipeline_assignments,
                entity_render_resource_assignments,
                global_render_resource_assignments,
            ),
             _| {
                // PERF: consider doing a par-iter over all renderable components so this can be parallelized
                for handle in render_graph.pipeline_descriptors.iter() {
                    for compiled_pipeline_handle in
                        pipeline_compiler.iter_compiled_pipelines(*handle).unwrap()
                    {
                        if let Some(compiled_pipeline_assignments) = pipeline_assignments
                            .assignments
                            .get(compiled_pipeline_handle)
                        {
                            let compiled_pipeline =
                                pipelines.get(compiled_pipeline_handle).unwrap();
                            let pipeline_layout = compiled_pipeline.get_layout().unwrap();

                            for bind_group in pipeline_layout.bind_groups.iter() {
                                global_render_resource_assignments
                                    .update_render_resource_set_id(bind_group);
                            }

                            for assignment_id in compiled_pipeline_assignments.iter() {
                                let entity = entity_render_resource_assignments
                                    .get(*assignment_id)
                                    .unwrap();
                                let mut renderable =
                                    world.get_component_mut::<Renderable>(*entity).unwrap();
                                if !renderable.is_visible || renderable.is_instanced {
                                    continue;
                                }

                                for bind_group in pipeline_layout.bind_groups.iter() {
                                    renderable
                                        .render_resource_assignments
                                        .update_render_resource_set_id(bind_group);
                                    // TODO: also setup bind groups here if possible
                                }
                            }
                        }
                    }
                }
            },
        )
}
