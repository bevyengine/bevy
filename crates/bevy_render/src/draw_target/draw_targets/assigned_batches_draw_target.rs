use crate::{
    batch::AssetBatchers,
    draw_target::DrawTarget,
    pass::RenderPass,
    pipeline::PipelineDescriptor,
    render_resource::{resource_name, RenderResourceAssignments},
    renderer::RenderContext,
    Renderable,
};
use bevy_asset::Handle;
use legion::prelude::*;

#[derive(Default)]
pub struct AssignedBatchesDrawTarget;

impl DrawTarget for AssignedBatchesDrawTarget {
    fn draw(
        &self,
        world: &World,
        resources: &Resources,
        render_pass: &mut dyn RenderPass,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        log::trace!("drawing batches for pipeline {:?}", pipeline_handle);
        let asset_batches = resources.get::<AssetBatchers>().unwrap();
        let global_render_resource_assignments =
            resources.get::<RenderResourceAssignments>().unwrap();
        render_pass.set_render_resources(pipeline_descriptor, &global_render_resource_assignments);
        for batch in asset_batches.get_batches() {
            let indices = render_pass
                .set_render_resources(pipeline_descriptor, &batch.render_resource_assignments);
            log::trace!("drawing batch {:?}", batch.render_resource_assignments.id);
            log::trace!("{:#?}", batch);
            for batched_entity in batch.entities.iter() {
                let renderable = world.get_component::<Renderable>(*batched_entity).unwrap();
                if !renderable.is_visible {
                    continue;
                }

                log::trace!("start drawing batched entity: {:?}", batched_entity);
                log::trace!("{:#?}", renderable.render_resource_assignments);
                let entity_indices = render_pass.set_render_resources(
                    pipeline_descriptor,
                    &renderable.render_resource_assignments,
                );
                let mut draw_indices = &indices;
                if entity_indices.is_some() {
                    if indices.is_some() {
                        // panic!("entities overriding their batch's vertex buffer is not currently supported");
                        log::trace!("using batch vertex indices");
                        draw_indices = &entity_indices;
                    } else {
                        log::trace!("using entity vertex indices");
                        draw_indices = &entity_indices;
                    }
                }

                if draw_indices.is_none() {
                    continue;
                }

                render_pass.draw_indexed(draw_indices.as_ref().unwrap().clone(), 0, 0..1);
                log::trace!("finish drawing batched entity: {:?}", batched_entity);
            }
        }
    }

    fn setup(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
    ) {
        let asset_batches = resources.get::<AssetBatchers>().unwrap();

        let global_render_resource_assignments =
            resources.get_mut::<RenderResourceAssignments>().unwrap();

        log::trace!(
            "setting up batch bind groups for pipeline: {:?} {:?}",
            pipeline_handle,
            pipeline_descriptor.name,
        );
        log::trace!("setting up global bind groups");
        render_context.setup_bind_groups(pipeline_descriptor, &global_render_resource_assignments);

        for batch in asset_batches.get_batches() {
            log::trace!(
                "setting up batch bind groups: {:?}",
                batch.render_resource_assignments.id
            );
            log::trace!("{:#?}", batch);
            render_context
                .setup_bind_groups(pipeline_descriptor, &batch.render_resource_assignments);
            for batched_entity in batch.entities.iter() {
                let renderable = world.get_component::<Renderable>(*batched_entity).unwrap();
                if !renderable.is_visible || renderable.is_instanced {
                    continue;
                }

                log::trace!(
                    "setting up entity bind group {:?} for batch {:?}",
                    batched_entity,
                    batch.render_resource_assignments.id
                );
                render_context.setup_bind_groups(
                    pipeline_descriptor,
                    &renderable.render_resource_assignments,
                );
            }
        }
    }

    fn get_name(&self) -> String {
        resource_name::draw_target::ASSIGNED_BATCHES.to_string()
    }
}
