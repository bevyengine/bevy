use core::panic::AssertUnwindSafe;
use std::panic::{catch_unwind, resume_unwind};

use async_channel::{Receiver, Sender};

use bevy_app::{App, AppExit, Plugin, SubApp};
use bevy_derive::AppLabel;
use bevy_ecs::{
    resource::Resource,
    schedule::MainThreadExecutor,
    world::{Mut, World},
};
use bevy_tasks::ComputeTaskPool;

use crate::RenderApp;

/// A Label for the sub app that runs the parts of pipelined rendering that need to run on the main thread.
///
/// The Main schedule of this app can be used to run logic after the render schedule starts, but
/// before I/O processing. This can be useful for something like frame pacing.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel, Default)]
pub struct RenderExtractApp;

/// Channels used by the main app to send and receive the render app.
#[derive(Resource)]
pub struct RenderAppChannels {
    app_to_render_sender: Sender<SubApp>,
    render_to_app_receiver: Receiver<SubApp>,
    render_app_in_render_thread: bool,
    /// Pumped during shutdown so the render app's main-thread tasks can finish.
    main_thread_executor: MainThreadExecutor,
}

impl RenderAppChannels {
    /// Create a `RenderAppChannels` from a [`async_channel::Receiver`] and [`async_channel::Sender`].
    ///
    /// `main_thread_executor` must be the [`MainThreadExecutor`] shared with the render world.
    /// It is pumped during shutdown so the final render update can complete.
    pub fn new(
        app_to_render_sender: Sender<SubApp>,
        render_to_app_receiver: Receiver<SubApp>,
        main_thread_executor: MainThreadExecutor,
    ) -> Self {
        Self {
            app_to_render_sender,
            render_to_app_receiver,
            render_app_in_render_thread: false,
            main_thread_executor,
        }
    }

    /// Send the `render_app` to the rendering thread.
    pub fn send_blocking(&mut self, render_app: SubApp) {
        self.app_to_render_sender.send_blocking(render_app).unwrap();
        self.render_app_in_render_thread = true;
    }

    /// Receive the `render_app` from the rendering thread.
    /// Return `None` if the render thread has panicked.
    pub async fn recv(&mut self) -> Option<SubApp> {
        let render_app = self.render_to_app_receiver.recv().await.ok()?;
        self.render_app_in_render_thread = false;
        Some(render_app)
    }
}

impl Drop for RenderAppChannels {
    fn drop(&mut self) {
        if self.render_app_in_render_thread {
            // The render world's non-send data was initialized on the main thread,
            // so wait for the render app to return and drop it here.
            //
            // A blocking receive would deadlock if the final render update has queued
            // main-thread tasks. Pump their executor while waiting, as renderer_extract does.
            let result = catch_unwind(AssertUnwindSafe(|| {
                ComputeTaskPool::get().scope_with_executor(
                    true,
                    Some(&self.main_thread_executor.0),
                    |scope| {
                        scope.spawn(async { self.render_to_app_receiver.recv().await.ok() });
                    },
                );
            }));

            if let Err(payload) = result {
                if std::thread::panicking() {
                    // The scope and the recovered render app's destructors can panic.
                    // Preserve an existing panic instead of aborting with a double panic.
                    // Even dropping the new panic's payload could panic again.
                    core::mem::forget(payload);
                } else {
                    resume_unwind(payload);
                }
            }
        }
    }
}

