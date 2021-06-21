mod conversion;
pub mod interaction;
mod presentation;

use bevy_app::{AppBuilder, CoreStage, Plugin};
use bevy_ecs::prelude::IntoSystem;
use bevy_utils::Duration;
use bevy_xr::{presentation::XrPresentationContext, XrMode};
use interaction::{OpenXrBindings, OpenXrInteractionContext, Spaces};
use openxr as xr;
use openxr::sys;

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct OpenXrTime(Duration);

impl OpenXrTime {
    pub fn from_nanos(nanos: u64) -> Self {
        Self(Duration::from_nanos(nanos))
    }

    pub fn as_nanos(self) -> u64 {
        self.0.as_nanos() as _
    }
}

impl std::ops::Add<Duration> for OpenXrTime {
    type Output = OpenXrTime;

    fn add(self, rhs: Duration) -> OpenXrTime {
        OpenXrTime(self.0 + rhs)
    }
}

pub(crate) enum SessionBackend {
    Vulkan(xr::Session<xr::Vulkan>),
    #[cfg(windows)]
    D3D11(xr::Session<xr::D3D11>),
}

impl SessionBackend {
    pub fn state<T: xr::ActionTy + xr::ActionInput>(
        &self,
        action: &xr::Action<T>,
    ) -> xr::Result<xr::ActionState<T>> {
        match self {
            SessionBackend::Vulkan(backend) => action.state(backend, xr::Path::NULL),
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => action.state(backend, xr::Path::NULL),
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
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => {
                action.apply_feedback(backend, xr::Path::NULL, haptic)
            }
        }
    }

    pub fn stop_feedback(&self, action: &xr::Action<xr::Haptic>) -> xr::Result<()> {
        match self {
            SessionBackend::Vulkan(backend) => action.stop_feedback(backend, xr::Path::NULL),
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => action.stop_feedback(backend, xr::Path::NULL),
        }
    }

    pub fn create_space(&self, action: &xr::Action<xr::Posef>) -> xr::Result<xr::Space> {
        match self {
            SessionBackend::Vulkan(backend) => {
                action.create_space(backend.clone(), xr::Path::NULL, xr::Posef::IDENTITY)
            }
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => {
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
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => {
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
            #[cfg(windows)]
            SessionBackend::D3D11(backend) => {
                backend.locate_views(view_configuration_type, display_time, space)
            }
        }
    }
}

enum FrameStream {
    Vulkan(xr::FrameStream<xr::Vulkan>),
    #[cfg(windows)]
    D3D11(xr::FrameStream<xr::D3D11>),
}

// Note: this is not a simple wrapper of xr::Session, instead it is a wrapper for every object
// which depends directly or indirectly on graphics initialization.
struct OpenXrSession {
    pub(crate) backend: SessionBackend,
    frame_stream: FrameStream,
    frame_waiter: xr::FrameWaiter,
    frame_state: xr::FrameState,
    spaces: Spaces,
}

#[derive(Debug)]
pub enum OpenXrError {
    Loader(xr::LoadError),
    InstanceCreation(sys::Result),
    UnsupportedMode,
}

#[derive(Clone)]
pub struct OpenXrContext {
    instance: xr::Instance,
    system_id: xr::SystemId,
    view_type: xr::ViewConfigurationType,
    blend_mode: xr::EnvironmentBlendMode,
    mode: XrMode,
}

impl OpenXrContext {
    fn new(mode: XrMode) -> Result<Self, OpenXrError> {
        let entry = xr::Entry::load().map_err(OpenXrError::Loader)?;

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
            .map_err(OpenXrError::InstanceCreation)?;

        let form_factor = match mode {
            XrMode::ImmersiveVR | XrMode::ImmersiveAR => xr::FormFactor::HEAD_MOUNTED_DISPLAY,
            XrMode::InlineVR | XrMode::InlineAR => xr::FormFactor::HANDHELD_DISPLAY,
        };

        let system_id = instance
            .system(form_factor)
            .map_err(|_| OpenXrError::UnsupportedMode)?;

        let view_type = if form_factor == xr::FormFactor::HANDHELD_DISPLAY {
            xr::ViewConfigurationType::PRIMARY_MONO
        } else {
            // todo: detect headsets with more views (Varjo)
            xr::ViewConfigurationType::PRIMARY_STEREO
        };

        let blend_modes = instance
            .enumerate_environment_blend_modes(system_id, view_type)
            .unwrap();

        let blend_mode = blend_modes
            .iter()
            .find(|m| match mode {
                XrMode::ImmersiveVR | XrMode::InlineVR => **m == xr::EnvironmentBlendMode::OPAQUE,
                XrMode::ImmersiveAR | XrMode::InlineAR => {
                    **m == xr::EnvironmentBlendMode::ADDITIVE
                        || **m == xr::EnvironmentBlendMode::ALPHA_BLEND
                }
            })
            .cloned()
            .ok_or(OpenXrError::UnsupportedMode)?;

        Ok(Self {
            instance,
            system_id,
            view_type,
            blend_mode,
            mode,
        })
    }
}

#[derive(Default)]
pub struct OpenXrPlugin;

impl Plugin for OpenXrPlugin {
    fn build(&self, app: &mut AppBuilder) {
        let context = if let Some(context) = app.world().get_resource::<OpenXrContext>() {
            context.clone()
        } else {
            let maybe_mode = app.world().get_resource::<XrMode>().cloned();
            let mode = maybe_mode.unwrap_or(XrMode::ImmersiveVR);

            let context = match OpenXrContext::new(mode) {
                Ok(context) => context,
                Err(OpenXrError::UnsupportedMode) => xr_mode_fallback(mode)
                    .iter()
                    .find_map(|mode| OpenXrContext::new(*mode).ok())
                    .unwrap(),
                Err(e) => panic!("Failed to initialize OpenXR: {:?}", e),
            };

            if let Some(mode) = maybe_mode {
                if context.mode != mode {
                    bevy_log::warn!("XrMode has been changed to {:?}", mode);
                }
            }

            context
        };

        let bindings = app
            .world()
            .get_resource::<OpenXrBindings>()
            .cloned()
            .unwrap_or_default();

        app.insert_resource(OpenXrInteractionContext::new(&context.instance, bindings))
            .insert_resource(context.mode)
            .insert_resource::<Box<dyn XrPresentationContext>>(Box::new(context));

        app.add_system_to_stage(CoreStage::PreUpdate, interaction::input_system.system())
            .add_system_to_stage(CoreStage::PostUpdate, interaction::output_system.system());
    }
}

fn xr_mode_fallback(mode: XrMode) -> &'static [XrMode] {
    match mode {
        XrMode::ImmersiveVR => &[XrMode::ImmersiveAR, XrMode::InlineVR, XrMode::InlineAR],
        XrMode::ImmersiveAR | XrMode::InlineVR => {
            &[XrMode::InlineAR, XrMode::ImmersiveVR, XrMode::ImmersiveAR]
        }
        XrMode::InlineAR => &[XrMode::ImmersiveAR, XrMode::InlineVR, XrMode::ImmersiveVR],
    }
}
