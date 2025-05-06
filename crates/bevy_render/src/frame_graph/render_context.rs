pub mod parameter;

pub mod render_pass_context;

pub use parameter::*;
pub use render_pass_context::*;

use wgpu::AdapterInfo;

use super::{
    FrameGraphBuffer, FrameGraphError, GraphResource, RenderPassInfo, ResourceRead, ResourceRef,
    ResourceTable, TransientResourceCache,
};
use crate::{
    diagnostic::internal::DiagnosticsRecorder,
    render_resource::{CachedRenderPipelineId, PipelineCache, RenderPipeline},
    renderer::RenderDevice,
};
use alloc::sync::Arc;

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
    ) -> Result<RenderPassContext<'a, 'b>, FrameGraphError> {
        self.flush_encoder();

        let mut command_encoder = self
            .render_device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        let render_pass = render_pass_info.create_render_pass(&mut command_encoder)?;

        Ok(RenderPassContext::new(command_encoder, render_pass, self))
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
