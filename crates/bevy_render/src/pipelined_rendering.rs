use async_channel::{Receiver, Sender};

use bevy_app::{App, AppLabel, Plugin, SubApp};
use bevy_ecs::{
    schedule::{MainThreadExecutor, StageLabel, SystemStage},
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
    /// |-------------------------------------------------------------------|
    /// |         | BeforeIoAfterRenderStart | winit events | main schedule |
    /// | extract |---------------------------------------------------------|
    /// |         | rendering schedule                                      |
    /// |-------------------------------------------------------------------|
    BeforeIoAfterRenderStart,
}

/// Resource for pipelined rendering to send the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Resource for pipelined rendering to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

#[derive(Default)]
pub struct PipelinedRenderingPlugin;
impl Plugin for PipelinedRenderingPlugin {
    fn build(&self, app: &mut App) {
        let mut sub_app = App::new();
        app.set_setup(setup_rendering);
        sub_app.add_stage(
            RenderExtractStage::BeforeIoAfterRenderStart,
            SystemStage::parallel(),
        );
        app.add_sub_app(RenderExtractApp, sub_app, update_rendering, |render_app| {
            render_app.run();
        });
    }
}

// Sets up the render thread and inserts resources into the main app used for controlling the render thread.
// This should be called after plugins have all been built as it removes the rendering sub app from the main app.
// This does nothing if pipelined rendering is not enabled.
fn setup_rendering(app: &mut App) {
    // skip setting up when headless
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
fn update_rendering(app_world: &mut World, _sub_app: &mut App) {
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
