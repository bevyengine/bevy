use crate::{
    asset::Handle,
    legion::prelude::*,
    render::{
        draw_target::DrawTarget,
        pipeline::PipelineDescriptor,
        render_resource::{resource_name, AssetBatchers},
        renderer::{RenderPass, Renderer},
    },
};

#[derive(Default)]
pub struct AssignedBatchesDrawTarget;

impl DrawTarget for AssignedBatchesDrawTarget {
    fn draw(
        &self,
        _world: &World,
        resources: &Resources,
        _render_pass: &mut dyn RenderPass,
        _pipeline_handle: Handle<PipelineDescriptor>,
    ) {
        let asset_batches = resources.get::<AssetBatchers>().unwrap();
        // let renderer = render_pass.get_renderer();
        // println!("Drawing batches");
        for batch in asset_batches.get_batches() {
            // render_resources.get
            // println!("{:?}", batch);
        }

        // println!();
        // println!();
        // println!();
    }

    fn setup(
        &mut self,
        _world: &World,
        _resources: &Resources,
        _renderer: &mut dyn Renderer,
        _pipeline_handle: Handle<PipelineDescriptor>,
    ) {
    }

    fn get_name(&self) -> String {
        resource_name::draw_target::ASSIGNED_BATCHES.to_string()
    }
}
