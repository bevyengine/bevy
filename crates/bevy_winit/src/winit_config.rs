use bevy_ecs::resource::Resource;
use core::time::Duration;

/// Settings for the [`WinitPlugin`](super::WinitPlugin).
#[derive(Debug, Resource, Clone, Copy)]
pub struct WinitSettings {
    /// Determines how frequently the application can update when it has focus.
    pub focused_mode: (MainUpdateMode, RenderUpdateMode),
    /// Determines how frequently the application can update when it's out of focus.
    pub unfocused_mode: (MainUpdateMode, RenderUpdateMode),
}

impl WinitSettings {
    /// Default settings for games.
    ///
    /// [`OnEachFrame`](MainUpdateMode::OnEachFrame) if windows have focus,
    /// [`reactive_low_power`](MainUpdateMode::reactive_low_power) otherwise.
    pub fn game() -> Self {
        WinitSettings {
            focused_mode: (
                MainUpdateMode::OnEachFrame { min_ticktime: None },
                RenderUpdateMode::Continuous,
            ),
            unfocused_mode: (
                MainUpdateMode::reactive_low_power(Duration::from_secs_f64(1.0 / 60.0)), /* 60Hz, */
                RenderUpdateMode::OnEachMainUpdate {
                    min_frametime: None,
                },
            ),
        }
    }

    /// Default settings for desktop applications.
    ///
    /// [`Reactive`](MainUpdateMode::Reactive) if windows have focus,
    /// [`reactive_low_power`](MainUpdateMode::reactive_low_power) otherwise.
    ///
    /// Use the [`EventLoopProxy`](crate::EventLoopProxy) to request a redraw from outside bevy.
    pub fn desktop_app() -> Self {
        WinitSettings {
            focused_mode: (
                MainUpdateMode::reactive(Duration::from_secs(5)),
                RenderUpdateMode::OnEachMainUpdate {
                    min_frametime: None,
                },
            ),
            unfocused_mode: (
                MainUpdateMode::reactive_low_power(Duration::from_secs(60)),
                RenderUpdateMode::OnEachMainUpdate {
                    min_frametime: None,
                },
            ),
        }
    }

    /// Default settings for mobile.
    ///
    /// [`Reactive`](MainUpdateMode::Reactive) if windows have focus,
    /// [`reactive_low_power`](MainUpdateMode::reactive_low_power) otherwise.
    ///
    /// Use the [`EventLoopProxy`](crate::EventLoopProxy) to request a redraw from outside bevy.
    pub fn mobile() -> Self {
        WinitSettings {
            focused_mode: (
                MainUpdateMode::reactive(Duration::from_secs_f32(1.0 / 60.0)),
                RenderUpdateMode::OnEachMainUpdate {
                    min_frametime: None,
                },
            ),
            unfocused_mode: (
                MainUpdateMode::reactive_low_power(Duration::from_secs(1)),
                RenderUpdateMode::OnEachMainUpdate {
                    min_frametime: None,
                },
            ),
        }
    }

    /// Setting for continous update of the application
    pub fn continous_update() -> Self {
        WinitSettings {
            focused_mode: (MainUpdateMode::Continuous, RenderUpdateMode::Continuous),
            unfocused_mode: (MainUpdateMode::Continuous, RenderUpdateMode::Continuous),
        }
    }

    /// Returns the current [`UpdateMode`].
    ///
    /// **Note:** The output depends on whether the window has focus or not.
    pub fn update_mode(&self, focused: bool) -> (MainUpdateMode, RenderUpdateMode) {
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

/// Determines how frequently the main [`SubApp`](bevy_app::SubApp) should update.
///
/// **Note:** This only controls how fast the main [`SubApp`](bevy_app::SubApp) is updated.
/// To control the update behavior of the rendering [`SubApp`](bevy_app::SubApp) use
/// [`RenderUpdateMode`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MainUpdateMode {
    /// The main [`SubApp`](bevy_app::SubApp) will update over and over, as fast as it possibly can,
    /// until an [`AppExit`](bevy_app::AppExit) event appears.
    Continuous,
    /// The main [`SubApp`](bevy_app::SubApp) will update after every [`Duration`] has passed, until an
    /// [`AppExit`](bevy_app::AppExit) event appears.
    Fixed(Duration),
    /// The main [`SubApp`](bevy_app::SubApp) will update after every frame is presented, until an
    /// [`AppExit`](bevy_app::AppExit) event appears.
    /// This should be used to update the main [`SubApp`](bevy_app::SubApp) at the same rate as the
    /// present rate (framerate) of the window.
    OnEachFrame {
        /// A floor on the time that needs to pass before the main [`SubApp`](bevy_app::SubApp) is
        /// updated. It can be set to 1.0 / TPS to achive a desired upper bound on the TPS.
        min_ticktime: Option<Duration>,
    },
    /// The main [`SubApp`](bevy_app::SubApp) will update in response to the following, until an
    /// [`AppExit`](bevy_app::AppExit) event appears:
    /// - `wait` time has elapsed since the previous update
    /// - new [window](`winit::event::WindowEvent`), [raw input](`winit::event::DeviceEvent`), or custom
    ///     events have appeared
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
        /// IMPORTANT: Does not react to [`winit::event::WindowEvent::RedrawRequested`]
        react_to_window_events: bool,
    },
}

impl MainUpdateMode {
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

/// Determines how frequently the render [`SubApp`](bevy_app::SubApp) should update. This
/// determines how often frames are generated. This does not determine how often they are
/// presented. This setting is independent of VSync. VSync is controlled by a window's
/// [`PresentMode`](bevy_window::PresentMode) setting. If the render [`SubApp`](bevy_app::SubApp)
/// tries to update update faster than the refresh rate, but VSync is enabled, the update rate
/// will be limited to match the refresh rate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderUpdateMode {
    /// The render [`SubApp`](bevy_app::SubApp) will update over and over, as fast as it possibly
    /// can only limited by the window setting (in case of VSync) and the GPU.
    Continuous,
    /// The render [`SubApp`](bevy_app::SubApp) will update after every [`Duration`] has passed.
    Fixed(Duration),
    /// The render [`SubApp`](bevy_app::SubApp) will update after every main [`SubApp`](bevy_app::SubApp)
    /// update.
    ///
    /// **Note:** You can control the update rate of the main [`SubApp`](bevy_app::SubApp) using
    /// [`MainUpdateMode`].
    OnEachMainUpdate {
        /// A floor on the time that needs to pass before the render [`SubApp`](bevy_app::SubApp) is
        /// updated. It can be set to 1.0 / FPS to achive a desired upper bound on the FPS.
        min_frametime: Option<Duration>,
    },
}
