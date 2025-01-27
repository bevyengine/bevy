use std::sync::{Arc, Mutex};

use bevy_app::{App, AppExit, AppLabel, DontUpdateOnUpdate, Plugin, SubApp};
use bevy_ecs::{
    resource::Resource,
    schedule::{MainThreadExecutor, ScheduleLabel},
    system::Res,
    world::{Mut, World},
};
use bevy_window::{UpdateSubAppOnWindowEvent, WindowEventKind};

use crate::RenderApp;

/// A Label for the sub app that runs the parts of pipelined rendering that need to run on the main thread.
///
/// The Main schedule of this app can be used to run logic after the render schedule starts, but
/// before I/O processing. This can be useful for something like frame pacing.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderExtractApp;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, ScheduleLabel)]
struct PipelineMain;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SharedRenderCommand {
    FinalizeExtract,
    Render,
}

struct SharedRenderStateInner {
    sub_app: Mutex<Option<SubApp>>,
    tx: async_channel::Sender<SharedRenderCommand>,
    rx: async_channel::Receiver<SharedRenderCommand>,
}

#[derive(Resource, Clone)]
pub struct SharedRenderState {
    inner: Arc<SharedRenderStateInner>,
}

impl SharedRenderState {
    pub fn new(sub_app: SubApp) -> Self {
        let (tx, rx) = async_channel::bounded(2);

        SharedRenderState {
            inner: Arc::new(SharedRenderStateInner {
                sub_app: Mutex::new(Some(sub_app)),
                tx,
                rx,
            }),
        }
    }

    pub fn blocking_with_mut(&self, f: impl FnOnce(Option<&mut SubApp>)) {
        match self.inner.sub_app.lock() {
            Ok(mut sub_app) => f(sub_app.as_mut()),
            Err(_) => f(None),
        }
    }

    fn block_for_command(&self) -> Option<SharedRenderCommand> {
        self.inner.rx.recv_blocking().ok()
    }

    pub fn queue_finalize_extract(&self) {
        let _ = self
            .inner
            .tx
            .send_blocking(SharedRenderCommand::FinalizeExtract);
    }

    pub fn queue_render(&self) {
        let _ = self.inner.tx.send_blocking(SharedRenderCommand::Render);
    }
}

impl Drop for SharedRenderState {
    fn drop(&mut self) {
        if let Ok(mut sub_app) = self.inner.sub_app.lock() {
            drop(sub_app.take());
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
///     This is run on the main app's thread. For more information checkout [`SyncWorldPlugin`].
/// - `extract` is the step where data is copied from the main world to the render world.
///     This is run on the main app's thread.
/// - On the render thread, we first apply the `extract commands`. This is not run during extract, so the
///     main schedule can start sooner.
/// - Then the `rendering schedule` is run. See [`RenderSet`](crate::RenderSet) for the standard steps in this process.
/// - In parallel to the rendering thread the [`RenderExtractApp`] schedule runs. By
///     default, this schedule is empty. But it is useful if you need something to run before I/O processing.
/// - Next all the `winit events` are processed.
/// - And finally the `main app schedule` is run.
/// - Once both the `main app schedule` and the `render schedule` are finished running, `extract` is run again.
///
/// [`SyncWorldPlugin`]: crate::sync_world::SyncWorldPlugin
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
        sub_app
            .set_extract(renderer_extract)
            .init_schedule(PipelineMain)
            .add_systems(
                PipelineMain,
                |shared_render_state: Res<SharedRenderState>| shared_render_state.queue_render(),
            );
        sub_app.update_schedule = Some(PipelineMain.intern());
        app.insert_sub_app(RenderExtractApp, sub_app);
    }

    // Sets up the render thread and inserts resources into the main app used for controlling the render thread.
    fn cleanup(&self, app: &mut App) {
        // skip setting up when headless
        if app.get_sub_app(RenderExtractApp).is_none() {
            return;
        }

        let mut render_app = app
            .remove_sub_app(RenderApp)
            .expect("Unable to get RenderApp. Another plugin may have removed the RenderApp before PipelinedRenderingPlugin");

        // clone main thread executor to render world
        let executor = app.world().get_resource::<MainThreadExecutor>().unwrap();
        render_app.world_mut().insert_resource(executor.clone());

        let render_sub_app = SharedRenderState::new(render_app);
        app.insert_resource(render_sub_app.clone());
        app.get_sub_app_mut(RenderExtractApp)
            .expect("Unable to get RenderExtractApp. Another plugin might have removed it.")
            .insert_resource(render_sub_app.clone());

        app.world_mut()
            .resource_mut::<DontUpdateOnUpdate>()
            .remove(RenderApp)
            .add(RenderExtractApp);
        app.world_mut()
            .resource_mut::<UpdateSubAppOnWindowEvent>()
            .remove(WindowEventKind::RequestRedraw, RenderApp)
            .add(WindowEventKind::RequestRedraw, RenderExtractApp);

        std::thread::spawn(move || {
            #[cfg(feature = "trace")]
            let _span = tracing::info_span!("render thread").entered();

            let mut render_sub_app_exists = true;
            while render_sub_app_exists {
                match render_sub_app.block_for_command() {
                    Some(command) => render_sub_app.blocking_with_mut(|render_app| {
                        if let Some(render_app) = render_app {
                            match command {
                                SharedRenderCommand::FinalizeExtract => {
                                    render_app.finalize_extract()
                                }
                                SharedRenderCommand::Render => {
                                    #[cfg(feature = "trace")]
                                    let _sub_app_span =
                                        tracing::info_span!("sub app", name = ?RenderApp).entered();
                                    render_app.update();
                                }
                            }
                        } else {
                            render_sub_app_exists = false;
                        }
                    }),
                    None => break,
                }
            }

            tracing::debug!("exiting pipelined rendering thread");
        });
    }
}

// This function waits for the rendering world to be available for exclusive access,
// runs extract, and frees it to be accessed by the render thread.
fn renderer_extract(app_world: &mut World, _world: &mut World) {
    app_world.resource_scope(|world, shared_render_state: Mut<SharedRenderState>| {
        shared_render_state.blocking_with_mut(|render_app| {
            if let Some(render_app) = render_app {
                render_app.extract(world);
                shared_render_state.queue_finalize_extract();
            } else {
                // Renderer thread panicked causing the mutex to get poisonied
                world.send_event(AppExit::error());
            }
        });
    });
}
