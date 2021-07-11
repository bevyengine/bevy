mod conversion;
mod interaction;
mod presentation;

use ash::{version::InstanceV1_0, vk::Handle};
pub use interaction::*;

use bevy_app::{App, AppBuilder, AppExit, CoreStage, Events, ManualEventReader, Plugin};
use bevy_ecs::schedule::Schedule;
use bevy_xr::{
    presentation::XrGraphicsContext, XrProfiles, XrSessionMode, XrSystem, XrVisibilityState,
};
use openxr::{self as xr, sys};
use presentation::GraphicsContextHandles;
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc, thread, time::Duration};

// The form-factor is selected at plugin-creation-time and cannot be changed anymore for the entire
// lifetime of the app. This will restrict which XrSessionMode can be selected.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum OpenXrFormFactor {
    HeadMountedDisplay,
    Handheld,
}

enum SessionBackend {
    Vulkan(xr::Session<xr::Vulkan>),
    #[cfg(windows)]
    D3D11(xr::Session<xr::D3D11>),
}

enum FrameStream {
    Vulkan(xr::FrameStream<xr::Vulkan>),
    #[cfg(windows)]
    D3D11(xr::FrameStream<xr::D3D11>),
}

pub struct OpenXrSession {
    backend: Option<SessionBackend>,
    frame_stream: Option<FrameStream>,
    frame_waiter: Option<xr::FrameWaiter>,
    _wgpu_device: Arc<wgpu::Device>,
}

impl Drop for OpenXrSession {
    fn drop(&mut self) {
        // Drop OpenXR session objects before wgpu::Device.
        self.backend.take();
        self.frame_stream.take();
        self.frame_waiter.take();
    }
}

#[derive(Debug)]
pub enum OpenXrError {
    Loader(xr::LoadError),
    InstanceCreation(sys::Result),
    UnsupportedFormFactor,
    UnavailableFormFactor,
    GraphicsCreation(Box<dyn Error>),
}

fn selected_extensions(entry: &xr::Entry) -> xr::ExtensionSet {
    let available = entry.enumerate_extensions().unwrap();

    let mut exts = xr::ExtensionSet::default();
    // Complete list: https://www.khronos.org/registry/OpenXR/specs/1.0/html/xrspec.html#extension-appendices-list
    exts.khr_composition_layer_depth = available.khr_composition_layer_depth;
    // todo: set depth layer
    exts.khr_vulkan_enable = available.khr_vulkan_enable;
    exts.khr_vulkan_enable2 = available.khr_vulkan_enable2;
    if cfg!(debug_assertions) {
        exts.ext_debug_utils = available.ext_debug_utils;
    }
    exts.ext_eye_gaze_interaction = available.ext_eye_gaze_interaction;
    // todo: implement eye tracking
    exts.ext_hand_tracking = available.ext_hand_tracking;
    // todo: implement hand tracking
    exts.ext_hp_mixed_reality_controller = available.ext_hp_mixed_reality_controller;
    exts.ext_performance_settings = available.ext_performance_settings;
    // todo: implement performance API
    exts.ext_samsung_odyssey_controller = available.ext_samsung_odyssey_controller;
    exts.ext_thermal_query = available.ext_thermal_query;
    // todo: implement thermal API
    exts.fb_color_space = available.fb_color_space;
    // todo: implement color space API
    exts.fb_display_refresh_rate = available.fb_display_refresh_rate;
    // todo: implement refresh rate API
    exts.htc_vive_cosmos_controller_interaction = available.htc_vive_cosmos_controller_interaction;
    exts.huawei_controller_interaction = available.huawei_controller_interaction;
    exts.msft_hand_interaction = available.msft_hand_interaction;
    // exts.msft_scene_unserstanding = available.msft_scene_unserstanding -> not available in openxrs
    // todo: implement scene understanding API
    // exts.msft_scene_unserstanding_serialization = available.msft_scene_unserstanding_serialization -> not available in openxrs
    // todo: implement scene serialization
    exts.msft_secondary_view_configuration = available.msft_secondary_view_configuration;
    // todo: implement secondary view. This requires integration with winit.
    exts.msft_spatial_anchor = available.msft_spatial_anchor;
    // todo: implement spatial anchors API
    exts.varjo_quad_views = available.varjo_quad_views;

    #[cfg(target_os = "android")]
    {
        exts.khr_android_create_instance = available.khr_android_create_instance;
        exts.khr_android_thread_settings = available.khr_android_thread_settings;
        // todo: set APPLICATION_MAIN and RENDER_MAIN threads
    }
    #[cfg(windows)]
    {
        exts.khr_d3d11_enable = available.khr_d3d11_enable;
    }

    exts
}

