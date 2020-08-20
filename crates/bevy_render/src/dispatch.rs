use crate::{
    pipeline::{
        ComputePipelineCompiler, ComputePipelineDescriptor, ComputePipelineSpecialization,
        PipelineLayout,
    },
    renderer::{
        BindGroup, BindGroupId, BufferUsage, RenderResource, RenderResourceBinding,
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
use std::{any::TypeId, sync::Arc};
use thiserror::Error;

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

/// A component that defines compute dispatching.
#[derive(Properties)]
pub struct Dispatch {
    pub only_once: bool,
    pub work_group_size_x: u32,
    pub work_group_size_y: u32,
    pub work_group_size_z: u32,
    #[property(ignore)]
    pub compute_commands: Vec<ComputeCommand>,
    #[property(ignore)]
    pub has_run: bool,
}

impl Default for Dispatch {
    fn default() -> Self {
        Self {
            only_once: true,
            work_group_size_x: 1,
            work_group_size_y: 1,
            work_group_size_z: 1,
            compute_commands: Default::default(),
            has_run: false,
        }
    }
}

impl Dispatch {
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
    pub fn dispatch(&mut self) {
        self.compute_command(ComputeCommand::Dispatch {
            x: self.work_group_size_x,
            y: self.work_group_size_y,
            z: self.work_group_size_z,
        });
        if self.only_once {
            self.has_run = true;
        }
    }

    #[inline]
    pub fn compute_command(&mut self, compute_command: ComputeCommand) {
        if !self.has_run {
            self.compute_commands.push(compute_command);
        }
    }
}

#[derive(Debug, Error)]
pub enum DispatchError {
    #[error("Pipeline does not exist.")]
    NonExistentPipeline,
    #[error("No pipeline set")]
    NoPipelineSet,
    #[error("Pipeline has no layout")]
    PipelineHasNoLayout,
    #[error("Failed to get a buffer for the given RenderResource.")]
    BufferAllocationFailure,
}

pub struct DispatchContext<'a> {
    pub pipelines: ResMut<'a, Assets<ComputePipelineDescriptor>>,
    pub shaders: ResMut<'a, Assets<Shader>>,
    pub pipeline_compiler: ResMut<'a, ComputePipelineCompiler>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub shared_buffers: Res<'a, SharedBuffers>,
    pub current_pipeline: Option<Handle<ComputePipelineDescriptor>>,
}

impl<'a> UnsafeClone for DispatchContext<'a> {
    unsafe fn unsafe_clone(&self) -> Self {
        Self {
            pipelines: self.pipelines.unsafe_clone(),
            shaders: self.shaders.unsafe_clone(),
            pipeline_compiler: self.pipeline_compiler.unsafe_clone(),
            render_resource_context: self.render_resource_context.unsafe_clone(),
            shared_buffers: self.shared_buffers.unsafe_clone(),
            current_pipeline: self.current_pipeline,
        }
    }
}

impl<'a> ResourceQuery for DispatchContext<'a> {
    type Fetch = FetchDispatchContext;
}

pub struct FetchDispatchContext;

// TODO: derive this impl
impl<'a> FetchResource<'a> for FetchDispatchContext {
    type Item = DispatchContext<'a>;

    fn borrow(resources: &Resources) {
        resources.borrow_mut::<Assets<ComputePipelineDescriptor>>();
        resources.borrow_mut::<Assets<Shader>>();
        resources.borrow_mut::<ComputePipelineCompiler>();
        resources.borrow::<Box<dyn RenderResourceContext>>();
        resources.borrow::<SharedBuffers>();
    }

    fn release(resources: &Resources) {
        resources.release_mut::<Assets<ComputePipelineDescriptor>>();
        resources.release_mut::<Assets<Shader>>();
        resources.release_mut::<ComputePipelineCompiler>();
        resources.release::<Box<dyn RenderResourceContext>>();
        resources.release::<SharedBuffers>();
    }

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        DispatchContext {
            pipelines: ResMut::new(
                resources
                    .get_unsafe_ref::<Assets<ComputePipelineDescriptor>>(ResourceIndex::Global),
            ),
            shaders: ResMut::new(resources.get_unsafe_ref::<Assets<Shader>>(ResourceIndex::Global)),
            pipeline_compiler: ResMut::new(
                resources.get_unsafe_ref::<ComputePipelineCompiler>(ResourceIndex::Global),
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
        access
            .mutable
            .insert(TypeId::of::<ComputePipelineCompiler>());
        access
            .immutable
            .insert(TypeId::of::<Box<dyn RenderResourceContext>>());
        access.immutable.insert(TypeId::of::<SharedBuffers>());
        access
    }
}

impl<'a> DispatchContext<'a> {
    pub fn get_uniform_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
    ) -> Result<RenderResourceBinding, DispatchError> {
        self.get_buffer(render_resource, BufferUsage::UNIFORM)
    }

    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Result<RenderResourceBinding, DispatchError> {
        self.shared_buffers
            .get_buffer(render_resource, buffer_usage)
            .ok_or_else(|| DispatchError::BufferAllocationFailure)
    }

    pub fn set_pipeline(
        &mut self,
        dispatch: &mut Dispatch,
        pipeline_handle: Handle<ComputePipelineDescriptor>,
        specialization: &ComputePipelineSpecialization,
    ) -> Result<(), DispatchError> {
        let specialized_pipeline = if let Some(specialized_pipeline) = self
            .pipeline_compiler
            .get_specialized_pipeline(pipeline_handle, specialization)
        {
            specialized_pipeline
        } else {
            self.pipeline_compiler.compile_pipeline(
                &**self.render_resource_context,
                &mut self.pipelines,
                &mut self.shaders,
                pipeline_handle,
                specialization,
            )
        };

        dispatch.set_pipeline(specialized_pipeline);
        self.current_pipeline = Some(specialized_pipeline);
        Ok(())
    }

    pub fn get_pipeline_descriptor(&self) -> Result<&ComputePipelineDescriptor, DispatchError> {
        self.current_pipeline
            .and_then(|handle| self.pipelines.get(&handle))
            .ok_or_else(|| DispatchError::NoPipelineSet)
    }

    pub fn get_pipeline_layout(&self) -> Result<&PipelineLayout, DispatchError> {
        self.get_pipeline_descriptor().and_then(|descriptor| {
            descriptor
                .get_layout()
                .ok_or_else(|| DispatchError::PipelineHasNoLayout)
        })
    }

    pub fn set_bind_groups_from_bindings(
        &self,
        dispatch: &mut Dispatch,
        render_resource_bindings: &mut [&mut RenderResourceBindings],
    ) -> Result<(), DispatchError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| DispatchError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| DispatchError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| DispatchError::PipelineHasNoLayout)?;
        for bindings in render_resource_bindings.iter_mut() {
            bindings.update_bind_groups(
                pipeline_descriptor.get_layout().unwrap(),
                &**self.render_resource_context,
            );
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
    ) -> Result<(), DispatchError> {
        let pipeline = self
            .current_pipeline
            .ok_or_else(|| DispatchError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(&pipeline)
            .ok_or_else(|| DispatchError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or_else(|| DispatchError::PipelineHasNoLayout)?;
        let bind_group_descriptor = &layout.bind_groups[index as usize];
        self.render_resource_context
            .create_bind_group(bind_group_descriptor.id, bind_group);
        Ok(())
    }
}

pub fn clear_compute_commands(mut query: Query<&mut Dispatch>) {
    for mut dispatch in &mut query.iter() {
        dispatch.clear_compute_commands();
    }
}
