use crate::{
    pipeline::{
        PipelineCompiler, PipelineDescriptor, PipelineLayout, PipelineSpecialization,
        VertexBufferDescriptors, ComputePipelineDescriptor,
    },
    renderer::{
        BindGroup, BindGroupId, BufferId, BufferUsage, RenderResource, RenderResourceBinding,
        RenderResourceBindings, RenderResourceContext, SharedBuffers,
    },
    shader::Shader,
};
use bevy_asset::{Assets, Handle};
use bevy_ecs::{
    FetchResource, Query, Res, ResMut, ResourceIndex, ResourceQuery, Resources, SystemId,
    TypeAccess, UnsafeClone,
};
use bevy_property::Properties;
use std::{any::TypeId, ops::Range, sync::Arc};
use thiserror::Error;

/// A queued command for the compute renderer
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ComputeCommand {
    SetPipeline {
        pipeline: Handle<PipelineDescriptor>,
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

/// A component that indicates how to dispatch a compute shader.
#[derive(Properties)]
pub struct Dispatch {
    #[property(ignore)]
    pub compute_commands: Vec<ComputeCommand>,
}

impl Default for Dispatch {
    fn default() -> Self {
        Self {
            compute_commands: Default::default(),
        }
    }
}

impl Dispatch {
    pub fn clear_compute_commands(&mut self) {
        self.compute_commands.clear();
    }

    pub fn set_pipeline(&mut self, pipeline: Handle<PipelineDescriptor>) {
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
        self.compute_commands.push(compute_command);
    }
}

#[derive(Debug, Error)]
pub enum ComputeError {
    #[error("Pipeline does not exist.")]
    NonExistentPipeline,
    #[error("No pipeline set")]
    NoPipelineSet,
    #[error("Pipeline has no layout")]
    PipelineHasNoLayout,
    #[error("Failed to get a buffer for the given RenderResource.")]
    BufferAllocationFailure,
}

pub struct ComputeContext<'a> {
    pub pipelines: ResMut<'a, Assets<ComputePipelineDescriptor>>,
    pub shaders: ResMut<'a, Assets<Shader>>,
    pub pipeline_compiler: ResMut<'a, PipelineCompiler>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub shared_buffers: Res<'a, SharedBuffers>,
    pub current_pipeline: Option<Handle<ComputePipelineDescriptor>>,
}

impl<'a> UnsafeClone for ComputeContext<'a> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            pipelines: self.pipelines.unsafe_clone(),
            shaders: self.shaders.unsafe_clone(),
            pipeline_compiler: self.pipeline_compiler.unsafe_clone(),
            render_resource_context: self.render_resource_context.unsafe_clone(),
            shared_buffers: self.shared_buffers.unsafe_clone(),
            current_pipeline: self.current_pipeline.clone(),
        }
    }
}

impl<'a> ResourceQuery for ComputeContext<'a> {
    type Fetch = FetchComputeContext;
}

pub struct FetchComputeContext;

// TODO: derive this impl
impl<'a> FetchResource<'a> for FetchComputeContext {
    type Item = ComputeContext<'a>;

    fn borrow(resources: &Resources) {
        resources.borrow_mut::<Assets<PipelineDescriptor>>();
        resources.borrow_mut::<Assets<Shader>>();
        resources.borrow_mut::<PipelineCompiler>();
        resources.borrow::<Box<dyn RenderResourceContext>>();
        resources.borrow::<VertexBufferDescriptors>();
        resources.borrow::<SharedBuffers>();
    }

