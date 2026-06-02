//! The objective here is to coordinate two participants that want to share World access:
//!
//! - The main Bevy schedule
//! - Futures and async tasks running on other threads
//!
//! This is done through the bridge primitive introduced in this crate
//!
//!
//! Invariants of this crate:
//!
//! - Normal rust safety invariants for &mut World (aliasing)
//! - At most one future has world access at a time
//! - Futures only access the world while the scoped pointer (managed by the bridge driver) is live
//! - `SystemState` is always initialized before use
//! - Deferred ops are only applied after every future finishes polling and releases world access
//! - The driver can't deadlock
//! - All futures that want world access can eventually complete (assuming fair scheduling by the async runtime)
//! - If the world is dropped, futures don't leak and eventually finish (in an error state)
//!
//!
//! The protocol:
//!
//! Futures (tasks on worker threads)
//! - enqueue requests (create signal guard clones: one kept, one sent)
//!
//! - Driver([`async_world_sync_point`]) (exclusive system, world-owning thread)
//!   1. Drain request queue for this sync point
//!   2. Publish World pointer (via `scoped_static_storage`). Future access scope begins
//!   3. Wake all drained futures
//!
//!  -> Futures race for locks (non-blocking)
//!
//!  -> Success: acquire both locks, do work, complete
//!
//!  -> Failure: signal driver (Drop signal guard), re-enqueue later
//!
//!  -> Direct access: non-queued future polled during scope,
//!  bypasses queue, acquires locks, completes (no signal)
//!   4. Wait for all signal guards to drop + scope mutex released
//!   5. Unpublish pointer, scope ends.
//!   6. Apply any deferred ops from `SystemState` of polled futures
//!   7. Loop (up to [`AsyncTickBudget`]) or return
//!   8. Schedule resumes (normal systems run)
//!
//!
//! Dual locking:
//!
//! The published World pointer lock is managed by the `ScopedStatic` primitive in `scoped_static_storage` (only one future can lock this at a time)
//! `SystemState` locks are managed by the `SystemStateCell` primitive of this crate (futures using different `SystemState` types can work in parallel)
//!
//!
//! Preventing driver deadlocks when futures panic:
//!
//! If a future panics while holding locks, rust's panic unwinding drops destructors in reverse scope order
//! - First, the `SystemState` `MutexGuard` drops (releasing the lock)
//! - Second, the World pointer's scope `MutexGuard` drops (releasing the lock)
//! - Finally, the guard signal constructed by the future during `poll()` drops, and the driver is notified
//!
//! How futures can fail cleanly:
//!
//! If the [`AsyncWorld`] cannot be reached ([`bevy_platform::sync::Weak::upgrade`] fails during `poll()`), the world has been dropped and the future cannot complete.
//!
//! If `SystemState`s are invalid, they can't be used and the future cannot complete
//!
//! Regardless, the future returns Ready(Err) and completes permanently
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://!bevy.org/assets/icon.png",
    html_favicon_url = "https://!bevy.org/assets/icon.png"
)]
#![no_std]

#[cfg(feature = "std")]
extern crate std;

// Forbid unsafe_code in every module except the tests, which need some unsafe for Future Pins.
#[forbid(unsafe_code)]
mod bridge_future;
#[forbid(unsafe_code)]
mod bridge_request;
#[forbid(unsafe_code)]
mod plugin;
#[forbid(unsafe_code)]
mod system_state;
#[forbid(unsafe_code)]
mod wake_signal;

pub use crate::bridge_future::{AsyncSystemState, BridgeError};
pub use crate::bridge_request::async_world_sync_point;
pub use crate::plugin::{AsyncPlugin, AsyncTickBudget, AsyncWorld};

/// The async prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        async_world_sync_point, AsyncPlugin, AsyncSystemState, AsyncTickBudget, AsyncWorld,
        BridgeError,
    };
}

#[cfg(test)]
mod tests {
    extern crate alloc;

    use core::pin::Pin;

    use alloc::{sync::Arc, vec::Vec};

