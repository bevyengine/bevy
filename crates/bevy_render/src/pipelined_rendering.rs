use async_channel::{Receiver, Sender};
use bevy_app::{App, SubApp};
use bevy_ecs::{schedule::MainThreadExecutor, system::Resource, world::Mut};
use bevy_tasks::ComputeTaskPool;

#[cfg(feature = "trace")]
use bevy_utils::tracing::Instrument;

use crate::RenderApp;

/// Resource to be used for pipelined rendering for sending the render app from the main thread to the rendering thread
#[derive(Resource)]
pub struct MainToRenderAppSender(pub Sender<SubApp>);

/// Resource used by pipelined rendering to send the render app from the render thread to the main thread
#[derive(Resource)]
pub struct RenderToMainAppReceiver(pub Receiver<SubApp>);

/// sets up the render thread and insert resource into the main app for controlling the render thread
pub fn setup_pipelined_rendering(app: &mut App) {
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
            #[cfg(feature = "trace")]
            let span = bevy_utils::tracing::info_span!("receive render world from main");
            #[cfg(feature = "trace")]
            let recv_task = recv_task.instrument(span);
            let mut sub_app = recv_task.await.unwrap();
            sub_app.run();
            render_to_app_sender.send(sub_app).await.unwrap();
        }
    };
    #[cfg(feature = "trace")]
    let span = bevy_utils::tracing::info_span!("render task");
    #[cfg(feature = "trace")]
    let render_task = render_task.instrument(span);
    ComputeTaskPool::get().spawn(render_task).detach();
}

pub fn update_rendering(app: &mut App) {
    app.update();

    // wait to get the render app back to signal that rendering is finished
    let mut render_app = app
        .world
        .resource_scope(|world, main_thread_executor: Mut<MainThreadExecutor>| {
            ComputeTaskPool::get()
                .scope(Some(main_thread_executor.0.clone()), |s| {
                    s.spawn(async {
                        let receiver = world.get_resource::<RenderToMainAppReceiver>().unwrap();
                        let recv = receiver.0.recv();
                        #[cfg(feature = "trace")]
                        let span = bevy_utils::tracing::info_span!("wait for render");
                        #[cfg(feature = "trace")]
                        let recv = recv.instrument(span);
                        recv.await.unwrap()
                    });
                })
                .pop()
        })
        .unwrap();

    render_app.extract(&mut app.world);

    {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("send world to render").entered();
        app.world
            .resource_scope(|_world, sender: Mut<MainToRenderAppSender>| {
                sender.0.send_blocking(render_app).unwrap();
            });
    }

    // frame pacing plugin should run here somehow. i.e. after rendering, but before input handling
}
