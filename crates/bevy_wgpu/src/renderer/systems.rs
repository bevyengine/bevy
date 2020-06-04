use bevy_asset::Assets;
use bevy_render::{
    pipeline::{PipelineAssignments, PipelineCompiler, PipelineDescriptor},
    render_resource::EntityRenderResourceAssignments,
    Renderable,
};
use legion::prelude::*;

// TODO: replace with system_fn once configurable "archetype access" is sorted out
// pub fn render_resource_sets_system(
//     world: &mut SubWorld,
//     pipelines: Res<Assets<PipelineDescriptor>>,
//     pipeline_compiler: Res<PipelineCompiler>,
//     pipeline_assignments: Res<PipelineAssignments>,
//     entity_render_resource_assignments: Res<EntityRenderResourceAssignments>,
//     query: &mut Query<Write<Renderable>>, // gives SubWorld write access to Renderable
// ) {
//     // PERF: consider doing a par-iter over all renderable components so this can be parallelized
//     for compiled_pipeline_handle in pipeline_compiler.iter_all_compiled_pipelines() {
//         if let Some(compiled_pipeline_assignments) = pipeline_assignments
//             .assignments
//             .get(compiled_pipeline_handle)
//         {
//             let compiled_pipeline = pipelines.get(compiled_pipeline_handle).unwrap();
//             let pipeline_layout = compiled_pipeline.get_layout().unwrap();

//             for assignment_id in compiled_pipeline_assignments.iter() {
//                 let entity = entity_render_resource_assignments
//                     .get(*assignment_id)
//                     .unwrap();
//                 let mut renderable = world.get_component_mut::<Renderable>(*entity).unwrap();
//                 if !renderable.is_visible || renderable.is_instanced {
//                     continue;
//                 }

//                 for bind_group in pipeline_layout.bind_groups.iter() {
//                     renderable
//                         .render_resource_assignments
//                         .update_render_resource_set_id(bind_group);
//                 }
//             }
//         }
//     }
// }

pub fn render_resource_sets_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("update_render_resource_sets")
        .read_resource::<Assets<PipelineDescriptor>>()
        .read_resource::<PipelineCompiler>()
        .read_resource::<PipelineAssignments>()
        .read_resource::<EntityRenderResourceAssignments>()
        .write_component::<Renderable>()
        .build(
            |_,
             world,
             (
                pipelines,
                pipeline_compiler,
                pipeline_assignments,
                entity_render_resource_assignments,
            ),
             _| {
                // PERF: consider doing a par-iter over all renderable components so this can be parallelized
                for compiled_pipeline_handle in pipeline_compiler.iter_all_compiled_pipelines() {
                    if let Some(compiled_pipeline_assignments) = pipeline_assignments
                        .assignments
                        .get(compiled_pipeline_handle)
                    {
                        let compiled_pipeline = pipelines.get(compiled_pipeline_handle).unwrap();
                        let pipeline_layout = compiled_pipeline.get_layout().unwrap();

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
                            }
                        }
                    }
                }
            },
        )
}
