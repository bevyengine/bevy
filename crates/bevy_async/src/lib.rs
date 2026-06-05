//! `bevy_async` allows async tasks to synchronize with the main Bevy schedule, allowing futures to
//! access the ECS. This crate provides a "bridge" that performs this synchronization.
//!
//! # How does bridging occur?
//!
//! Users need to:
//!
//! - Create a "sync point" type. This is a marker type to indicate which sync point will be used
//!   when accessing the ECS. This should generally just be as simple as `struct MySyncPoint;`.
//! - Add an [`async_world_sync_point`] system somewhere in your schedule. For example, adding
//!   `app.add_systems(Update, async_world_sync_point::<MySyncPoint>);` will allow your async tasks
//!   to access the ECS during the [`Update`] schedule. This system can also have ordering
//!   constraints to ensure its place in the schedule.
//! - Users call [`AsyncWorld::system_state`] to create the state they need to access the ECS. This
//!   state should be reused whenever possible - features like [`Local`] or [`Changed`] rely on the
//!   state being preserved between usages, and queries remain cached which can be more performant.
//!   [`AsyncWorld::system_state`] can be called inside or **outside** the async task.
//! - Inside the async task, call [`AsyncSystemState::bridge`] with the sync point type you'd like
//!   to use and the closure to run ECS access with, and then await this future.
//!
//! # Alternatives
//!
//! It is possible to access the ECS **without** this crate (in limited ways). For example, you can
//! use a channel as demonstrated in the [`async_channel_pattern`] example, or you can simply
//! [`check_ready`] on the [`Task`] as demonstrated in the [`async_compute`] example.
//!
//! ## Advantages to using this crate
//!
//! This crate:
//!
//! - Provides an out-of-the-box solution for all ECS accesses. The alternatives above require
//!   manual setup (e.g., you need to create your own channel, your own systems), and requires you
//!   to "hard-code" what ECS access you use (the system that provides the ECS access needs to
//!   decide whether it will provide `&mut World` or only specific accesses).
//! - Allows the closure with ECS access to borrow from the async task itself. Most other solutions
//!   require passing `'static` types, which prevents borrowing data from the async task.
//! - Allows you to await ECS access, allowing other futures to run concurrently, and
//!   (more importantly) allow later code to happen after the ECS access.
//! - Allows you to reuse the [`AsyncSystemState`] which maintains [`SystemParam`] state, including
//!   [`QueryState`]. This allows tools like [`Changed`] to work correctly across multiple ECS
//!   accesses.
//!
//! [`Update`]: bevy_app::Update
//! [`Local`]: bevy_ecs::system::Local
//! [`Changed`]: bevy_ecs::query::Changed
//! [`async_channel_pattern`]: https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/async_channel_pattern.rs
//! [`check_ready`]: bevy_tasks::futures::check_ready
//! [`Task`]: bevy_tasks::Task
//! [`async_compute`]: https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/async_compute.rs
//! [`QueryState`]: bevy_ecs::query::QueryState
//! [`SystemParam`]: bevy_ecs::system::SystemParam
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
