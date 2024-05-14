#[cfg(target_os = "android")]
pub use winit::platform::android::activity as android_activity;

#[cfg(target_os = "android")]
use bevy_window::{PrimaryWindow, RawHandleWrapper};

use crate::UpdateMode;

/// [`AndroidApp`] provides an interface to query the application state as well as monitor events
/// (for example lifecycle and input events).
#[cfg(target_os = "android")]
pub static ANDROID_APP: std::sync::OnceLock<android_activity::AndroidApp> =
    std::sync::OnceLock::new();

#[derive(PartialEq, Eq, Debug)]
pub(crate) enum UpdateState {
    NotYetStarted,
    Active,
    Suspended,
    WillSuspend,
    WillResume,
}

impl UpdateState {
    #[inline]
    pub(crate) fn is_active(&self) -> bool {
        match self {
            Self::NotYetStarted | Self::Suspended => false,
            Self::Active | Self::WillSuspend | Self::WillResume => true,
        }
    }
}

/// Persistent state that is used to run the [`App`] according to the current
/// [`UpdateMode`].
pub(crate) struct WinitAppRunnerState {
    /// Current activity state of the app.
    pub(crate) activity_state: UpdateState,
    /// Current update mode of the app.
    pub(crate) update_mode: UpdateMode,
    /// Is `true` if a new [`WindowEvent`] has been received since the last update.
    pub(crate) window_event_received: bool,
    /// Is `true` if a new [`DeviceEvent`] has been received since the last update.
    pub(crate) device_event_received: bool,
    /// Is `true` if the app has requested a redraw since the last update.
    pub(crate) redraw_requested: bool,
    /// Is `true` if enough time has elapsed since `last_update` to run another update.
    pub(crate) wait_elapsed: bool,
    /// Number of "forced" updates to trigger on application start
    pub(crate) startup_forced_updates: u32,
}

impl WinitAppRunnerState {
    pub(crate) fn reset_on_update(&mut self) {
        self.window_event_received = false;
        self.device_event_received = false;
    }
}

impl Default for WinitAppRunnerState {
    fn default() -> Self {
        Self {
            activity_state: UpdateState::NotYetStarted,
            update_mode: UpdateMode::Continuous,
            window_event_received: false,
            device_event_received: false,
            redraw_requested: false,
            wait_elapsed: false,
            // 3 seems to be enough, 5 is a safe margin
            startup_forced_updates: 5,
        }
    }
}
