mod conversion;
pub mod interaction;
mod presentation;

use bevy_app::{AppBuilder, CoreStage, Plugin};
use bevy_ecs::prelude::IntoSystem;
use bevy_xr::{
    implementation::XrStateBackend,
    interaction::{
        HandAction, HandType, Motion, Orientation, Pose, Position, TrackingReferenceMode,
        Vibration, XR_HAND_JOINT_COUNT,
    },
    presentation::XrPresentationResourceContext,
    ViewerType, XrConfig, XrDuration, XrMode, XrTime,
};
use conversion::{from_xr_time, to_quat, to_vec3, to_xr_duration, to_xr_time};
use glam::Vec2;
use interaction::{OpenXrBindingDesc, OpenXrVendorInput, OpenXrVendorOutput, PoseActions, Spaces};
use openxr as xr;
use std::sync::{Arc, Mutex};

pub(crate) enum SessionBackend {
    Vulkan(xr::Session<xr::Vulkan>),
    D3D11(xr::Session<xr::D3D11>),
    OpenGL(xr::Session<xr::OpenGL>),
}

impl SessionBackend {
    pub fn state<T: xr::ActionTy + xr::ActionInput>(
        &self,
        action: &xr::Action<T>,
    ) -> xr::Result<xr::ActionState<T>> {
        match self {
            SessionBackend::Vulkan(backend) => action.state(backend, xr::Path::NULL),
            SessionBackend::D3D11(backend) => action.state(backend, xr::Path::NULL),
            SessionBackend::OpenGL(backend) => action.state(backend, xr::Path::NULL),
        }
    }

    pub fn apply_feedback(
        &self,
        action: &xr::Action<xr::Haptic>,
        haptic: &xr::HapticBase,
    ) -> xr::Result<()> {
        match self {
            SessionBackend::Vulkan(backend) => {
                action.apply_feedback(backend, xr::Path::NULL, haptic)
            }
            SessionBackend::D3D11(backend) => {
                action.apply_feedback(backend, xr::Path::NULL, haptic)
            }
            SessionBackend::OpenGL(backend) => {
                action.apply_feedback(backend, xr::Path::NULL, haptic)
            }
        }
    }

    pub fn stop_feedback(&self, action: &xr::Action<xr::Haptic>) -> xr::Result<()> {
        match self {
            SessionBackend::Vulkan(backend) => action.stop_feedback(backend, xr::Path::NULL),
            SessionBackend::D3D11(backend) => action.stop_feedback(backend, xr::Path::NULL),
            SessionBackend::OpenGL(backend) => action.stop_feedback(backend, xr::Path::NULL),
        }
    }

