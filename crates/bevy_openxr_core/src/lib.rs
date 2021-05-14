use once_cell::sync::OnceCell;

use bevy_app::prelude::*;
use bevy_ecs::IntoSystem;

mod swapchain;
pub use swapchain::*;

mod device;
pub use device::*;

mod systems;
use systems::*;

mod view_transform;
pub use view_transform::*;

static mut WGPU_INSTANCE: OnceCell<WgpuData> = once_cell::sync::OnceCell::new();

struct WgpuData((wgpu::wgpu_openxr::WGPUOpenXR, openxr::Instance));

impl std::fmt::Debug for WgpuData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "WgpuData[]")
    }
}

pub fn set_openxr(wgpu_openxr: wgpu::wgpu_openxr::WGPUOpenXR, openxr_instance: openxr::Instance) {
    unsafe {
        WGPU_INSTANCE
            .set(WgpuData((wgpu_openxr, openxr_instance)))
            .unwrap()
    };
}

#[derive(Default)]
pub struct OpenXRCorePlugin;

impl Plugin for OpenXRCorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        let (wgpu_openxr, openxr_instance) = unsafe { WGPU_INSTANCE.take().unwrap() }.0;

        let options = OpenXROptions::default(); // FIXME user configurable
        println!("XR OPTIONS {:?}", options);

        let openxr_builder = OpenXRStructBuilder::new()
            .set_instance(openxr_instance)
            .set_wgpu_openxr(wgpu_openxr)
            .set_options(options);

        let openxr_inner = openxr_builder.build();

        let xr_device = XRDevice {
            inner: Some(openxr_inner),
            swapchain: None,
        };

        app.insert_resource(xr_device)
            .add_system(openxr_event_system.system())
            .add_event::<XRViewConfigurationEvent>()
            .add_event::<XRState>()
            .init_resource::<HandPoseState>()
            .add_system(xr_event_debug.system())
            .set_runner(xr_runner); // FIXME conditional, or extract xr_events to whole new system? probably good
    }
}

#[derive(Clone, Debug)]
pub struct OpenXROptions {
    pub view_type: openxr::ViewConfigurationType,
    pub hand_trackers: bool,
}

impl Default for OpenXROptions {
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

pub struct OpenXRStructBuilder {
    instance: Option<openxr::Instance>,
    options: Option<OpenXROptions>,
    wgpu_openxr: Option<wgpu::wgpu_openxr::WGPUOpenXR>,
}

impl OpenXRStructBuilder {
    pub fn new() -> Self {
        OpenXRStructBuilder {
            instance: None,
            options: None,
            wgpu_openxr: None,
        }
    }

    pub fn set_instance(mut self, instance: openxr::Instance) -> Self {
        self.instance = Some(instance);
        self
    }

    pub fn set_options(mut self, options: OpenXROptions) -> Self {
        self.options = Some(options);
        self
    }

    pub fn set_wgpu_openxr(mut self, wgpu_openxr: wgpu::wgpu_openxr::WGPUOpenXR) -> Self {
        self.wgpu_openxr = Some(wgpu_openxr);
        self
    }

    pub fn build(mut self) -> OpenXRStruct {
        let handles = self
            .wgpu_openxr
            .take()
            .unwrap()
            .get_session_handles()
            .unwrap();

        OpenXRStruct::new(
            self.instance.take().unwrap(),
            handles,
            self.options.take().unwrap(),
        )
    }
}

pub struct OpenXRStruct {
    event_storage: EventDataBufferHolder,
    session_state: XRState,
    previous_frame_state: XRState,
    pub handles: wgpu::OpenXRHandles,
    pub instance: openxr::Instance,
    pub options: OpenXROptions,
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
        options: OpenXROptions,
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

fn xr_runner(mut app: App) {
    let mut frame = 0;
    loop {
        let start = std::time::Instant::now();
        app.update();

        if frame % 70 == 0 {
            let took = start.elapsed();
            let fps = 1000.0 / took.as_millis() as f32;
            println!("Frame {} took {:?} ({} fps)", frame, took, fps);
        }

        frame += 1;
    }
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
