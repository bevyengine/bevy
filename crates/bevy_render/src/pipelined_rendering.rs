use async_channel::{Receiver, Sender};

use bevy_app::{App, AppLabel, Main, Plugin, SubApp};
use bevy_ecs::{
    schedule::MainThreadExecutor,
    system::Resource,
    world::{Mut, World},
};
use bevy_tasks::ComputeTaskPool;

use crate::RenderApp;

/// A Label for the sub app that runs the parts of pipelined rendering that need to run on the main thread.
///
/// The Main schedule of this app can be used to run logic after the render schedule starts, but
/// before I/O processing. This can be useful for something like frame pacing.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderExtractApp;

/// Channel to send the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Channel to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

/// The [`PipelinedRenderingPlugin`] can be added to your application to enable pipelined rendering.
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
/// The plugin is dependent on the [`crate::RenderApp`] added by [`crate::RenderPlugin`] and so must
/// be added after that plugin. If it is not added after, the plugin will do nothing.
///
/// A single frame of execution looks something like below    
///
/// ```text
/// |--------------------------------------------------------------------|
/// |         | RenderExtractApp schedule | winit events | main schedule |
/// | extract |----------------------------------------------------------|
/// |         | extract commands | rendering schedule                    |
/// |--------------------------------------------------------------------|
/// ```
///
/// - `extract` is the step where data is copied from the main world to the render world.
/// This is run on the main app's thread.
/// - On the render thread, we first apply the `extract commands`. This is not run during extract, so the
/// main schedule can start sooner.
/// - Then the `rendering schedule` is run. See [`RenderSet`](crate::RenderSet) for the standard steps in this process.
/// - In parallel to the rendering thread the [`RenderExtractApp`] schedule runs. By
/// default this schedule is empty. But it is useful if you need something to run before I/O processing.
/// - Next all the `winit events` are processed.
/// - And finally the `main app schedule` is run.
/// - Once both the `main app schedule` and the `render schedule` are finished running, `extract` is run again.
#[derive(Default)]
pub struct PipelinedRenderingPlugin;

impl Plugin for PipelinedRenderingPlugin {
    fn build(&self, app: &mut App) {
        // Don't add RenderExtractApp if RenderApp isn't initialized.
        if app.get_sub_app(RenderApp).is_err() {
            return;
        }
        app.insert_resource(MainThreadExecutor::new());

        let mut sub_app = App::empty();
        sub_app.init_schedule(Main);
        app.insert_sub_app(RenderExtractApp, SubApp::new(sub_app, update_rendering));
    }

    // Sets up the render thread and inserts resources into the main app used for controlling the render thread.
    fn cleanup(&self, app: &mut App) {
        // skip setting up when headless
        if app.get_sub_app(RenderExtractApp).is_err() {
            return;
        }

        let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
        let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);

        let mut render_app = app
            .remove_sub_app(RenderApp)
            .expect("Unable to get RenderApp. Another plugin may have removed the RenderApp before PipelinedRenderingPlugin");

        // clone main thread executor to render world
        let executor = app.world.get_resource::<MainThreadExecutor>().unwrap();
        render_app.app.world.insert_resource(executor.clone());

        render_to_app_sender.send_blocking(render_app).unwrap();

        app.insert_resource(MainToRenderAppSender(app_to_render_sender));
        app.insert_resource(RenderToMainAppReceiver(render_to_app_receiver));

        std::thread::spawn(move || {
            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("render thread").entered();

            loop {
                // run a scope here to allow main world to use this thread while it's waiting for the render app
                let mut render_app = ComputeTaskPool::get()
                    .scope(|s| {
                        s.spawn(async { app_to_render_receiver.recv().await.unwrap() });
                    })
                    .pop()
                    .unwrap();

                #[cfg(feature = "trace")]
                let _sub_app_span =
                    bevy_utils::tracing::info_span!("sub app", name = ?RenderApp).entered();
                render_app.run();
                render_to_app_sender.send_blocking(render_app).unwrap();
            }
        });
    }
}

// This function waits for the rendering world to be received,
// runs extract, and then sends the rendering world back to the render thread.
fn update_rendering(app_world: &mut World, _sub_app: &mut App) {
    app_world.resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
        // we use a scope here to run any main thread tasks that the render world still needs to run
        // while we wait for the render world to be received.
        let mut render_app = ComputeTaskPool::get()
            .scope_with_executor(true, Some(&*main_thread_executor.0), |s| {
                s.spawn(async {
                    let receiver = world.get_resource::<RenderToMainAppReceiver>().unwrap();
                    receiver.0.recv().await.unwrap()
                });
            })
            .pop()
            .unwrap();

        render_app.extract(world);

        let sender = world.resource::<MainToRenderAppSender>();
        sender.0.send_blocking(render_app).unwrap();
    });
}
