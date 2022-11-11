use async_channel::{Receiver, Sender};

use bevy_app::{App, SubApp};
use bevy_ecs::{
    schedule::MainThreadExecutor,
    system::Resource,
    world::{Mut, World},
};
use bevy_tasks::ComputeTaskPool;

use crate::{PipelinedRenderingApp, RenderApp};

/// Resource for pipelined rendering to send the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Resource for pipelined rendering to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

/// Sets up the render thread and inserts resources into the main app used for controlling the render thread
/// This does nothing if pipelined rendering is not enabled.
pub fn setup_rendering(app: &mut App) {
    if app.get_sub_app(PipelinedRenderingApp).is_err() {
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

/// This function is used for synchronizing the main app with the render world.
/// Do not call this function if pipelined rendering is not setup.
pub fn update_rendering(app_world: &mut World) {
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
