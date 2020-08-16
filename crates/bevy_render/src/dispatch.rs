use crate::{
    pipeline::{
        ComputePipelineDescriptor,
    },
    renderer::{
        BindGroup, BindGroupId,
    },
};
use bevy_asset::{Handle};
use bevy_ecs::{
    ResMut
};
use std::{sync::Arc};

/// A queued command for the compute renderer
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ComputeCommand {
    SetPipeline {
        pipeline: Handle<ComputePipelineDescriptor>,
    },
    SetBindGroup {
        index: u32,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<Arc<Vec<u32>>>,
    },
    Dispatch {
        x: u32,
        y: u32,
        z: u32,
    },
}

/// A resource that records compute commands.
pub struct DispatchResource {
    pub compute_commands: Vec<ComputeCommand>,
}

impl Default for DispatchResource {
    fn default() -> Self {
        Self {
            compute_commands: Default::default(),
        }
    }
}

impl DispatchResource {
    pub fn clear_compute_commands(&mut self) {
        self.compute_commands.clear();
    }

    pub fn set_pipeline(&mut self, pipeline: Handle<ComputePipelineDescriptor>) {
        self.compute_command(ComputeCommand::SetPipeline { pipeline });
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup) {
        self.compute_command(ComputeCommand::SetBindGroup {
            index,
            bind_group: bind_group.id,
            dynamic_uniform_indices: bind_group.dynamic_uniform_indices.clone(),
        });
    }

    /// Dispatches compute work operations.
    /// x, y and z denote the number of work groups to dispatch in each dimension.
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        self.compute_command(ComputeCommand::Dispatch {
            x,
            y,
            z
        });
    }

    #[inline]
    pub fn compute_command(&mut self, compute_command: ComputeCommand) {
        dbg!("Adding compute command!");
        self.compute_commands.push(compute_command);
    }
}

pub fn clear_compute_commands(mut dispatch_resource: ResMut<DispatchResource>) {
    dispatch_resource.clear_compute_commands();
}