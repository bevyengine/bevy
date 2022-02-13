use std::sync::{Arc, Mutex, MutexGuard};

use bevy_diagnostic::{DiagnosticId, Diagnostics, Diagnostic};
use bevy_ecs::system::{ResMut, Res};
use bevy_utils::HashMap;
use wgpu::{CommandEncoder, Queue};

use crate::{renderer::RenderDevice};

pub struct GpuProfilerInner {
    profiler: wgpu_profiler::GpuProfiler,
    diagnostic_ids: HashMap<String, DiagnosticId>
}

pub struct GpuProfileScope<'a>(MutexGuard<'a, GpuProfilerInner>);

impl <'a> GpuProfileScope<'a> {
    pub fn end_scope(mut self, encoder: &mut CommandEncoder) {
        self.0.profiler.end_scope(encoder);
    }
}

#[derive(Clone)]
pub struct GpuProfiler(Arc<Mutex<GpuProfilerInner>>);

impl GpuProfiler {
    pub fn new(queue: &Queue) -> Self {
        let profiler = GpuProfiler(Arc::new(Mutex::new(GpuProfilerInner {
            profiler: wgpu_profiler::GpuProfiler::new(4, queue.get_timestamp_period()),
            diagnostic_ids: HashMap::default()
        })));
        profiler
    }
}

impl GpuProfiler {
    pub fn begin_scope<'a>(&'a self, label: &str, encoder: &mut CommandEncoder, device: &RenderDevice) -> GpuProfileScope<'a> {
        let mut scope = self.0.lock().unwrap();
        scope.profiler.begin_scope(label, encoder, device.wgpu_device());
        GpuProfileScope::<'a>(scope)
    }

    pub fn resolve_queries(&self, encoder: &mut CommandEncoder) {
        let profiler = &mut self.0.lock().unwrap().profiler;
        profiler.resolve_queries(encoder);
    }

    pub fn end_frame(&self, diagnostics: &mut Diagnostics) {
        let profiler = &mut self.0.lock().unwrap();
        profiler.profiler.end_frame().unwrap_or_default();
        if let Some(results) = profiler.profiler.process_finished_frame() {
            for result in results {
                let id = match profiler.diagnostic_ids.get(&result.label) {
                    Some(id) => *id,
                    None => {
                        let diagnostic_id = DiagnosticId::default();
                        profiler.diagnostic_ids.insert(result.label.clone(), diagnostic_id);
                        let diagnostic = Diagnostic::new(diagnostic_id, result.label, 600).with_suffix("ms");
                        diagnostics.add(diagnostic);
                        diagnostic_id
                    },
                };
                diagnostics.add_measurement(id, (result.time.end - result.time.start) * 1000.0);
            }
        }
    }
}

pub fn gpu_profiler_system(profiler: Res<GpuProfiler>, mut diagnostics: ResMut<Diagnostics>) {
    profiler.end_frame(diagnostics.as_mut());
}