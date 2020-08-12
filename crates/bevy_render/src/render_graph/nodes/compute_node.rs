use crate::{
    pipeline::{
        ComputePipelineDescriptor,
    },
    render_graph::{Node, ResourceSlots},
    renderer::{
        BindGroupId, RenderContext,
    }, dispatch::ComputeCommand,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{Resources, World, HecsQuery};
use std::marker::PhantomData;


pub struct ComputeNode<Q: HecsQuery> {
    _marker: PhantomData<Q>,
}

impl<Q: HecsQuery> ComputeNode<Q> {
    pub fn new() -> Self {
        ComputeNode {
            _marker: PhantomData::default(),
        }
    }
}

impl<Q: HecsQuery + Send + Sync + 'static> Node for ComputeNode<Q> {
    fn update(
        &mut self,
        _world: &World,
        resources: &Resources,
        render_context: &mut dyn RenderContext,
        _input: &ResourceSlots,
        _output: &mut ResourceSlots,
    ) {
        let pipelines = resources.get::<Assets<ComputePipelineDescriptor>>().unwrap();
        
        render_context.begin_compute_pass(
            &mut |compute_pass| {
                // each Draw component contains an ordered list of render commands. we turn those into actual render commands here
                let compute_commands = Vec::<ComputeCommand>::new();
                let mut compute_state = ComputeState::default();
                for compute_command in compute_commands.iter() {
                    match compute_command {
                        ComputeCommand::SetPipeline { pipeline } => {
                            // TODO: Filter pipelines
                            compute_pass.set_pipeline(*pipeline);
                            let descriptor = pipelines.get(pipeline).unwrap();
                            compute_state.set_pipeline(*pipeline, descriptor);
                        },
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
        );
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
