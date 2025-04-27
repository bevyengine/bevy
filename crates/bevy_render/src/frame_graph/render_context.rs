use wgpu::AdapterInfo;

use super::{
    BindGroupRef, FrameGraphBuffer, FrameGraphError, GraphResource, RenderPassInfo, ResourceRead,
    ResourceRef, ResourceTable, TransientResourceCache,
};
use crate::{
    diagnostic::internal::DiagnosticsRecorder,
    render_resource::{CachedRenderPipelineId, PipelineCache, RenderPipeline},
    renderer::RenderDevice,
};
use alloc::sync::Arc;
use core::ops::Range;

pub trait ExtraResource {
    type Resource;
    fn extra_resource(
        &self,
        resource_context: &RenderContext,
    ) -> Result<Self::Resource, FrameGraphError>;
}

pub struct RenderContext<'a> {
    pub(crate) render_device: RenderDevice,
    pub(crate) resource_table: ResourceTable,
    pub(crate) transient_resource_cache: &'a mut TransientResourceCache,
    command_buffer_queue: Vec<wgpu::CommandBuffer>,
    pipeline_cache: &'a PipelineCache,
    // #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
    // force_serial: bool,
    diagnostics_recorder: Option<Arc<DiagnosticsRecorder>>,
    command_encoder: Option<wgpu::CommandEncoder>,
}

impl<'a> RenderContext<'a> {
    pub fn new(
        render_device: RenderDevice,
        transient_resource_cache: &'a mut TransientResourceCache,
        pipeline_cache: &'a PipelineCache,
        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        adapter_info: AdapterInfo,
        diagnostics_recorder: Option<DiagnosticsRecorder>,
    ) -> Self {
        // HACK: Parallel command encoding is currently bugged on AMD + Windows/Linux + Vulkan
        #[cfg(any(target_os = "windows", target_os = "linux"))]
        let _force_serial =
            adapter_info.driver.contains("AMD") && adapter_info.backend == wgpu::Backend::Vulkan;
        #[cfg(not(any(
            target_os = "windows",
            target_os = "linux",
            all(target_arch = "wasm32", target_feature = "atomics")
        )))]
        let force_serial = {
            drop(adapter_info);
            false
        };

        Self {
            render_device,
            resource_table: Default::default(),
            transient_resource_cache,
            command_buffer_queue: vec![],
            pipeline_cache,
            command_encoder: None,
            // #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
            // force_serial,
            diagnostics_recorder: diagnostics_recorder.map(Arc::new),
        }
    }

    fn flush_encoder(&mut self) {
        if let Some(encoder) = self.command_encoder.take() {
            self.command_buffer_queue.push(encoder.finish());
        }
    }

    pub fn begin_render_pass<'b>(
        &'b mut self,
        render_pass_info: &RenderPassInfo,
    ) -> Result<TrackedRenderPass<'a, 'b>, FrameGraphError> {
        self.flush_encoder();

        let mut command_encoder = self
            .render_device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let render_pass = render_pass_info.create_render_pass(self, &mut command_encoder)?;

        Ok(TrackedRenderPass {
            command_encoder,
            render_pass,
            render_context: self,
        })
    }

    pub fn get_render_pipeline(
        &self,
        id: CachedRenderPipelineId,
    ) -> Result<&RenderPipeline, FrameGraphError> {
        self.pipeline_cache
            .get_render_pipeline(id)
            .ok_or(FrameGraphError::ResourceNotFound)
    }

    pub fn get_resource<ResourceType: GraphResource>(
        &self,
        resource_ref: &ResourceRef<ResourceType, ResourceRead>,
    ) -> Result<&ResourceType, FrameGraphError> {
        self.resource_table
            .get_resource(resource_ref)
            .ok_or(FrameGraphError::ResourceNotFound)
    }

    pub fn add_command_buffer(&mut self, command_buffer: wgpu::CommandBuffer) {
        self.command_buffer_queue.push(command_buffer);
    }

    pub fn command_encoder(&mut self) -> &mut wgpu::CommandEncoder {
        self.command_encoder.get_or_insert_with(|| {
            self.render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default())
        })
    }

    pub fn finish(
        mut self,
    ) -> (
        Vec<wgpu::CommandBuffer>,
        RenderDevice,
        Option<DiagnosticsRecorder>,
    ) {
        self.flush_encoder();

        let mut command_buffers = self.command_buffer_queue;

        let mut diagnostics_recorder = self.diagnostics_recorder.take().map(|v| {
            Arc::try_unwrap(v)
                .ok()
                .expect("diagnostic recorder shouldn't be held longer than necessary")
        });

        if let Some(recorder) = &mut diagnostics_recorder {
            let mut command_encoder = self
                .render_device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
            recorder.resolve(&mut command_encoder);
            command_buffers.push(command_encoder.finish());
        }

        (command_buffers, self.render_device, diagnostics_recorder)
    }
}

pub struct TrackedRenderPass<'a, 'b> {
    command_encoder: wgpu::CommandEncoder,
    render_pass: wgpu::RenderPass<'static>,
    render_context: &'b mut RenderContext<'a>,
}

impl<'a, 'b> TrackedRenderPass<'a, 'b> {
    pub fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_ref: &BindGroupRef,
        offsets: &[u32],
    ) -> Result<(), FrameGraphError> {
        let bind_group = bind_group_ref.extra_resource(&self.render_context)?;
        self.render_pass.set_bind_group(index, &bind_group, offsets);

        Ok(())
    }

    pub fn set_render_pipeline(
        &mut self,
        id: CachedRenderPipelineId,
    ) -> Result<(), FrameGraphError> {
        let pipeline = self.render_context.get_render_pipeline(id)?;
        self.render_pass.set_pipeline(pipeline);

        Ok(())
    }

    pub fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>) {
        self.render_pass
            .draw_indexed(indices, base_vertex, instances);
    }

    pub fn draw(&mut self, vertices: Range<u32>, instances: Range<u32>) {
        self.render_pass.draw(vertices, instances);
    }

    pub fn set_vertex_buffer(
        &mut self,
        slot: u32,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(buffer_ref)?;
        self.render_pass
            .set_vertex_buffer(slot, buffer.resource.slice(0..));

        Ok(())
    }

    pub fn set_index_buffer(
        &mut self,
        buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        index_format: wgpu::IndexFormat,
    ) -> Result<(), FrameGraphError> {
        let buffer = self.render_context.get_resource(buffer_ref)?;

        self.render_pass
            .set_index_buffer(buffer.resource.slice(0..), index_format);

        Ok(())
    }

    pub fn end(self) {
        drop(self.render_pass);
        let command_buffer = self.command_encoder.finish();
        self.render_context.add_command_buffer(command_buffer);
    }
}
