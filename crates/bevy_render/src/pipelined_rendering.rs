use async_channel::{Receiver, Sender};

use bevy_app::{App, AppLabel, SubApp};
use bevy_ecs::{
    schedule::{MainThreadExecutor, Stage, StageLabel, SystemStage},
    system::Resource,
    world::{Mut, World},
};
use bevy_tasks::ComputeTaskPool;

use crate::RenderApp;

/// A Label for the sub app that runs the parts of pipelined rendering that need to run on the main thread.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
pub struct RenderExtractApp;

/// Labels for stages in the sub app that syncs with the rendering task.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum RenderExtractStage {
    /// This stage runs after the render schedule starts, but before I/O processing and the main app schedule.
    /// This can be useful for something like frame pacing.
    /// |-----------------------------------------------------------------|
    /// |         | BeforeIoAfterRendering | winit events | main schedule |
    /// | extract |-------------------------------------------------------|
    /// |         | rendering schedule                                    |
    /// |-----------------------------------------------------------------|
    BeforeIoAfterRendering,
}

/// Resource for pipelined rendering to send the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Resource for pipelined rendering to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

pub(crate) fn build_pipelined_rendering(main_app: &mut App) {
    let mut app = App::new();
    main_app.set_setup(setup_rendering);
    main_app.add_stage(
        RenderExtractStage::BeforeIoAfterRendering,
        SystemStage::parallel(),
    );
    app.add_sub_app(
        RenderExtractApp,
        App::new(),
        |app_world, _render_app| {
            update_rendering(app_world);
        },
        |render_app| {
            {
                #[cfg(feature = "trace")]
                let _stage_span =
                    bevy_utils::tracing::info_span!("stage", name = "before_io_after_rendering")
                        .entered();

                // render
                let render = render_app
                    .schedule
                    .get_stage_mut::<SystemStage>(RenderExtractStage::BeforeIoAfterRendering)
                    .unwrap();
                render.run(&mut render_app.world);
            }
        },
    );
}

// Sets up the render thread and inserts resources into the main app used for controlling the render thread.
// This should be called after plugins have all been built as it removes the rendering sub app from the main app.
// This does nothing if pipelined rendering is not enabled.
fn setup_rendering(app: &mut App) {
    if app.get_sub_app(RenderExtractApp).is_err() {
        return;
    }

    let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
    let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);

    let render_app = app.remove_sub_app(RenderApp).unwrap();
    render_to_app_sender.send_blocking(render_app).unwrap();

    app.insert_resource(MainToRenderAppSender(app_to_render_sender));
    app.insert_resource(RenderToMainAppReceiver(render_to_app_receiver));

    ComputeTaskPool::get()
        .spawn(async move {
            loop {
                // TODO: exit loop when app is exited
                let recv_task = app_to_render_receiver.recv();
                let mut sub_app = recv_task.await.unwrap();
                sub_app.run();
                render_to_app_sender.send(sub_app).await.unwrap();
            }
        })
        .detach();
}

// This function is used for synchronizing the main app with the render world.
// Do not call this function if pipelined rendering is not setup.
fn update_rendering(app_world: &mut World) {
    app_world.resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
        // we use a scope here to run any main thread tasks that the render world still needs to run
        // while we wait for the render world to be received.
        let mut render_app = ComputeTaskPool::get()
            .scope_with_executor(Some(main_thread_executor.0.clone()), |s| {
                s.spawn(async {
                    let receiver = world.get_resource::<RenderToMainAppReceiver>().unwrap();
                    receiver.0.recv().await.unwrap()
                });
            })
            .pop()
            .unwrap();

        render_app.extract(world);

        let sender = world.get_resource::<MainToRenderAppSender>().unwrap();
        sender.0.send_blocking(render_app).unwrap();
    });
}