pub struct OpenXrContext {
    instance: xr::Instance,
    form_factor: xr::FormFactor,
    system: xr::SystemId,
    // Note: the lifecycle of graphics handles is managed by wgpu objects
    graphics_handles: GraphicsContextHandles,
    wgpu_device: Arc<wgpu::Device>,
    graphics_context: Option<XrGraphicsContext>,
}

impl OpenXrContext {
    fn new(form_factor: OpenXrFormFactor) -> Result<Self, OpenXrError> {
        let entry = xr::Entry::load().map_err(OpenXrError::Loader)?;

        #[cfg(target_os = "android")]
        entry.initialize_android_loader();

        let extensions = selected_extensions(&entry);

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Bevy App",
                    application_version: 0,
                    engine_name: "Bevy Engine",
                    engine_version: 0,
                },
                &extensions,
                &[], // todo: add debug layer
            )
            .map_err(OpenXrError::InstanceCreation)?;

        let form_factor = match form_factor {
            OpenXrFormFactor::HeadMountedDisplay => xr::FormFactor::HEAD_MOUNTED_DISPLAY,
            OpenXrFormFactor::Handheld => xr::FormFactor::HEAD_MOUNTED_DISPLAY,
        };

        let system = instance.system(form_factor).map_err(|e| match e {
            sys::Result::ERROR_FORM_FACTOR_UNSUPPORTED => OpenXrError::UnsupportedFormFactor,
            sys::Result::ERROR_FORM_FACTOR_UNAVAILABLE => OpenXrError::UnavailableFormFactor,
            e => panic!("{}", e), // should never happen
        })?;

        let (graphics_handles, graphics_context) =
            presentation::create_graphics_context(&instance, system)
                .map_err(OpenXrError::GraphicsCreation)?;

        Ok(Self {
            instance,
            form_factor,
            system,
            graphics_handles,
            wgpu_device: graphics_context.device.clone(),
            graphics_context: Some(graphics_context),
        })
    }

    pub fn instance(&self) -> &xr::Instance {
        &self.instance
    }
}

fn get_system_info(
    instance: &xr::Instance,
    system: xr::SystemId,
    mode: XrSessionMode,
) -> Option<(xr::ViewConfigurationType, xr::EnvironmentBlendMode)> {
    let view_type = match mode {
        XrSessionMode::ImmersiveVR | XrSessionMode::ImmersiveAR => {
            if instance.exts().varjo_quad_views.is_some() {
                xr::ViewConfigurationType::PRIMARY_QUAD_VARJO
            } else {
                xr::ViewConfigurationType::PRIMARY_STEREO
            }
        }
        XrSessionMode::InlineVR | XrSessionMode::InlineAR => {
            xr::ViewConfigurationType::PRIMARY_MONO
        }
    };

    let blend_modes = instance
        .enumerate_environment_blend_modes(system, view_type)
        .unwrap();

    let blend_mode = match mode {
        XrSessionMode::ImmersiveVR | XrSessionMode::InlineVR => blend_modes
            .into_iter()
            .find(|b| *b == xr::EnvironmentBlendMode::OPAQUE)?,
        XrSessionMode::ImmersiveAR | XrSessionMode::InlineAR => blend_modes
            .iter()
            .cloned()
            .find(|b| *b == xr::EnvironmentBlendMode::ALPHA_BLEND)
            .or_else(|| {
                blend_modes
                    .into_iter()
                    .find(|b| *b == xr::EnvironmentBlendMode::ADDITIVE)
            })?,
    };

    Some((view_type, blend_mode))
}

#[derive(Default)]
pub struct OpenXrPlugin;

