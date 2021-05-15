use bevy_app::{prelude::*, EventReader};
use bevy_ecs::system::IntoSystem;

mod device;
pub mod hand_tracking;
mod runner;
mod swapchain;
mod systems;
mod view_transform;
mod xr_instance;

pub use device::*;
pub use swapchain::*;
use systems::*;
pub use view_transform::*;
pub use xr_instance::{set_xr_instance, XrInstance};

#[derive(Default)]
pub struct OpenXRCorePlugin;

impl Plugin for OpenXRCorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let xr_instance = xr_instance::take_xr_instance();
        let options = XrOptions::default(); // FIXME user configurable?
        let xr_device = xr_instance.into_device_with_options(options);

        app.insert_resource(xr_device)
            .add_system(openxr_event_system.system())
            .add_event::<XRViewConfigurationEvent>()
            .add_event::<XRState>()
            .init_resource::<hand_tracking::HandPoseState>()
            .add_system(xr_event_debug.system())
            .set_runner(runner::xr_runner); // FIXME conditional, or extract xr_events to whole new system? probably good
    }
}

#[derive(Clone, Debug)]
pub struct XrOptions {
    pub view_type: openxr::ViewConfigurationType,
    pub hand_trackers: bool,
}

impl Default for XrOptions {
    fn default() -> Self {
        #[cfg(target_os = "android")]
        let hand_trackers = true;

        #[cfg(not(target_os = "android"))]
        let hand_trackers = false;

        Self {
            view_type: openxr::ViewConfigurationType::PRIMARY_STEREO,
            hand_trackers,
        }
    }
}

// TODO: proposal to rename into `XRInstance`
pub struct OpenXRStruct {
    event_storage: EventDataBufferHolder,
    session_state: XRState,
    previous_frame_state: XRState,
    pub handles: wgpu::OpenXRHandles,
    pub instance: openxr::Instance,
    pub options: XrOptions,
}

impl std::fmt::Debug for OpenXRStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OpenXRStruct[...]")
    }
}

impl OpenXRStruct {
    pub fn new(
        instance: openxr::Instance,
        handles: wgpu::OpenXRHandles,
        options: XrOptions,
    ) -> Self {
        OpenXRStruct {
            event_storage: EventDataBufferHolder(openxr::EventDataBuffer::new()),
            session_state: XRState::Paused,
            previous_frame_state: XRState::Paused,
            instance,
            handles,
            options,
        }
    }

    fn change_state(&mut self, state: XRState, state_flag: &mut bool) -> bool {
        if self.session_state != state {
            self.previous_frame_state = self.session_state;
            self.session_state = state;
            *state_flag = true;
            true
        } else {
            false
        }
    }

    fn get_changed_state(&self, state_flag: &bool) -> Option<XRState> {
        if *state_flag {
            Some(self.session_state)
        } else {
            None
        }
    }

    pub fn handle_openxr_events(&mut self) -> Option<XRState> {
        let mut state_changed = false;

        while let Some(event) = self.instance.poll_event(&mut self.event_storage.0).unwrap() {
            match event {
                openxr::Event::SessionStateChanged(e) => {
                    println!("entered state {:?}", e.state());

                    match e.state() {
                        // XR Docs: The application is ready to call xrBeginSession and sync its frame loop with the runtime.
                        openxr::SessionState::READY => {
                            self.handles.session.begin(self.options.view_type).unwrap();
                            self.change_state(XRState::Running, &mut state_changed);
                        }
                        // XR Docs: The application should exit its frame loop and call xrEndSession.
                        openxr::SessionState::STOPPING => {
                            self.handles.session.end().unwrap();
                            self.change_state(XRState::Paused, &mut state_changed);
                        }
                        // XR Docs:
                        // EXITING: The application should end its XR experience and not automatically restart it.
                        // LOSS_PENDING: The session is in the process of being lost. The application should destroy the current session and can optionally recreate it.
                        openxr::SessionState::EXITING | openxr::SessionState::LOSS_PENDING => {
                            self.change_state(XRState::Exiting, &mut state_changed);
                            return self.get_changed_state(&state_changed);
                        }
                        // XR Docs: The application has synced its frame loop with the runtime and is visible to the user but cannot receive XR input.
                        openxr::SessionState::VISIBLE => {
                            self.change_state(XRState::Running, &mut state_changed);
                        }
                        // XR Docs: The application has synced its frame loop with the runtime, is visible to the user and can receive XR input.
                        openxr::SessionState::FOCUSED => {
                            self.change_state(XRState::RunningFocused, &mut state_changed);
                        }
                        // XR Docs: The initial state after calling xrCreateSession or returned to after calling xrEndSession.
                        openxr::SessionState::IDLE => {
                            // FIXME is this handling ok?
                            self.change_state(XRState::Paused, &mut state_changed);
                        }
                        _ => {}
                    }
                }
                openxr::Event::InstanceLossPending(_) => {
                    self.change_state(XRState::Exiting, &mut state_changed);
                    return self.get_changed_state(&state_changed);
                }
                openxr::Event::EventsLost(e) => {
                    println!("lost {} events", e.lost_event_count());
                }
                openxr::Event::ReferenceSpaceChangePending(_) => {
                    println!("OpenXR: Event: ReferenceSpaceChangePending");
                }
                openxr::Event::PerfSettingsEXT(_) => {
                    println!("OpenXR: Event: PerfSettingsEXT");
                }
                openxr::Event::VisibilityMaskChangedKHR(_) => {
                    println!("OpenXR: Event: VisibilityMaskChangedKHR");
                }
                openxr::Event::InteractionProfileChanged(_) => {
                    println!("OpenXR: Event: InteractionProfileChanged");
                }
                openxr::Event::MainSessionVisibilityChangedEXTX(_) => {
                    println!("OpenXR: Event: MainSessionVisibilityChangedEXTX");
                }
                _ => {
                    println!("OpenXR: Event: unknown")
                }
            }
        }

        match self.session_state {
            XRState::Paused => std::thread::sleep(std::time::Duration::from_millis(100)),
            _ => (),
        }

        self.get_changed_state(&state_changed)
    }

    pub fn is_running(&self) -> bool {
        self.session_state == XRState::Running || self.session_state == XRState::RunningFocused
    }
}

pub struct EventDataBufferHolder(openxr::EventDataBuffer);

// FIXME FIXME FIXME UB AND BAD THINGS CAN/WILL HAPPEN. Required by EventDataBuffer
// read openxr docs about whether EventDataBuffer is thread-safe
// or move to resourcesmut?
// FIXME or process events in own thread?
unsafe impl Sync for EventDataBufferHolder {}
unsafe impl Send for EventDataBufferHolder {}

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum XRState {
    Paused,
    Running,
    RunningFocused,
    Exiting,
}

fn xr_event_debug(mut state_events: EventReader<XRState>) {
    for event in state_events.iter() {
        println!("#STATE EVENT: {:#?}", event);
    }
}

pub struct XRViewConfigurationEvent {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub enum Error {
    XR(openxr::sys::Result),
}

impl From<openxr::sys::Result> for Error {
    fn from(e: openxr::sys::Result) -> Self {
        Error::XR(e)
    }
}
