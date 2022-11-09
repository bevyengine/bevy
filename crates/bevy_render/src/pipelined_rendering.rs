use async_channel::{Receiver, Sender};
use bevy_app::{App, SubApp};
use bevy_ecs::{
    schedule::MainThreadExecutor,
    system::Resource,
    world::{Mut, World},
};
use bevy_tasks::ComputeTaskPool;

#[cfg(feature = "trace")]
use bevy_utils::tracing::Instrument;

use crate::{PipelinedRenderingApp, RenderApp};

/// Resource to be used for pipelined rendering for sending the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Resource used by pipelined rendering to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

/// sets up the render thread and insert resource into the main app for controlling the render thread
pub fn setup_rendering(app: &mut App) {
    // skip this if pipelined rendering is not enabled
    if app.get_sub_app(PipelinedRenderingApp).is_err() {
        return;
    }

    let (app_to_render_sender, app_to_render_receiver) = async_channel::bounded::<SubApp>(1);
    let (render_to_app_sender, render_to_app_receiver) = async_channel::bounded::<SubApp>(1);

    let render_app = app.remove_sub_app(RenderApp).unwrap();
    render_to_app_sender.send_blocking(render_app).unwrap();

    app.insert_resource(MainToRenderAppSender(app_to_render_sender));
    app.insert_resource(RenderToMainAppReceiver(render_to_app_receiver));

    let render_task = async move {
        loop {
            // TODO: exit loop when app is exited
            let recv_task = app_to_render_receiver.recv();
            let mut sub_app = recv_task.await.unwrap();
            sub_app.run();
            render_to_app_sender.send(sub_app).await.unwrap();
        }
    };
    #[cfg(feature = "trace")]
    let span = bevy_utils::tracing::info_span!("render app");
    #[cfg(feature = "trace")]
    let render_task = render_task.instrument(span);
    ComputeTaskPool::get().spawn(render_task).detach();
}

pub fn update_rendering(app_world: &mut World) {
    // wait to get the render app back to signal that rendering is finished
    let mut render_app = app_world
        .resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
            ComputeTaskPool::get()
                .scope(Some(main_thread_executor.0.clone()), |s| {
                    s.spawn(async {
                        let receiver = world.get_resource::<RenderToMainAppReceiver>().unwrap();
                        let recv = receiver.0.recv();
                        recv.await.unwrap()
                    });
                })
                .pop()
        })
        .unwrap();

    render_app.extract(app_world);

    app_world.resource_scope(|_world, sender: Mut<MainToRenderAppSender>| {
        sender.0.send_blocking(render_app).unwrap();
    });
    // frame pacing plugin should run here somehow. i.e. after rendering, but before input handling
}
