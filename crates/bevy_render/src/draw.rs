use crate::{
    pipeline::{
        PipelineCompiler, PipelineDescriptor, PipelineLayout, PipelineSpecialization,
        VERTEX_FALLBACK_LAYOUT_NAME,
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

/// A queued command for the renderer
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum RenderCommand {
    SetPipeline {
        pipeline: Handle<PipelineDescriptor>,
    },
    SetVertexBuffer {
        slot: u32,
        buffer: BufferId,
        offset: u64,
    },
    SetIndexBuffer {
        buffer: BufferId,
        offset: u64,
    },
    SetBindGroup {
        index: u32,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<Arc<[u32]>>,
    },
    DrawIndexed {
        indices: Range<u32>,
        base_vertex: i32,
        instances: Range<u32>,
    },
    Draw {
        vertices: Range<u32>,
        instances: Range<u32>,
    },
}

/// A component that indicates how to draw an entity.
#[derive(Debug, Properties, Clone)]
pub struct Draw {
    pub is_visible: bool,
    pub is_transparent: bool,
    #[property(ignore)]
    pub render_commands: Vec<RenderCommand>,
}

impl Default for Draw {
    fn default() -> Self {
        Self {
            is_visible: true,
            is_transparent: false,
            render_commands: Default::default(),
        }
    }
}

impl Draw {
    pub fn clear_render_commands(&mut self) {
        self.render_commands.clear();
    }

    pub fn set_pipeline(&mut self, pipeline: &Handle<PipelineDescriptor>) {
        self.render_command(RenderCommand::SetPipeline {
            pipeline: pipeline.clone_weak(),
        });
    }

    pub fn set_vertex_buffer(&mut self, slot: u32, buffer: BufferId, offset: u64) {
        self.render_command(RenderCommand::SetVertexBuffer {
            slot,
            buffer,
            offset,
        });
    }

    pub fn set_index_buffer(&mut self, buffer: BufferId, offset: u64) {
        self.render_command(RenderCommand::SetIndexBuffer { buffer, offset });
    }

    pub fn set_bind_group(&mut self, index: u32, bind_group: &BindGroup) {
        self.render_command(RenderCommand::SetBindGroup {
            index,
            bind_group: bind_group.id,
            dynamic_uniform_indices: bind_group.dynamic_uniform_indices.clone(),
        });
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_command(RenderCommand::DrawIndexed {
            base_vertex,
            indices,
            instances,
        });
    }

    #[inline]
    pub fn render_command(&mut self, render_command: RenderCommand) {
        self.render_commands.push(render_command);
    }
}

#[derive(Debug, Error)]
pub enum DrawError {
    #[error("Pipeline does not exist.")]
    NonExistentPipeline,
    #[error("No pipeline set")]
    NoPipelineSet,
    #[error("Pipeline has no layout")]
    PipelineHasNoLayout,
    #[error("Failed to get a buffer for the given RenderResource.")]
    BufferAllocationFailure,
}

//#[derive(Debug)]
pub struct DrawContext<'a> {
    pub pipelines: ResMut<'a, Assets<PipelineDescriptor>>,
    pub shaders: ResMut<'a, Assets<Shader>>,
    pub pipeline_compiler: ResMut<'a, PipelineCompiler>,
    pub render_resource_context: Res<'a, Box<dyn RenderResourceContext>>,
    pub shared_buffers: Res<'a, SharedBuffers>,
    pub current_pipeline: Option<Handle<PipelineDescriptor>>,
}

impl<'a> UnsafeClone for DrawContext<'a> {
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

impl<'a> ResourceQuery for DrawContext<'a> {
    type Fetch = FetchDrawContext;
}

#[derive(Debug)]
pub struct FetchDrawContext;

// TODO: derive this impl
impl<'a> FetchResource<'a> for FetchDrawContext {
    type Item = DrawContext<'a>;

    fn borrow(resources: &Resources) {
        resources.borrow_mut::<Assets<PipelineDescriptor>>();
        resources.borrow_mut::<Assets<Shader>>();
        resources.borrow_mut::<PipelineCompiler>();
        resources.borrow::<Box<dyn RenderResourceContext>>();
        resources.borrow::<SharedBuffers>();
    }

    fn release(resources: &Resources) {
        resources.release_mut::<Assets<PipelineDescriptor>>();
        resources.release_mut::<Assets<Shader>>();
        resources.release_mut::<PipelineCompiler>();
        resources.release::<Box<dyn RenderResourceContext>>();
        resources.release::<SharedBuffers>();
    }

    unsafe fn get(resources: &'a Resources, _system_id: Option<SystemId>) -> Self::Item {
        let pipelines = {
            let (value, type_state) = resources
                .get_unsafe_ref_with_type_state::<Assets<PipelineDescriptor>>(
                    ResourceIndex::Global,
                );
            ResMut::new(value, type_state.mutated())
        };
        let shaders = {
            let (value, type_state) =
                resources.get_unsafe_ref_with_type_state::<Assets<Shader>>(ResourceIndex::Global);
            ResMut::new(value, type_state.mutated())
        };
        let pipeline_compiler = {
            let (value, type_state) =
                resources.get_unsafe_ref_with_type_state::<PipelineCompiler>(ResourceIndex::Global);
            ResMut::new(value, type_state.mutated())
        };

        DrawContext {
            pipelines,
            shaders,
            pipeline_compiler,
            render_resource_context: Res::new(
                resources.get_unsafe_ref::<Box<dyn RenderResourceContext>>(ResourceIndex::Global),
            ),
            shared_buffers: Res::new(
                resources.get_unsafe_ref::<SharedBuffers>(ResourceIndex::Global),
            ),
            current_pipeline: None,
        }
    }

    fn access() -> TypeAccess<TypeId> {
        let mut access = TypeAccess::default();
        access.add_write(TypeId::of::<Assets<PipelineDescriptor>>().into());
        access.add_write(TypeId::of::<Assets<Shader>>().into());
        access.add_write(TypeId::of::<PipelineCompiler>().into());
        access.add_read(TypeId::of::<Box<dyn RenderResourceContext>>().into());
        access.add_read(TypeId::of::<SharedBuffers>().into());
        access
    }
}

impl<'a> DrawContext<'a> {
    pub fn get_uniform_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
    ) -> Result<RenderResourceBinding, DrawError> {
        self.get_buffer(render_resource, BufferUsage::UNIFORM)
    }

    pub fn get_buffer<T: RenderResource>(
        &self,
        render_resource: &T,
        buffer_usage: BufferUsage,
    ) -> Result<RenderResourceBinding, DrawError> {
        self.shared_buffers
            .get_buffer(render_resource, buffer_usage)
            .ok_or(DrawError::BufferAllocationFailure)
    }

    pub fn set_pipeline(
        &mut self,
        draw: &mut Draw,
        pipeline_handle: &Handle<PipelineDescriptor>,
        specialization: &PipelineSpecialization,
    ) -> Result<(), DrawError> {
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

        draw.set_pipeline(&specialized_pipeline);
        self.current_pipeline = Some(specialized_pipeline.clone_weak());
        Ok(())
    }

    pub fn get_pipeline_descriptor(&self) -> Result<&PipelineDescriptor, DrawError> {
        self.current_pipeline
            .as_ref()
            .and_then(|handle| self.pipelines.get(handle))
            .ok_or(DrawError::NoPipelineSet)
    }

    pub fn get_pipeline_layout(&self) -> Result<&PipelineLayout, DrawError> {
        self.get_pipeline_descriptor().and_then(|descriptor| {
            descriptor
                .get_layout()
                .ok_or(DrawError::PipelineHasNoLayout)
        })
    }

    pub fn set_bind_groups_from_bindings(
        &self,
        draw: &mut Draw,
        render_resource_bindings: &mut [&mut RenderResourceBindings],
    ) -> Result<(), DrawError> {
        let pipeline = self
            .current_pipeline
            .as_ref()
            .ok_or(DrawError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(pipeline)
            .ok_or(DrawError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or(DrawError::PipelineHasNoLayout)?;
        for bindings in render_resource_bindings.iter_mut() {
            bindings.update_bind_groups(pipeline_descriptor, &**self.render_resource_context);
        }
        for bind_group_descriptor in layout.bind_groups.iter() {
            for bindings in render_resource_bindings.iter_mut() {
                if let Some(bind_group) =
                    bindings.get_descriptor_bind_group(bind_group_descriptor.id)
                {
                    draw.set_bind_group(bind_group_descriptor.index, bind_group);
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
    ) -> Result<(), DrawError> {
        let pipeline = self
            .current_pipeline
            .as_ref()
            .ok_or(DrawError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(pipeline)
            .ok_or(DrawError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or(DrawError::PipelineHasNoLayout)?;
        let bind_group_descriptor = &layout.bind_groups[index as usize];
        self.render_resource_context
            .create_bind_group(bind_group_descriptor.id, bind_group);
        Ok(())
    }

    pub fn set_vertex_buffers_from_bindings(
        &self,
        draw: &mut Draw,
        render_resource_bindings: &[&RenderResourceBindings],
    ) -> Result<(), DrawError> {
        let pipeline = self
            .current_pipeline
            .as_ref()
            .ok_or(DrawError::NoPipelineSet)?;
        let pipeline_descriptor = self
            .pipelines
            .get(pipeline)
            .ok_or(DrawError::NonExistentPipeline)?;
        let layout = pipeline_descriptor
            .get_layout()
            .ok_or(DrawError::PipelineHasNoLayout)?;
        // figure out if the fallback buffer is needed
        let need_fallback_buffer = layout
            .vertex_buffer_descriptors
            .iter()
            .any(|x| x.name == VERTEX_FALLBACK_LAYOUT_NAME);
        for bindings in render_resource_bindings.iter() {
            if let Some(index_buffer) = bindings.index_buffer {
                draw.set_index_buffer(index_buffer, 0);
            }
            if let Some(main_vertex_buffer) = bindings.vertex_attribute_buffer {
                draw.set_vertex_buffer(0, main_vertex_buffer, 0);
            }
            if need_fallback_buffer {
                if let Some(fallback_vertex_buffer) = bindings.vertex_fallback_buffer {
                    draw.set_vertex_buffer(1, fallback_vertex_buffer, 0);
                }
            }
        }
        Ok(())
    }
}

pub trait Drawable {
    fn draw(&mut self, draw: &mut Draw, context: &mut DrawContext) -> Result<(), DrawError>;
}

pub fn clear_draw_system(mut query: Query<&mut Draw>) {
    for mut draw in query.iter_mut() {
        draw.clear_render_commands();
    }
}