/// The [`PipelinedRenderingPlugin`] can be added to your application to enable pipelined rendering.
///
/// This moves rendering into a different thread, so that the Nth frame's rendering can
/// be run at the same time as the N + 1 frame's simulation.
///
/// ```text
/// |--------------------|--------------------|--------------------|--------------------|
/// | simulation thread  | frame 1 simulation | frame 2 simulation | frame 3 simulation |
/// |--------------------|--------------------|--------------------|--------------------|
/// | rendering thread   |                    | frame 1 rendering  | frame 2 rendering  |
/// |--------------------|--------------------|--------------------|--------------------|
/// ```
///
/// The plugin is dependent on the [`RenderApp`] added by [`crate::RenderPlugin`] and so must
/// be added after that plugin. If it is not added after, the plugin will do nothing.
///
/// A single frame of execution looks something like below
///
/// ```text
/// |---------------------------------------------------------------------------|
/// |      |         | RenderExtractApp schedule | winit events | main schedule |
/// | sync | extract |----------------------------------------------------------|
/// |      |         | extract commands | rendering schedule                    |
/// |---------------------------------------------------------------------------|
/// ```
///
/// - `sync` is the step where the entity-entity mapping between the main and render world is updated.
///   This is run on the main app's thread. For more information checkout [`SyncWorldPlugin`].
/// - `extract` is the step where data is copied from the main world to the render world.
///   This is run on the main app's thread.
/// - On the render thread, we first apply the `extract commands`. This is not run during extract, so the
///   main schedule can start sooner.
/// - Then the `rendering schedule` is run. See [`RenderSystems`](crate::RenderSystems) for the standard steps in this process.
/// - In parallel to the rendering thread the [`RenderExtractApp`] schedule runs. By
///   default, this schedule is empty. But it is useful if you need something to run before I/O processing.
/// - Next all the `winit events` are processed.
/// - And finally the `main app schedule` is run.
/// - Once both the `main app schedule` and the `render schedule` are finished running, `extract` is run again.
///
/// [`SyncWorldPlugin`]: bevy_extract::sync_world::SyncWorldPlugin
#[derive(Default)]
pub struct PipelinedRenderingPlugin;

impl Plugin for PipelinedRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Don't add RenderExtractApp if RenderApp isn't initialized.
        if app.get_sub_app(RenderApp).is_none() {
            return;
        }
        app.insert_resource(MainThreadExecutor::new());

        let mut sub_app = SubApp::new();
        sub_app.set_extract(renderer_extract);
        app.insert_sub_app(RenderExtractApp, sub_app);
    }

    // Sets up the render thread and inserts resources into the main app used for controlling the render thread.
    fn cleanup(&self, app: &mut App) {
        // skip setting up when headless
        if app.get_sub_app(RenderExtractApp).is_none() {
            return;
        }

        let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
        let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);

        let mut render_app = app
            .remove_sub_app(RenderApp)
            .expect("Unable to get RenderApp. Another plugin may have removed the RenderApp before PipelinedRenderingPlugin");

        // clone main thread executor to render world
        let executor = app.world().resource::<MainThreadExecutor>().clone();
        render_app.world_mut().insert_resource(executor.clone());

        render_to_app_sender.send_blocking(render_app).unwrap();

        app.insert_resource(RenderAppChannels::new(
            app_to_render_sender,
            render_to_app_receiver,
            executor,
        ));

        std::thread::Builder::new()
            .name("Render thread".into())
            .spawn(move || {
                #[cfg(feature = "trace")]
                let _span = bevy_log::info_span!("render thread").entered();

                let compute_task_pool = ComputeTaskPool::get();
                loop {
                    // run a scope here to allow main world to use this thread while it's waiting for the render app
                    let sent_app = compute_task_pool
                        .scope(|s| {
                            s.spawn(async { app_to_render_receiver.recv().await });
                        })
                        .pop();
                    let Some(Ok(mut render_app)) = sent_app else {
                        break;
                    };

                    {
                        #[cfg(feature = "trace")]
                        let _sub_app_span =
                            bevy_log::info_span!("sub app", name = ?RenderApp).entered();
                        render_app.update();
                    }

                    if render_to_app_sender.send_blocking(render_app).is_err() {
                        break;
                    }
                }

                bevy_log::debug!("exiting pipelined rendering thread");
            })
            .expect("Failed to create render thread");
    }
}

// This function waits for the rendering world to be received,
// runs extract, and then sends the rendering world back to the render thread.
fn renderer_extract(app_world: &mut World, _world: &mut World) {
    app_world.resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
        world.resource_scope(|world, mut render_channels: Mut<RenderAppChannels>| {
            // we use a scope here to run any main thread tasks that the render world still needs to run
            // while we wait for the render world to be received.
            if let Some(mut render_app) = ComputeTaskPool::get()
                .scope_with_executor(true, Some(&*main_thread_executor.0), |s| {
                    s.spawn(async { render_channels.recv().await });
                })
                .pop()
                .unwrap()
            {
                render_app.extract(world);

                render_channels.send_blocking(render_app);
            } else {
                // Renderer thread panicked
                world.write_message(AppExit::error());
            }
        });
    });
}