impl Plugin for OpenXrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        if !app.world().contains_resource::<OpenXrContext>() {
            let context =
                OpenXrContext::new(OpenXrFormFactor::HeadMountedDisplay).unwrap_or_else(|_| {
                    match OpenXrContext::new(OpenXrFormFactor::Handheld) {
                        Ok(context) => context,
                        // In case OpenXR is suported, there should be always at least one supported
                        // form factor. If "Handheld" is unsupported, "HeadMountedDisplay" is
                        // supported (but in this case unavailable).
                        Err(
                            OpenXrError::UnsupportedFormFactor | OpenXrError::UnavailableFormFactor,
                        ) => panic!(
                            "OpenXR: No available form factors. Consider manually handling {}",
                            "the creation of the OpenXrContext resource."
                        ),
                        Err(e) => panic!(
                            "OpenXR: Failed to create OpenXrContext: {:?}\n{} {}",
                            e,
                            "Consider manually handling",
                            "the creation of the OpenXrContext resource."
                        ),
                    }
                });
            app.world_mut().insert_resource(context);
        }

        let context = app.world().get_resource_mut::<OpenXrContext>().unwrap();

        let bindings = app
            .world()
            .get_resource::<OpenXrBindings>()
            .cloned()
            .unwrap_or_default();

        app
            // .insert_resource(Arc::new(OpenXrInteractionContext::new(
            //     &context.instance,
            //     bindings,
            // )))
            .insert_resource::<XrGraphicsContext>(context.graphics_context.take().unwrap())
            // .add_system_to_stage(CoreStage::PreUpdate, interaction::handl_input)
            // .add_system_to_stage(CoreStage::PostUpdate, interaction::output_system)
            // .insert_resource(XrSystem::new(Box::new(System {
            //     instance: context.instance,
            // })))
            .set_runner(runner);
    }
}

