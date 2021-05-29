pub mod interaction;
pub mod presentation;

pub use interaction::*;
pub use presentation::XrVisibilityState;

use bevy_app::{App, Plugin};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum XrSessionMode {
    ImmersiveVR,
    ImmersiveAR,
    InlineVR,
    InlineAR,
}

pub struct XrSystem {
    available_session_modes: Vec<XrSessionMode>,
    session_mode: XrSessionMode,
    action_set_desc: Vec<XrProfileDescriptor>,
}

impl XrSystem {
    pub fn new(available_session_modes: Vec<XrSessionMode>) -> Self {
        Self {
            session_mode: available_session_modes[0],
            available_session_modes,
            action_set_desc: vec![],
        }
    }

    pub fn selected_session_mode(&self) -> XrSessionMode {
        self.session_mode
    }

    pub fn available_session_modes(&self) -> Vec<XrSessionMode> {
        self.available_session_modes.clone()
    }

    /// In case this method returns false, it may be either because the mode is not supported or
    /// currently not available.
    pub fn is_session_mode_supported(&self, mode: XrSessionMode) -> bool {
        self.available_session_modes.contains(&mode)
    }

    /// Set session mode. Returns false if the mode is unsupported.
    pub fn request_session_mode(&mut self, mode: XrSessionMode) -> bool {
        if self.is_session_mode_supported(mode) {
            self.session_mode = mode;

            true
        } else {
            false
        }
    }

    pub fn set_action_set(&mut self, action_set_desc: Vec<XrProfileDescriptor>) {
        self.action_set_desc = action_set_desc;
    }

    pub fn action_set(&self) -> &[XrProfileDescriptor] {
        &self.action_set_desc
    }
}

#[derive(Default)]
pub struct XrPlugin;

impl Plugin for XrPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<XrVibrationEvent>()
            .init_resource::<XrProfiles>();
    }
}
