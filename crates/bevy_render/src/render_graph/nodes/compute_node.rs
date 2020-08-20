use crate::{
    dispatch::{ComputeCommand, Dispatch},
    pipeline::ComputePipelineDescriptor,
    render_graph::{Node, ResourceSlots},
    renderer::{BindGroupId, RenderContext},
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Resources, World};

pub struct ComputeNode;

impl ComputeNode {
    pub fn new() -> Self {
        ComputeNode {}
    }
}

impl Default for ComputeNode {
    fn default() -> Self {
        Self::new()
    }
}

impl Node for ComputeNode {
    fn update(
        &mut self,
        world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let pipelines = resources
            .get::<Assets<ComputePipelineDescriptor>>()
            .unwrap();
        render_context.begin_compute_pass(&mut |compute_pass| {
            let mut compute_state = ComputeState::default();

            let mut entities = world.query::<&Dispatch>();

            for dispatch in entities.iter() {
                for compute_command in dispatch.compute_commands.iter() {
                    match compute_command {
                        ComputeCommand::SetPipeline { pipeline } => {
                            // TODO: Filter pipelines
                            compute_pass.set_pipeline(*pipeline);
                            let descriptor = pipelines.get(pipeline).unwrap();
                            compute_state.set_pipeline(*pipeline, descriptor);
                        }
                        ComputeCommand::SetBindGroup {
                            index,
                            bind_group,
                            dynamic_uniform_indices,
                        } => {
                            let pipeline = pipelines.get(&compute_state.pipeline.unwrap()).unwrap();
                            let layout = pipeline.get_layout().unwrap();
                            let bind_group_descriptor = layout.get_bind_group(*index).unwrap();
                            compute_pass.set_bind_group(
                                *index,
                                bind_group_descriptor.id,
                                *bind_group,
                                dynamic_uniform_indices
                                    .as_ref()
                                    .map(|indices| indices.as_slice()),
                            );
                            compute_state.set_bind_group(*index, *bind_group);
                        }
                        ComputeCommand::Dispatch { x, y, z } => {
                            compute_pass.dispatch(*x, *y, *z);
                        }
                    }
                }
            }
        });
    }
}

/// Tracks the current pipeline state to ensure compute calls are valid.
#[derive(Default)]
struct ComputeState {
    pipeline: Option<Handle<ComputePipelineDescriptor>>,
    bind_groups: Vec<Option<BindGroupId>>,
}

impl ComputeState {
    pub fn set_bind_group(&mut self, index: u32, bind_group: BindGroupId) {
        self.bind_groups[index as usize] = Some(bind_group);
    }

    pub fn set_pipeline(
        &mut self,
        handle: Handle<ComputePipelineDescriptor>,
        descriptor: &ComputePipelineDescriptor,
    ) {
        self.bind_groups.clear();
        self.pipeline = Some(handle);
        let layout = descriptor.get_layout().unwrap();
        self.bind_groups.resize(layout.bind_groups.len(), None);
    }
}
