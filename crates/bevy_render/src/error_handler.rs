use alloc::sync::Arc;
use bevy_ecs::{
    resource::Resource,
    world::{Mut, World},
};
use std::sync::Mutex;
use wgpu::ErrorSource;
use wgpu_types::error::ErrorType;

use crate::{
    insert_future_resources,
    render_resource::PipelineCache,
    renderer::{RenderDevice, WgpuWrapper},
    settings::RenderCreation,
    FutureRenderResources, RenderStartup,
};

/// Resource to indicate renderer behavior upon error.
pub enum RenderErrorPolicy {
    /// Pretends nothing happened and continues rendering.
    /// This discards the error after logging it to console.
    Ignore,
    /// Keeps the app alive, but stops rendering further.
    /// This keeps the error state, and will continue polling the [`RenderErrorHandler`]
    /// every frame until some other policy is returned.
    StopRendering,
    /// Attempt renderer recovery with the given [`RenderCreation`].
    Recover(RenderCreation),
}

/// Determines what [`RenderErrorPolicy`] should be used to respond to a given [`RenderError`].
///
/// The handler has access to both the main world and the render world in that order.
/// By the time this is invoked, the error has already been logged. The error is provided
/// for the decision-making reason of how to appropriately respond to it. Not all errors
/// are equally severe: validation errors may be ignored for example, while device lost errors
/// require recovery to continue rendering.
#[derive(Resource)]
pub struct RenderErrorHandler(
    pub for<'a> fn(&'a RenderError, &'a mut World, &'a mut World) -> RenderErrorPolicy,
);

impl RenderErrorHandler {
    fn handle(&self, error: &RenderError, main_world: &mut World, render_world: &mut World) {
        match self.0(error, main_world, render_world) {
            RenderErrorPolicy::Ignore => {
                // Pretend that didn't happen.
                render_world.insert_resource(RenderState::Ready);
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
}

impl Default for RenderErrorHandler {
    fn default() -> Self {
        // This is what we've always done historically,
        // but we could choose a new default once recovery works better.
        Self(|_, _, _| RenderErrorPolicy::Ignore)
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

/// Updates the state machine that handles the renderer and device lifecycle.
/// Polls the [`DeviceErrorHandler`] and fires the [`RenderErrorHandler`] if needed.
///
/// Runs [`crate::RenderStartup`] after every time a [`RenderDevice`] is acquired.
///
/// We need both the main and render world to properly handle errors, so we wedge ourselves into [extract](bevy_app::SubApp::set_extract).
pub(crate) fn update_state(main_world: &mut World, render_world: &mut World) {
    if let Some(error) = render_world.resource::<DeviceErrorHandler>().poll() {
        render_world.insert_resource(RenderState::Errored(error));
    };

    // Remove the render state so we can provide both worlds to the `RenderErrorHandler`.
    let state = render_world.remove_resource::<RenderState>().unwrap();

    match &state {
        RenderState::Initializing => {
            render_world.run_schedule(RenderStartup);
            render_world.insert_resource(RenderState::Ready);
        }
        RenderState::Ready => {
            // all is well
        }
        RenderState::Errored(error) => {
            main_world.resource_scope(|main_world, error_handler: Mut<RenderErrorHandler>| {
                error_handler.handle(error, main_world, render_world);
            });
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

    // Put the state back if we didn't set a new one
    if render_world.get_resource::<RenderState>().is_none() {
        render_world.insert_resource(state);
    }
}
