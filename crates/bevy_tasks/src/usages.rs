use super::TaskPool;

crate::cfg::web! {
    if {} else {
        /// A function used by `bevy_app` to tick the global tasks pools on the main thread.
        /// This will run a maximum of 100 local tasks per executor per call to this function.
        ///
        /// # Warning
        ///
        /// This function *must* be called on the main thread, or the task pools will not be updated appropriately.
        pub fn tick_global_task_pools_on_main_thread() {
            for _ in 0..100 {
                if !TaskPool::try_tick_local() {
                    break;
                }
            }
        }
    }
}
