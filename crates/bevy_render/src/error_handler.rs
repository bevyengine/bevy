use alloc::sync::Arc;
use bevy_app::AppExit;
use bevy_ecs::{resource::Resource, world::World};
use std::sync::Mutex;
use wgpu::ErrorSource;
use wgpu_types::error::ErrorType;

use crate::{
    insert_future_resources,
    render_resource::PipelineCache,
    renderer::RenderDevice,
    settings::{RenderCreation, WgpuSettings},
    FutureRenderResources,
};

/// Resource to indicate renderer behavior upon error.
#[derive(Resource, Default)]
pub enum RenderErrorPolicy {
    /// Panics on error.
    Panic,
    #[default]
    /// Signals app exit on error.
    Shutdown,
    /// Keeps the app alive, but stops rendering further.
    StopRendering,
    /// Attempt renderer recovery the given number of times.
    Recover(usize, WgpuSettings),
}

/// The current state of the renderer.
#[derive(Resource, Debug)]
pub(crate) enum RenderState {
    /// Just started, [`RenderStartup`] will run in this state.
    Initializing,
    /// Everything is okay and we are rendering stuff every frame.
    Ready,
    /// An error was encountered, and we may decide how to handle it.
    Errored(ErrorType, String, Option<ErrorSource>),
    /// We are recreating the render context after an error to recover.
    Reinitializing,
}

/// Resource to allows polling wgpu error handlers.
#[derive(Resource)]
pub(crate) struct RenderErrorHandler {
    device_lost: Arc<Mutex<Option<(wgpu::DeviceLostReason, String)>>>,
    uncaptured: Arc<Mutex<Option<wgpu::Error>>>,
}

impl RenderErrorHandler {
    /// Creates and registers error handlers on the given device and stores them to later be polled.
    pub(crate) fn new(device: &RenderDevice) -> Self {
        let device_lost = Arc::new(Mutex::new(None));
        let uncaptured = Arc::new(Mutex::new(None));
        {
            // scoped clone to move into closures
            let device_lost = device_lost.clone();
            let uncaptured = uncaptured.clone();
            let device = device.wgpu_device();
            // we log errors as soon as they are captured so they stay chronological in logs
            // and only keep the first error, as it often causes other errors downstream
            device.set_device_lost_callback(move |reason, str| {
                bevy_log::error!("Caught DeviceLost error: {reason:?} {str}");
                assert!(device_lost.lock().unwrap().replace((reason, str)).is_none());
            });
            device.on_uncaptured_error(Arc::new(move |e| {
                bevy_log::error!("Caught rendering error: {e}");
                uncaptured.lock().unwrap().get_or_insert(e);
            }));
        }
        Self {
            device_lost,
            uncaptured,
        }
    }

    /// Checks to see if any errors have been caught, and returns an appropriate `RenderState`
    pub(crate) fn poll(&self) -> RenderState {
        if let Some(error) = self.uncaptured.lock().unwrap().take() {
            let (ty, str, source) = match error {
                wgpu::Error::OutOfMemory { source } => {
                    (ErrorType::OutOfMemory, "".to_string(), source)
                }
                wgpu::Error::Validation {
                    source,
                    description,
                } => (ErrorType::Validation, description, source),
                wgpu::Error::Internal {
                    source,
                    description,
                } => (ErrorType::Internal, description, source),
            };
            return RenderState::Errored(ty, str, Some(source));
        }
        // Device lost is more important so we let it take precedence; every error gets logged anyways.
        if let Some((_, str)) = self.device_lost.lock().unwrap().take() {
            return RenderState::Errored(ErrorType::DeviceLost, str, None);
        }
        RenderState::Ready
    }
}

/// We need both the main and render world to properly handle errors, so we wedge ourselves into Extract.
/// Returns true if `RenderStartup` should be run.
pub(crate) fn update(main_world: &mut World, render_world: &mut World) -> bool {
    match render_world.resource::<RenderState>() {
        RenderState::Initializing => {
            render_world.insert_resource(RenderState::Ready);
            return true;
        }
        RenderState::Ready => {
            // all is well
        }
        RenderState::Errored(error_type, str, source) => {
            match main_world.resource::<RenderErrorPolicy>() {
                RenderErrorPolicy::Panic => {
                    panic!("Rendering error {error_type:?}: {str} in {source:?}");
                }
                RenderErrorPolicy::Shutdown => {
                    // error was already logged by `RenderErrorHandler`
                    main_world.write_message(AppExit::error());
                }
                RenderErrorPolicy::StopRendering => {
                    // do nothing
                }
                RenderErrorPolicy::Recover(i, settings) => {
                    if *i > 0 {
                        render_world.insert_resource(RenderState::Reinitializing);
                        render_world
                            .insert_resource(RenderErrorPolicy::Recover(i - 1, settings.clone()));
                        assert!(insert_future_resources(
                            &RenderCreation::Automatic(settings.clone()),
                            main_world
                        ));
                    }
                }
            }
        }
        RenderState::Reinitializing => {
            if let Some(render_resources) = main_world
                .get_resource::<FutureRenderResources>()
                .unwrap()
                .clone()
                .lock()
                .unwrap()
                .take()
            {
                let synchronous_pipeline_compilation = render_world
                    .resource::<PipelineCache>()
                    .synchronous_pipeline_compilation;
                render_resources.unpack_into(
                    main_world,
                    render_world,
                    synchronous_pipeline_compilation,
                );
                render_world.insert_resource(RenderState::Initializing);
            }
        }
    }
    false
}
