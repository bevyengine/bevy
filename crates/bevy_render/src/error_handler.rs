use alloc::sync::Arc;
use bevy_app::AppExit;
use bevy_ecs::{resource::Resource, world::World};
use std::sync::Mutex;
use wgpu::ErrorSource;
use wgpu_types::error::ErrorType;

use crate::{
    insert_future_resources,
    render_resource::PipelineCache,
    renderer::{RenderDevice, WgpuWrapper},
    settings::RenderCreation,
    FutureRenderResources,
};

/// Resource to indicate renderer behavior upon error.
#[expect(clippy::large_enum_variant, reason = "ergonomics")]
#[derive(Default)]
pub enum RenderErrorPolicy {
    /// Pretends nothing happened and continues rendering.
    #[default]
    Ignore,
    /// Panics on error.
    Panic,
    /// Signals app exit on error.
    Shutdown,
    /// Keeps the app alive, but stops rendering further.
    StopRendering,
    /// Attempt renderer recovery with the given [`RenderCreation`].
    Recover(RenderCreation),
}

/// Determines what [`RenderErrorPolicy`] should be used to respond to a given [`RenderError`].
#[derive(Resource)]
pub struct RenderErrorHandler(pub for<'a> fn(&'a RenderError) -> RenderErrorPolicy);

impl Default for RenderErrorHandler {
    fn default() -> Self {
        // This is what we've always done historically,
        // but we could choose a new default once recovery works better.
        Self(|_| RenderErrorPolicy::Ignore)
    }
}

/// An error encountered during rendering.
#[derive(Debug)]
pub struct RenderError {
    pub ty: ErrorType,
    pub description: String,
    pub source: Option<WgpuWrapper<ErrorSource>>,
}

/// The current state of the renderer.
#[derive(Resource, Debug)]
pub(crate) enum RenderState {
    /// Just started, [`crate::RenderStartup`] will run in this state.
    Initializing,
    /// Everything is okay and we are rendering stuff every frame.
    Ready,
    /// An error was encountered, and we may decide how to handle it.
    Errored(RenderError),
    /// We are recreating the render context after an error to recover.
    Reinitializing,
}

/// Resource to allow polling wgpu error handlers.
#[derive(Resource)]
pub(crate) struct DeviceErrorHandler {
    device_lost: Arc<Mutex<Option<(wgpu::DeviceLostReason, String)>>>,
    uncaptured: Arc<Mutex<Option<WgpuWrapper<wgpu::Error>>>>,
}

impl DeviceErrorHandler {
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
                uncaptured
                    .lock()
                    .unwrap()
                    .get_or_insert(WgpuWrapper::new(e));
            }));
        }
        Self {
            device_lost,
            uncaptured,
        }
    }

    /// Checks to see if any errors have been caught, and returns an appropriate `RenderState`
    pub(crate) fn poll(&self) -> Option<RenderError> {
        // Device lost is more important so we let it take precedence; every error gets logged anyways.
        if let Some((_, description)) = self.device_lost.lock().unwrap().take() {
            return Some(RenderError {
                ty: ErrorType::DeviceLost,
                description,
                source: None,
            });
        }
        if let Some(error) = self.uncaptured.lock().unwrap().take() {
            let (ty, description, source) = match error.into_inner() {
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
            return Some(RenderError {
                ty,
                description,
                source: Some(WgpuWrapper::new(source)),
            });
        }
        None
    }
}

/// We need both the main and render world to properly handle errors, so we wedge ourselves into Extract.
/// Returns true if `RenderStartup` should be run.
pub(crate) fn update_state(main_world: &mut World, render_world: &mut World) -> bool {
    if let Some(error) = render_world.resource::<DeviceErrorHandler>().poll() {
        render_world.insert_resource(RenderState::Errored(error));
    }
    match render_world.resource::<RenderState>() {
        RenderState::Initializing => {
            render_world.insert_resource(RenderState::Ready);
            return true;
        }
        RenderState::Ready => {
            // all is well
        }
        RenderState::Errored(error) => {
            match main_world.resource::<RenderErrorHandler>().0(error) {
                RenderErrorPolicy::Ignore => {
                    // Pretend that didn't happen.
                    render_world.insert_resource(RenderState::Ready);
                }
                RenderErrorPolicy::Panic => {
                    panic!("Rendering error {error:?}");
                }
                RenderErrorPolicy::Shutdown => {
                    // error was already logged by `DeviceErrorHandler`
                    main_world.write_message(AppExit::error());
                }
                RenderErrorPolicy::StopRendering => {
                    // do nothing
                }
                RenderErrorPolicy::Recover(render_creation) => {
                    assert!(insert_future_resources(&render_creation, main_world));
                    render_world.insert_resource(RenderState::Reinitializing);
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
