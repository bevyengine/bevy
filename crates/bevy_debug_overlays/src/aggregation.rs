use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_render::renderer::GpuTimerScopes;
use bevy_utils::HashMap;

#[derive(Resource, Default)]
pub struct AggregatedGpuTimers(pub HashMap<String, f64>);

// TODO: Handle nesting
pub fn aggregate_gpu_timers(
    gpu_timers: Res<GpuTimerScopes>,
    mut aggregated_gpu_timers: ResMut<AggregatedGpuTimers>,
) {
    let mut stack = gpu_timers.take();
    while let Some(gpu_timer) = stack.pop() {
        let average = aggregated_gpu_timers.0.entry(gpu_timer.label).or_default();
        let duration = gpu_timer.time.end - gpu_timer.time.start;
        *average = (*average * 0.1) + (duration * 0.9);

        for gpu_timer in gpu_timer.nested_scopes {
            stack.push(gpu_timer);
        }
    }
}
