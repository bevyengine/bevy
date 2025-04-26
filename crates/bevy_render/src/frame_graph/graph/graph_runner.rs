use bevy_ecs::world::World;

use crate::{
    diagnostic::internal::{DiagnosticsRecorder, RenderDiagnosticsMutex},
    frame_graph::{FrameGraphError, RenderContext, TransientResourceCache},
    render_resource::PipelineCache,
    renderer::RenderDevice,
};

use super::FrameGraph;

pub struct FrameGraphRunner;

impl FrameGraphRunner {
    pub fn run(
        graph: &mut FrameGraph,
        render_device: RenderDevice,
        transient_resource_cache: &mut TransientResourceCache,
        pipeline_cache: &PipelineCache,
        mut diagnostics_recorder: Option<DiagnosticsRecorder>,
        queue: &wgpu::Queue,
        #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
        adapter: &wgpu::Adapter,
        world: &World,
        finalizer: impl FnOnce(&mut wgpu::CommandEncoder),
    ) -> Result<Option<DiagnosticsRecorder>, FrameGraphError> {
        if let Some(recorder) = &mut diagnostics_recorder {
            recorder.begin_frame();
        }

        let mut render_context = RenderContext::new(
            render_device,
            transient_resource_cache,
            pipeline_cache,
            #[cfg(not(all(target_arch = "wasm32", target_feature = "atomics")))]
            adapter.get_info(),
            diagnostics_recorder,
        );

        graph.execute(&mut render_context)?;

        finalizer(render_context.command_encoder());

        let (render_device, mut diagnostics_recorder) = {
            let (commands, render_device, diagnostics_recorder) = render_context.finish();

            #[cfg(feature = "trace")]
            let _span = info_span!("submit_graph_commands").entered();
            queue.submit(commands);

            (render_device, diagnostics_recorder)
        };

        if let Some(recorder) = &mut diagnostics_recorder {
            let render_diagnostics_mutex = world.resource::<RenderDiagnosticsMutex>().0.clone();
            recorder.finish_frame(&render_device, move |diagnostics| {
                *render_diagnostics_mutex.lock().expect("lock poisoned") = Some(diagnostics);
            });
        }

        Ok(diagnostics_recorder)
    }
}