    use crate::prelude::*;
    use bevy_app::prelude::*;
    use bevy_app::ScheduleRunnerPlugin;
    use bevy_ecs::prelude::*;
    use bevy_platform::sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    };
    use bevy_tasks::futures::check_ready;
    use bevy_tasks::AsyncComputeTaskPool;

    /// A future wrapper around `F` that **first** polls the `F` future, then increments `counter`.
    ///
    /// This allows tests to wait until async bridge futures are polled once (and therefore are
    /// waiting for ECS access) before allowing ECS access, which we can then track since the ECS
    /// cannot proceed until all async tasks have moved on.
    struct PollThenCount<F> {
        future: F,
        counted: bool,
        counter: Arc<Mutex<usize>>,
    }

    impl<F: Future> Future for PollThenCount<F> {
        type Output = F::Output;

        fn poll(
            self: Pin<&mut Self>,
            cx: &mut core::task::Context<'_>,
        ) -> core::task::Poll<Self::Output> {
            #[expect(
                unsafe_code,
                reason = "we need to access all fields independently to update the future's state"
            )]
            // SAFETY: We don't move out of `this` - we just create a pin to the future (which
            // we poll), then assign to `counted` and update `counter`.
            let this = unsafe { self.get_unchecked_mut() };
            #[expect(unsafe_code, reason = "we need to poll the future for !Unpin types")]
            // SAFETY: We never move this.future, so it is pinned in place, so this pin is
            // valid.
            let result = unsafe { Pin::new_unchecked(&mut this.future) }.poll(cx);
            if !this.counted {
                this.counted = true;
                *this.counter.lock().unwrap() += 1;
            }
            result
        }
    }

    #[test]
    fn more_tasks_than_threads() {
        struct MySyncPoint;

        let mut app = App::new();
        app.add_plugins((
            AsyncPlugin::default(),
            ScheduleRunnerPlugin::default(),
            TaskPoolPlugin::default(),
        ))
        .insert_resource(AsyncTickBudget(3))
        .add_systems(Update, async_world_sync_point::<MySyncPoint>);

        let system_state = app
            .world()
            .resource::<AsyncWorld>()
            .system_state::<Commands>();

        let task_pool = AsyncComputeTaskPool::get();
        let desired_tasks = task_pool.thread_num() * 10;

        let barrier_counter = Arc::new(Mutex::new(0));
        let mut tasks = Vec::new();
        for _ in 0..desired_tasks {
            let barrier_counter = barrier_counter.clone();
            let system_state = system_state.clone();
            tasks.push(task_pool.spawn(async move {
                let future = system_state.bridge(MySyncPoint, |_: Commands| {});
                PollThenCount {
                    future,
                    counted: false,
                    counter: barrier_counter,
                }
                .await
                .unwrap()
            }));
        }

        // Spinloop until all the tasks are waiting for ECS access.
        while *barrier_counter.lock().unwrap() != desired_tasks {
            // If we're configured to be single-threaded, tick the task pools.
            bevy_tasks::cfg::multi_threaded! {
                if {} else {
                    bevy_tasks::tick_global_task_pools_on_main_thread();
                }
            }
        }

        // Clear the barrier counters.
        *barrier_counter.lock().unwrap() = 0;

        app.update();

        'outer: {
            for _ in 0..10000 {
                bevy_tasks::cfg::multi_threaded! {
                    if {} else {
                        bevy_tasks::tick_global_task_pools_on_main_thread();
                    }
                }
                tasks.retain_mut(|task| check_ready(task).is_none());
                if tasks.is_empty() {
                    break 'outer;
                }
            }

            panic!("Ran out of iterations waiting for tasks to complete");
        }
    }

    #[test]
    fn different_sync_points_allow_different_tasks() {
        struct Sync1;
        struct Sync2;

        let mut app = App::new();
        app.add_plugins((
            AsyncPlugin::default(),
            ScheduleRunnerPlugin::default(),
            TaskPoolPlugin::default(),
        ));

        let system_state = app
            .world()
            .resource::<AsyncWorld>()
            .system_state::<Commands>();

        let system_state_clone = system_state.clone();
        let mut task_1 = AsyncComputeTaskPool::get().spawn(async move {
            system_state_clone
                .bridge(Sync1, |_: Commands| {})
                .await
                .unwrap();
            1
        });
        let mut task_2 = AsyncComputeTaskPool::get().spawn(async move {
            system_state.bridge(Sync2, |_: Commands| {}).await.unwrap();
            2
        });

        assert!(check_ready(&mut task_1).is_none());
        assert!(check_ready(&mut task_2).is_none());

        app.world_mut()
            .run_system_cached(async_world_sync_point::<Sync1>)
            .unwrap();

        assert_eq!(check_ready(&mut task_1).unwrap(), 1);
        assert!(check_ready(&mut task_2).is_none());

        app.world_mut()
            .run_system_cached(async_world_sync_point::<Sync2>)
            .unwrap();

        assert_eq!(check_ready(&mut task_2).unwrap(), 2);
    }

    /// This tests that if a world is dropped we return an error from attempting to run it and
    /// that everything cleans up nicely
    /// Because of a quirk of how bevy's task pools work we have to always have at least one
    /// active world for anything to progress on them.
    /// That's what `other_app` is for.
    #[test]
    fn dropped_world() {
        struct MySyncPoint;
        static WORLD_WAS_DROPPED: AtomicBool = AtomicBool::new(false);
        let mut other_app = App::new();
        other_app.add_plugins((TaskPoolPlugin::default(), ScheduleRunnerPlugin::default()));
        let mut app = App::new();
        app.add_plugins((
            AsyncPlugin::default(),
            ScheduleRunnerPlugin::default(),
            TaskPoolPlugin::default(),
        ));

        app.add_systems(Startup, move |world: Res<AsyncWorld>| {
            let world = world.clone();
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let system_state = world.system_state::<Commands>();
                    match system_state
                        .bridge(MySyncPoint, |mut commands: Commands| {
                            commands.spawn_empty();
                        })
                        .await
                    {
                        Err(BridgeError::WorldDropped) => {
                            WORLD_WAS_DROPPED.store(true, Ordering::Relaxed);
                        }
                        _ => unreachable!("World should have Dropped"),
                    }
                })
                .detach();
        });
        app.update();
        drop(app);
        other_app.update();
        assert!(WORLD_WAS_DROPPED.load(Ordering::Relaxed));
    }

    bevy_tasks::cfg::multi_threaded! {
        #[test]
        fn ecs_then_stuck() {
            use bevy_platform::sync::{Arc, Mutex};

            // We want to make sure that the implementation here doesn't block the ECS thread. So we
            // spawn a task that does some ECS work and then immediately blocks (not awaits). Since
            // this test blocks a thread, we cannot run this test unless we are multi_threaded.

            struct MySyncPoint;

            let mut app = App::new();
            app.add_plugins((
                AsyncPlugin,
                ScheduleRunnerPlugin::default(),
                TaskPoolPlugin::default(),
            ));

            let mutex = Arc::new(Mutex::new(()));

            let mutex_clone = mutex.clone();
            app.add_systems(Startup, move |world: Res<AsyncWorld>| {
                let system_state = world.system_state::<Commands>();

                let mutex_clone = mutex_clone.clone();
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        system_state
                            .bridge(MySyncPoint, |mut commands| {
                                commands.spawn_empty();
                            })
                            .await
                            .unwrap();
                        let _guard = mutex_clone.lock().unwrap();
                    })
                    .detach();
            })
            .add_systems(Update, async_world_sync_point::<MySyncPoint>);

            // Lock the guard while we update - this makes it impossible to get the lock, and
            // therefore, makes the task stuck. That's fine as long as this happens in another
            // thread.
            let _guard = mutex.lock().unwrap();

            app.update();
        }
    }

    #[test]
    fn invalid_parameters() {
        struct MySyncPoint;
        static FAILED_VALIDATION: AtomicBool = AtomicBool::new(false);

        #[derive(Resource)]
        struct MyResource;

        let mut app = App::new();
        app.add_plugins((
            AsyncPlugin,
            ScheduleRunnerPlugin::default(),
            TaskPoolPlugin::default(),
        ));

        app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

        app.add_systems(Startup, move |world: Res<AsyncWorld>| {
            let world = world.clone();
            AsyncComputeTaskPool::get()
                .spawn(async move {
                    let system_state = world.system_state::<Res<MyResource>>();
                    match system_state.bridge(MySyncPoint, |_| unreachable!()).await {
                        Err(BridgeError::SystemParamValidation(_)) => {
                            FAILED_VALIDATION.store(true, Ordering::Relaxed);
                        }
                        _ => unreachable!("Parameter validation should have failed"),
                    }
                })
                .detach();
        });

        app.update();

        assert!(FAILED_VALIDATION.load(Ordering::Relaxed));
    }

    #[test]
    #[cfg(not(feature = "std"))]
    fn no_std_test() {
        use crate::prelude::*;
        use bevy_app::prelude::*;
        use bevy_app::ScheduleRunnerPlugin;
        use bevy_ecs::prelude::*;
        use bevy_platform::sync::atomic::AtomicBool;
        use bevy_platform::sync::atomic::Ordering;
        use bevy_tasks::AsyncComputeTaskPool;

        struct MySyncPoint;
        static ACCESS_RAN: AtomicBool = AtomicBool::new(false);
        let mut app = App::new();
        app.add_plugins((
            AsyncPlugin,
            ScheduleRunnerPlugin::default(),
            TaskPoolPlugin::default(),
        ));

        app.add_systems(Update, async_world_sync_point::<MySyncPoint>);

        app.add_systems(Startup, move |world: Res<AsyncWorld>| {
            let world = world.clone();
            AsyncComputeTaskPool::get()
                .spawn_local(async move {
                    let system_state = world.system_state::<Commands>();
                    system_state
                        .bridge(MySyncPoint, |mut commands: Commands| {
                            commands.spawn_empty();
                            ACCESS_RAN.store(true, Ordering::Relaxed);
                        })
                        .await
                        .unwrap();
                })
                .detach();
        });

        app.update();

        assert!(ACCESS_RAN.load(Ordering::Relaxed));
    }
}