// Currently, only the session loop is implemented. If the session is destroyed or fails to
// create, the app will exit.
// todo: Implement the instance loop when the the lifecycle API is implemented.
fn runner(mut app: App) {
    let context = app.world.remove_resource::<OpenXrContext>().unwrap();
    let instance = context.instance;
    let system = context.system;
    let graphics_handles = context.graphics_handles;

    let mut app_exit_event_reader = ManualEventReader::<AppExit>::default();

    // Find the available session modes
    let available_session_modes = [
        XrSessionMode::ImmersiveVR,
        XrSessionMode::ImmersiveAR,
        XrSessionMode::InlineVR,
        XrSessionMode::InlineAR,
    ]
    .iter()
    .filter_map(|mode| get_system_info(&instance, system, *mode).map(|_| *mode))
    .collect();

    app.world
        .insert_resource(XrSystem::new(available_session_modes));

    // Run the startup systems. The user can verify which session modes are supported and choose
    // one.
    app.schedule
        .get_stage_mut::<Schedule>(&CoreStage::Startup)
        .unwrap()
        .run_once(&mut app.world);

    let mode = app
        .world
        .get_resource::<XrSystem>()
        .unwrap()
        .selected_session_mode();

    // Remove XrSystem. The user cannot make any more changes to the session mode.
    // todo: when the lifecycle API is implemented, allow the user to change the session mode at any
    // moment.
    app.world.remove_resource::<XrSystem>();

    let (view_type, blend_mode) = get_system_info(&instance, system, mode).unwrap();

    let (session_backend, mut frame_waiter, frame_stream) = match graphics_handles {
        GraphicsContextHandles::Vulkan {
            instance: vk_instance,
            physical_device,
            device,
            queue_family_index,
            queue_index,
        } => {
            let (session, frame_waiter, frame_stream) = unsafe {
                instance
                    .create_session(
                        system,
                        &xr::vulkan::SessionCreateInfo {
                            instance: vk_instance.handle().as_raw() as *const _,
                            physical_device: physical_device.as_raw() as *const _,
                            device: device.handle().as_raw() as *const _,
                            queue_family_index,
                            queue_index,
                        },
                    )
                    .unwrap()
            };
            (
                SessionBackend::Vulkan(session),
                frame_waiter,
                FrameStream::Vulkan(frame_stream),
            )
        }
        #[cfg(windows)]
        GraphicsContextHandles::D3D11 { device } => {
            let (session, frame_waiter, frame_stream) = self
                .instance
                .create_session(
                    self.system_id,
                    &xr::d3d::SessionCreateInfo {
                        device: device as _,
                    },
                )
                .unwrap();
            (
                SessionBackend::D3D11(session),
                frame_waiter,
                FrameStream::D3D11(frame_stream),
            )
        }
    };

    let session = match &session_backend {
        SessionBackend::Vulkan(backend) => backend.clone().into_any_graphics(),
        #[cfg(windows)]
        SessionBackend::D3D11(backend) => backend.clone().into_any_graphics(),
    };

    let mut event_storage = xr::EventDataBuffer::new();

    let mut running = false;
    'session_loop: loop {
        while let Some(event) = instance.poll_event(&mut event_storage).unwrap() {
            match event {
                xr::Event::EventsLost(e) => {
                    bevy_log::error!("OpenXR: Lost {} events", e.lost_event_count());
                }
                xr::Event::InstanceLossPending(_) => {
                    bevy_log::info!("OpenXR: Shutting down for runtime request");
                    break 'session_loop;
                }
                xr::Event::SessionStateChanged(e) => {
                    bevy_log::debug!("entered state {:?}", e.state());

                    match e.state() {
                        xr::SessionState::UNKNOWN | xr::SessionState::IDLE => (),
                        xr::SessionState::READY => {
                            session.begin(view_type).unwrap();
                            running = true;
                        }
                        xr::SessionState::SYNCHRONIZED => {
                            app.world.insert_resource(XrVisibilityState::Hidden)
                        }
                        xr::SessionState::VISIBLE => {
                            app.world.insert_resource(XrVisibilityState::Visible)
                        }
                        xr::SessionState::FOCUSED => {
                            app.world.insert_resource(XrVisibilityState::Focused)
                        }
                        xr::SessionState::STOPPING => {
                            session.end().unwrap();
                            running = false;
                        }
                        xr::SessionState::EXITING | xr::SessionState::LOSS_PENDING => {
                            break 'session_loop;
                        }
                        _ => unreachable!(),
                    }
                }
                xr::Event::ReferenceSpaceChangePending(_) => {
                    // todo: handle recentering event
                }
                xr::Event::PerfSettingsEXT(e) => {
                    let sub_domain = match e.sub_domain() {
                        xr::PerfSettingsSubDomainEXT::COMPOSITING => "compositing",
                        xr::PerfSettingsSubDomainEXT::RENDERING => "rendering",
                        xr::PerfSettingsSubDomainEXT::THERMAL => "thermal",
                        _ => unreachable!(),
                    };
                    let domain = match e.domain() {
                        xr::PerfSettingsDomainEXT::CPU => "CPU",
                        xr::PerfSettingsDomainEXT::GPU => "GPU",
                        _ => unreachable!(),
                    };
                    let from = match e.from_level() {
                        xr::PerfSettingsNotificationLevelEXT::NORMAL => "normal",
                        xr::PerfSettingsNotificationLevelEXT::WARNING => "warning",
                        xr::PerfSettingsNotificationLevelEXT::IMPAIRED => "critical",
                        _ => unreachable!(),
                    };
                    let to = match e.to_level() {
                        xr::PerfSettingsNotificationLevelEXT::NORMAL => "normal",
                        xr::PerfSettingsNotificationLevelEXT::WARNING => "warning",
                        xr::PerfSettingsNotificationLevelEXT::IMPAIRED => "critical",
                        _ => unreachable!(),
                    };
                    bevy_log::warn!(
                        "OpenXR: The {} state of the {} went from {} to {}",
                        sub_domain,
                        domain,
                        from,
                        to
                    );

                    // todo: react to performance notifications
                }
                xr::Event::VisibilityMaskChangedKHR(_) => (), // todo: update visibility mask
                xr::Event::InteractionProfileChanged(_) => {
                    let left_hand = instance
                        .path_to_string(
                            session
                                .current_interaction_profile(
                                    instance.string_to_path("/user/hand/left").unwrap(),
                                )
                                .unwrap(),
                        )
                        .ok();
                    let right_hand = instance
                        .path_to_string(
                            session
                                .current_interaction_profile(
                                    instance.string_to_path("/user/hand/right").unwrap(),
                                )
                                .unwrap(),
                        )
                        .ok();

                    app.world.insert_resource(XrProfiles {
                        left_hand,
                        right_hand,
                    })
                }
                xr::Event::MainSessionVisibilityChangedEXTX(_) => (), // unused
                xr::Event::DisplayRefreshRateChangedFB(_) => (),      // shouldn't be needed
                _ => {
                    bevy_log::debug!("OpenXR: Unhandled event")
                }
            }
        }

        if !running {
            thread::sleep(Duration::from_millis(200));
            continue;
        }

        let frame_state = frame_waiter.wait().unwrap();

        // app.world
        //     .get_resource_mut::<OpenXrTrackingState>()
        //     .unwrap()
        //     .next_vsync_time = frame_state.predicted_display_time;

        app.update();

        if let Some(app_exit_events) = app.world.get_resource_mut::<Events<AppExit>>() {
            if app_exit_event_reader
                .iter(&app_exit_events)
                .next_back()
                .is_some()
            {
                match session.request_exit() {
                    Ok(()) => (),
                    Err(xr::sys::Result::ERROR_SESSION_NOT_RUNNING) => break,
                    Err(e) => panic!("{}", e),
                }
            }
        }
    }
}
