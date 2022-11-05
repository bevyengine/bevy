use super::TaskPool;

/// Used by `bevy_core` to tick the global tasks pools on the main thread.
/// This will run a maximum of 100 local tasks per executor per call to this function.
#[cfg(not(target_arch = "wasm32"))]
pub fn tick_global_task_pools_on_main_thread() {
    TaskPool::get().with_local_executor(|local_executor| {
        for _ in 0..100 {
            local_executor.try_tick();
        }
    });
}