#[cfg(all(test, feature = "multi_threaded"))]
mod tests {
    use super::*;
    use bevy_tasks::{block_on, TaskPool};
    use core::time::Duration;
    use std::{
        sync::mpsc,
        thread::{self, JoinHandle, ThreadId},
    };

    const TIMEOUT: Duration = Duration::from_secs(10);

    // The executor must be created and ticked on the synthetic main thread.
    // Keep the test thread free to time out even if shutdown deadlocks.
    fn run_on_main_thread(test: impl FnOnce() + Send + 'static) {
        ComputeTaskPool::get_or_init(TaskPool::new);
        let (done_sender, done_receiver) = mpsc::channel();
        let main_thread = thread::spawn(move || {
            test();
            done_sender.send(()).unwrap();
        });

        done_receiver
            .recv_timeout(TIMEOUT)
            .expect("render shutdown did not finish");
        main_thread.join().unwrap();
    }

    fn start_render_thread(render_app: SubApp) -> (RenderAppChannels, JoinHandle<()>) {
        let main_thread_id = thread::current().id();
        let executor = MainThreadExecutor::new();
        let render_executor = executor.clone();
        let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
        let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);
        let mut channels =
            RenderAppChannels::new(app_to_render_sender, render_to_app_receiver, executor);
        channels.send_blocking(render_app);

        let (queued_sender, queued_receiver) = mpsc::channel();
        let render_thread = thread::spawn(move || {
            let render_app = app_to_render_receiver.recv_blocking().unwrap();
            let task = render_executor.0.spawn(async move {
                assert_eq!(thread::current().id(), main_thread_id);
            });
            queued_sender.send(()).unwrap();
            block_on(task);
            render_to_app_sender.send_blocking(render_app).unwrap();
        });

        // Ensure shutdown encounters pending main-thread work, regardless of scheduling.
        queued_receiver.recv_timeout(TIMEOUT).unwrap();
        (channels, render_thread)
    }

    struct NotifyOnDrop(mpsc::Sender<ThreadId>);

    impl Drop for NotifyOnDrop {
        fn drop(&mut self) {
            self.0.send(thread::current().id()).unwrap();
        }
    }

    struct PanicOnDrop;

    impl Drop for PanicOnDrop {
        fn drop(&mut self) {
            panic!("render app drop panic");
        }
    }

    #[test]
    fn drop_pumps_main_thread_executor_to_avoid_shutdown_deadlock() {
        run_on_main_thread(|| {
            let main_thread_id = thread::current().id();
            let (dropped_sender, dropped_receiver) = mpsc::channel();
            let mut render_app = SubApp::new();
            render_app
                .world_mut()
                .insert_non_send(NotifyOnDrop(dropped_sender));
            let (channels, render_thread) = start_render_thread(render_app);

            drop(channels);

            render_thread.join().unwrap();
            assert_eq!(
                dropped_receiver.recv_timeout(TIMEOUT).unwrap(),
                main_thread_id
            );
        });
    }

    #[test]
    fn drop_suppresses_render_app_drop_panic_during_unwind() {
        run_on_main_thread(|| {
            let mut render_app = SubApp::new();
            render_app.world_mut().insert_non_send(PanicOnDrop);
            let (channels, render_thread) = start_render_thread(render_app);

            let result = catch_unwind(AssertUnwindSafe(move || {
                let _channels = channels;
                panic!("main app panic");
            }));

            assert_eq!(
                result.unwrap_err().downcast_ref::<&str>(),
                Some(&"main app panic")
            );
            render_thread.join().unwrap();
        });
    }

    #[test]
    fn drop_propagates_render_app_drop_panic_without_existing_panic() {
        run_on_main_thread(|| {
            let mut render_app = SubApp::new();
            render_app.world_mut().insert_non_send(PanicOnDrop);
            let (channels, render_thread) = start_render_thread(render_app);

            let result = catch_unwind(AssertUnwindSafe(|| drop(channels)));

            assert_eq!(
                result.unwrap_err().downcast_ref::<&str>(),
                Some(&"render app drop panic")
            );
            render_thread.join().unwrap();
        });
    }
}