    fn release(resources: &Resources) {
        resources.release_mut::<Assets<PipelineDescriptor>>();
        resources.release_mut::<Assets<Shader>>();
        resources.release_mut::<PipelineCompiler>();
        resources.release::<Box<dyn RenderResourceContext>>();
        resources.release::<VertexBufferDescriptors>();
        resources.release::<SharedBuffers>();
    }

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        ComputeContext {
            pipelines: ResMut::new(
                resources.get_unsafe_ref::<Assets<ComputePipelineDescriptor>>(ResourceIndex::Global),
            ),
            shaders: ResMut::new(resources.get_unsafe_ref::<Assets<Shader>>(ResourceIndex::Global)),
            pipeline_compiler: ResMut::new(
                resources.get_unsafe_ref::<PipelineCompiler>(ResourceIndex::Global),
            ),
            render_resource_context: Res::new(
                resources.get_unsafe_ref::<Box<dyn RenderResourceContext>>(ResourceIndex::Global),
            ),
            shared_buffers: Res::new(
                resources.get_unsafe_ref::<SharedBuffers>(ResourceIndex::Global),
            ),
            current_pipeline: None,
        }
    }

    fn access() -> TypeAccess {
        let mut access = TypeAccess::default();
        access
            .mutable
            .insert(TypeId::of::<Assets<ComputePipelineDescriptor>>());
        access.mutable.insert(TypeId::of::<Assets<Shader>>());
        access.mutable.insert(TypeId::of::<PipelineCompiler>());
        access
            .immutable
            .insert(TypeId::of::<Box<dyn RenderResourceContext>>());
        access.immutable.insert(TypeId::of::<SharedBuffers>());
        access
    }
}

impl<'a> ComputeContext<'a> {
    pub fn get_uniform_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
    ) -> Result<RenderResourceBinding, ComputeError> {
        self.get_buffer(render_resource, BufferUsage::UNIFORM)
    }

    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Result<RenderResourceBinding, ComputeError> {
        self.shared_buffers
            .get_buffer(render_resource, buffer_usage)
            .ok_or_else(|| ComputeError::BufferAllocationFailure)
    }

    pub fn set_pipeline(
        &mut self,
        dispatch: &mut Dispatch,
        pipeline_handle: Handle<PipelineDescriptor>,
        specialization: &PipelineSpecialization,
    ) -> Result<(), ComputeError> {
        todo!();
        // let specialized_pipeline = if let Some(specialized_pipeline) = self
        //     .pipeline_compiler
        //     .get_specialized_pipeline(pipeline_handle, specialization)
        // {
        //     specialized_pipeline
        // } else {
        //     self.pipeline_compiler.compile_pipeline(
        //         &**self.render_resource_context,
        //         &mut self.pipelines,
        //         &mut self.shaders,
        //         pipeline_handle,
        //         &self.vertex_buffer_descriptors,
        //         specialization,
        //     )
        // };

        // dispatch.set_pipeline(specialized_pipeline);
        // self.current_pipeline = Some(specialized_pipeline);
        Ok(())
    }

    pub fn get_pipeline_descriptor(&self) -> Result<&ComputePipelineDescriptor, ComputeError> {
        self.current_pipeline
            .and_then(|handle| self.pipelines.get(&handle))
            .ok_or_else(|| ComputeError::NoPipelineSet)
    }

    pub fn get_pipeline_layout(&self) -> Result<&PipelineLayout, ComputeError> {
        self.get_pipeline_descriptor().and_then(|descriptor| {
            descriptor
                .get_layout()
                .ok_or_else(|| ComputeError::PipelineHasNoLayout)
        })
    }

    pub fn set_bind_groups_from_bindings(
        &self,
        dispatch: &mut Dispatch,
        render_resource_bindings: &mut [&mut RenderResourceBindings],
    ) -> Result<(), ComputeError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| ComputeError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| ComputeError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| ComputeError::PipelineHasNoLayout)?;
        for bindings in render_resource_bindings.iter_mut() {
            todo!();
            //bindings.update_bind_groups(pipeline_descriptor, &**self.render_resource_context);
        }
        for bind_group_descriptor in layout.bind_groups.iter() {
            for bindings in render_resource_bindings.iter_mut() {
                if let Some(bind_group) =
                    bindings.get_descriptor_bind_group(bind_group_descriptor.id)
                {
                    dispatch.set_bind_group(bind_group_descriptor.index, bind_group);
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn create_bind_group_resource(
        &self,
        index: u32,
        bind_group: &BindGroup,
    ) -> Result<(), ComputeError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| ComputeError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| ComputeError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| ComputeError::PipelineHasNoLayout)?;
        let bind_group_descriptor = &layout.bind_groups[index as usize];
        self.render_resource_context
            .create_bind_group(bind_group_descriptor.id, bind_group);
        Ok(())
    }
}