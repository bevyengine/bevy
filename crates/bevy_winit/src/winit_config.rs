use bevy_ecs::system::Resource;
use bevy_utils::Duration;

/// Settings for the [`WinitPlugin`](super::WinitPlugin).
#[derive(Debug, Resource, Clone)]
pub struct WinitSettings {
    /// Determines how frequently the application can update when it has focus.
    pub focused_mode: UpdateMode,
    /// Determines how frequently the application can update when it's out of focus.
    pub unfocused_mode: UpdateMode,
}

impl WinitSettings {
    /// Default settings for games.
    ///
    /// [`Continuous`](UpdateMode::Continuous) if windows have focus,
    /// [`ReactiveLowPower`](UpdateMode::ReactiveLowPower) otherwise.
    pub fn game() -> Self {
        WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / 60.0)), // 60Hz,
        }
    }

    /// Default settings for desktop applications.
    ///
    /// [`Reactive`](UpdateMode::Reactive) if windows have focus,
    /// [`ReactiveLowPower`](UpdateMode::ReactiveLowPower) otherwise.
    ///
    /// Use the [`EventLoopProxy`](crate::EventLoopProxy) to request a redraw from outside bevy.
    pub fn desktop_app() -> Self {
        WinitSettings {
            focused_mode: UpdateMode::reactive(Duration::from_secs(5)),
            unfocused_mode: UpdateMode::reactive_low_power(Duration::from_secs(60)),
        }
    }

    /// Returns the current [`UpdateMode`].
    ///
    /// **Note:** The output depends on whether the window has focus or not.
    pub fn update_mode(&self, focused: bool) -> UpdateMode {
        match focused {
            true => self.focused_mode,
            false => self.unfocused_mode,
        }
    }
}

impl Default for WinitSettings {
    fn default() -> Self {
        WinitSettings::game()
    }
}

/// Determines how frequently an [`App`](bevy_app::App) should update.
///
/// **Note:** This setting is independent of VSync. VSync is controlled by a window's
/// [`PresentMode`](bevy_window::PresentMode) setting. If an app can update faster than the refresh
/// rate, but VSync is enabled, the update rate will be indirectly limited by the renderer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UpdateMode {
    /// The [`App`](bevy_app::App) will update over and over, as fast as it possibly can, until an
    /// [`AppExit`](bevy_app::AppExit) event appears.
    Continuous,
    /// The [`App`](bevy_app::App) will update in response to the following, until an
    /// [`AppExit`](bevy_app::AppExit) event appears:
    /// - `wait` time has elapsed since the previous update
    /// - a redraw has been requested by [`RequestRedraw`](bevy_window::RequestRedraw)
    /// - new [window](`winit::event::WindowEvent`), [raw input](`winit::event::DeviceEvent`), or custom
    ///     events have appeared
    /// - a redraw has been requested with the [`EventLoopProxy`](crate::EventLoopProxy)
    Reactive {
        /// The approximate time from the start of one update to the next.
        ///
        /// **Note:** This has no upper limit.
        /// The [`App`](bevy_app::App) will wait indefinitely if you set this to [`Duration::MAX`].
        wait: Duration,
        /// Reacts to device events, that will wake up the loop if it's in a wait state
        react_to_device_events: bool,
        /// Reacts to user events, that will wake up the loop if it's in a wait state
        react_to_user_events: bool,
        /// Reacts to window events, that will wake up the loop if it's in a wait state
        react_to_window_events: bool,
    },
}

impl UpdateMode {
    /// Reactive mode, will update the app for any kind of event
    pub fn reactive(wait: Duration) -> Self {
        Self::Reactive {
            wait,
            react_to_device_events: true,
            react_to_user_events: true,
            react_to_window_events: true,
        }
    }

    /// Low power mode
    ///
    /// Unlike [`Reactive`](`UpdateMode::reactive()`), this will ignore events that
    /// don't come from interacting with a window, like [`MouseMotion`](winit::event::DeviceEvent::MouseMotion).
    /// Use this if, for example, you only want your app to update when the mouse cursor is
    /// moving over a window, not just moving in general. This can greatly reduce power consumption.
    pub fn reactive_low_power(wait: Duration) -> Self {
        Self::Reactive {
            wait,
            react_to_device_events: false,
            react_to_user_events: true,
            react_to_window_events: true,
        }
    }
}