    pub fn create_space(&self, action: &xr::Action<xr::Posef>) -> xr::Result<xr::Space> {
        match self {
            SessionBackend::Vulkan(backend) => {
                action.create_space(backend.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
            }
            SessionBackend::D3D11(backend) => {
                action.create_space(backend.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
            }
            SessionBackend::OpenGL(backend) => {
                action.create_space(backend.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
            }
        }
    }

    pub fn create_reference_space(
        &self,
        reference_type: xr::ReferenceSpaceType,
    ) -> xr::Result<xr::Space> {
        match self {
            SessionBackend::Vulkan(backend) => {
                backend.create_reference_space(reference_type, xr::Posef::IDENTITY)
            }
            SessionBackend::D3D11(backend) => {
                backend.create_reference_space(reference_type, xr::Posef::IDENTITY)
            }
            SessionBackend::OpenGL(backend) => {
                backend.create_reference_space(reference_type, xr::Posef::IDENTITY)
            }
        }
    }

    pub fn locate_views(
        &self,
        view_configuration_type: xr::ViewConfigurationType,
        display_time: xr::Time,
        space: &xr::Space,
    ) -> xr::Result<(xr::ViewStateFlags, Vec<xr::View>)> {
        match self {
            SessionBackend::Vulkan(backend) => {
                backend.locate_views(view_configuration_type, display_time, space)
            }
            SessionBackend::D3D11(backend) => {
                backend.locate_views(view_configuration_type, display_time, space)
            }
            SessionBackend::OpenGL(backend) => {
                backend.locate_views(view_configuration_type, display_time, space)
            }
        }
    }
}

enum FrameStream {
    Vulkan(xr::FrameStream<xr::Vulkan>),
    D3D11(xr::FrameStream<xr::D3D11>),
    OpenGL(xr::FrameStream<xr::OpenGL>),
}

// Note: this is not a simple wrapper of xr::Session, instead it is a wrapper for every object
// which depends directly or indirectly on graphics initialization.
struct Session {
    pub(crate) backend: SessionBackend,
    frame_stream: FrameStream,
    frame_waiter: xr::FrameWaiter,
    frame_state: xr::FrameState,
    spaces: Spaces,
}

#[derive(Default, Clone)]
pub struct OpenXrConfig {
    pub vendor_bindings: Vec<OpenXrBindingDesc>,
}

pub struct OpenXrResourceContext {
    instance: xr::Instance,
    system_id: xr::SystemId,
    view_type: xr::ViewConfigurationType,
    blend_mode: xr::EnvironmentBlendMode,
    _action_set: xr::ActionSet,
    session: Arc<Mutex<Option<Session>>>,
    pose_actions: PoseActions,
}

impl OpenXrResourceContext {
    fn new(app: &mut AppBuilder, xr_config: &XrConfig, openxr_config: &OpenXrConfig) -> Self {
        let entry = xr::Entry::load().expect("Could not find OpenXR loader");

        let available_extensions = entry.enumerate_extensions().unwrap();

        let mut enabled_extensions = xr::ExtensionSet::default();
        enabled_extensions.khr_vulkan_enable2 = available_extensions.khr_vulkan_enable2;
        // todo: add more extensions

        let instance = entry
            .create_instance(
                &xr::ApplicationInfo {
                    application_name: "Bevy App",
                    application_version: 0,
                    engine_name: "Bevy Engine",
                    engine_version: 0,
                },
                &enabled_extensions,
                &[], // todo: add debug layer
            )
            .unwrap();

        let (system_id, form_factor, blend_mode) = match &xr_config.mode {
            XrMode::Display {
                viewer: display,
                blend,
            } => {
                let form_factors = match display {
                    ViewerType::PreferHeadMounted => [
                        xr::FormFactor::HEAD_MOUNTED_DISPLAY,
                        xr::FormFactor::HANDHELD_DISPLAY,
                    ],
                    ViewerType::PreferHandheld => [
                        xr::FormFactor::HANDHELD_DISPLAY,
                        xr::FormFactor::HEAD_MOUNTED_DISPLAY,
                    ],
                };

                form_factors
                    .iter()
                    .find_map(|form| Some((instance.system(*form).ok()?, *form, blend)))
                    .unwrap()
            }
            XrMode::OnlyTracking => {
                // Note: MND_HEADLESS extension exists
                panic!("bevy_openxr does not support XrMode::OnlyTracking.");
            }
        };

        let view_type = if form_factor == xr::FormFactor::HANDHELD_DISPLAY {
            xr::ViewConfigurationType::PRIMARY_MONO
        } else {
            // todo: detect headsets with more views (Varjo)
            xr::ViewConfigurationType::PRIMARY_STEREO
        };

        let blend_modes = instance
            .enumerate_environment_blend_modes(system_id, view_type)
            .unwrap();
        let blend_mode = match blend_mode {
            bevy_xr::BlendMode::PreferVR => *blend_modes
                .iter()
                .find(|m| **m == xr::EnvironmentBlendMode::OPAQUE)
                .or_else(|| blend_modes.first())
                .unwrap(),
            bevy_xr::BlendMode::AR => *blend_modes
                .iter()
                .find(|m| {
                    **m == xr::EnvironmentBlendMode::ADDITIVE
                        || **m == xr::EnvironmentBlendMode::ALPHA_BLEND
                })
                .unwrap_or_else(|| panic!("The XR device does not support AR mode")),
        };

        let action_set = instance
            .create_action_set("interaction", "XR interactions", 0)
            .unwrap();

        let session = Arc::new(Mutex::new(None));

        let pose_actions = register_actions(
            app,
            xr_config,
            openxr_config,
            &instance,
            &action_set,
            &session,
        );

        Self {
            instance,
            system_id,
            view_type,
            blend_mode,
            _action_set: action_set,
            session,
            pose_actions,
        }
    }
}

fn register_actions(
    app: &mut AppBuilder,
    xr_config: &XrConfig,
    openxr_config: &OpenXrConfig,
    instance: &xr::Instance,
    action_set: &xr::ActionSet,
    session: &Arc<Mutex<Option<Session>>>,
) -> PoseActions {
    // Register bindings
    let mut bindings = interaction::pose_bindings();

    if xr_config.enable_generic_controllers {
        // todo: add virtual controller bindings, which should be loaded from an external resource.
    }

    // This must be set after pose_bindings and controller bindings, to allow the user to override
    // the behavior for certain actions.
    bindings.extend(openxr_config.vendor_bindings.clone());

    // Create actions. Various types of actions must be created all at once with one call per
    // vendor.
    let mut actions = interaction::create_actions(&instance, &action_set, &bindings);

    // Redistribute actions in various containers
    let pose_actions = interaction::extract_pose_actions(&mut actions);

    if xr_config.enable_generic_controllers {
        let (controller_input_actions, controller_output_actions) =
            interaction::extract_controller_actions(&mut actions);

        let input_system = Box::new(interaction::controller_input_system_fn(
            Arc::clone(session),
            controller_input_actions,
        ));
        app.add_system_to_stage(CoreStage::PreUpdate, input_system.system());

        let output_system = Box::new(interaction::controller_output_system_fn(
            Arc::clone(session),
            controller_output_actions,
        ));
        app.add_system_to_stage(CoreStage::PostUpdate, output_system.system());
    }

    if !openxr_config.vendor_bindings.is_empty() {
        app.add_event::<OpenXrVendorInput<bool>>()
            .add_event::<OpenXrVendorInput<f32>>()
            .add_event::<OpenXrVendorInput<Vec2>>()
            .add_event::<OpenXrVendorOutput<Vibration>>();

        let input_system = Box::new(interaction::vendor_input_system_fn(
            Arc::clone(session),
            actions.clone(),
        ));
        app.add_system_to_stage(CoreStage::PreUpdate, input_system.system());

        let output_system = Box::new(interaction::vendor_output_system_fn(
            Arc::clone(session),
            actions,
        ));
        app.add_system_to_stage(CoreStage::PostUpdate, output_system.system());
    }

    pose_actions
}

struct OpenXrState {
    view_type: xr::ViewConfigurationType,
    session: Arc<Mutex<Option<Session>>>,
}

impl XrStateBackend for OpenXrState {
    fn set_tracking_reference_mode(&self, mode: TrackingReferenceMode) -> bool {
        if let Some(session) = &mut *self.session.lock().unwrap() {
            let reference_type = match mode {
                TrackingReferenceMode::Tilted => xr::ReferenceSpaceType::VIEW,
                TrackingReferenceMode::GravityAligned => xr::ReferenceSpaceType::LOCAL,
                TrackingReferenceMode::Stage => xr::ReferenceSpaceType::STAGE,
            };
            if let Ok(space) = session.backend.create_reference_space(reference_type) {
                session.spaces.reference = space;

                true
            } else {
                false
            }
        } else {
            false
        }
    }

    fn views_poses(&self, time: XrTime) -> Vec<Pose> {
        if let Some(session) = &*self.session.lock().unwrap() {
            let (flags, views) = session
                .backend
                .locate_views(
                    self.view_type,
                    from_xr_time(time),
                    &session.spaces.reference,
                )
                .unwrap();

            views
                .into_iter()
                .map(|view| {
                    let position = if flags.contains(xr::ViewStateFlags::POSITION_VALID) {
                        Some(Position {
                            value: to_vec3(view.pose.position),
                            tracked: flags.contains(xr::ViewStateFlags::POSITION_TRACKED),
                        })
                    } else {
                        None
                    };
                    let orientation = if flags.contains(xr::ViewStateFlags::ORIENTATION_VALID) {
                        Some(Orientation {
                            value: to_quat(view.pose.orientation),
                            tracked: flags.contains(xr::ViewStateFlags::ORIENTATION_TRACKED),
                        })
                    } else {
                        None
                    };

                    Pose {
                        position,
                        orientation,
                    }
                })
                .collect()
        } else {
            vec![]
        }
    }

    fn hand_motion(&self, hand_type: HandType, action: HandAction, time: XrTime) -> Motion {
        if let Some(session) = &*self.session.lock().unwrap() {
            let space = match (hand_type, action) {
                (HandType::Left, HandAction::Grip) => &session.spaces.left_grip,
                (HandType::Right, HandAction::Grip) => &session.spaces.right_grip,
                (HandType::Left, HandAction::Aim) => &session.spaces.left_aim,
                (HandType::Right, HandAction::Aim) => &session.spaces.right_aim,
            };

            let (location, velocity) = space
                .relate(&session.spaces.reference, from_xr_time(time))
                .unwrap();

            let position = if location
                .location_flags
                .contains(xr::SpaceLocationFlags::POSITION_VALID)
            {
                Some(Position {
                    value: to_vec3(location.pose.position),
                    tracked: location
                        .location_flags
                        .contains(xr::SpaceLocationFlags::POSITION_TRACKED),
                })
            } else {
                None
            };
            let orientation = if location
                .location_flags
                .contains(xr::SpaceLocationFlags::ORIENTATION_VALID)
            {
                Some(Orientation {
                    value: to_quat(location.pose.orientation),
                    tracked: location
                        .location_flags
                        .contains(xr::SpaceLocationFlags::ORIENTATION_TRACKED),
                })
            } else {
                None
            };
            let linear_velocity = if velocity
                .velocity_flags
                .contains(xr::SpaceVelocityFlags::LINEAR_VALID)
            {
                Some(to_vec3(velocity.linear_velocity))
            } else {
                None
            };
            let angular_velocity = if velocity
                .velocity_flags
                .contains(xr::SpaceVelocityFlags::ANGULAR_VALID)
            {
                Some(to_vec3(velocity.angular_velocity))
            } else {
                None
            };

            Motion {
                pose: Pose {
                    position,
                    orientation,
                },
                linear_velocity,
                angular_velocity,
            }
        } else {
            Motion::default()
        }
    }

    fn hand_skeleton_motion(
        &self,
        hand_type: HandType,
        time: XrTime,
    ) -> [Motion; XR_HAND_JOINT_COUNT] {
        todo!()
    }

    fn generic_tracker_motion(&self, _: usize, _: XrTime) -> Motion {
        // Generic trackers are not supported
        Motion {
            pose: Pose {
                position: None,
                orientation: None,
            },
            linear_velocity: None,
            angular_velocity: None,
        }
    }

    fn predicted_display_time(&self) -> XrTime {
        if let Some(session) = &*self.session.lock().unwrap() {
            to_xr_time(session.frame_state.predicted_display_time)
        } else {
            XrTime::from_nanos(0)
        }
    }

    fn predicted_display_period(&self) -> XrDuration {
        if let Some(session) = &*self.session.lock().unwrap() {
            to_xr_duration(session.frame_state.predicted_display_period)
        } else {
            XrDuration::from_nanos(0)
        }
    }

    fn should_render(&self) -> bool {
        if let Some(session) = &*self.session.lock().unwrap() {
            session.frame_state.should_render
        } else {
            false
        }
    }
}

#[derive(Default)]
pub struct OpenXrPlugin;

impl Plugin for OpenXrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let xr_config = app
            .world()
            .get_resource::<XrConfig>()
            .cloned()
            .unwrap_or_else(|| panic!("You need to add XrConfig resource."));

        let openxr_config = app
            .world()
            .get_resource::<OpenXrConfig>()
            .cloned()
            .unwrap_or_default();

        let context = OpenXrResourceContext::new(app, &xr_config, &openxr_config);

        app.insert_resource::<Box<dyn XrStateBackend>>(Box::new(OpenXrState {
            session: Arc::clone(&context.session),
            view_type: context.view_type,
        }));

        app.insert_resource::<Box<dyn XrPresentationResourceContext>>(Box::new(context));
    }
}
