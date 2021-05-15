pub(crate) enum XREvent {
    ViewCreated(XRViewCreated),
}

/// Current state of XR hardware/session
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum XRState {
    Paused,
    Running,
    RunningFocused,
    Exiting,
}

/// XR View has been configured/created
pub struct XRViewCreated {
    pub width: u32,
    pub height: u32,
}
