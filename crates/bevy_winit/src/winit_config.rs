use bevy_ecs::system::Resource;
use bevy_utils::Duration;

/// Settings for the [`WinitPlugin`](super::WinitPlugin) app runner.
#[derive(Debug, Resource)]
pub struct WinitSettings {
    /// Controls how the [`EventLoop`](winit::event_loop::EventLoop) is deployed.
    ///
    /// - If this value is set to `false` (default), [`run`] is called, and exiting the loop will
    /// terminate the program.
    /// - If this value is set to `true`, [`run_return`] is called, and exiting the loop will
    /// return control to the caller.
    ///
    /// **NOTE:** This cannot be changed while the loop is running. `winit` discourages use of `run_return`.
    ///
    /// # Supported platforms
    ///
    /// `run_return` is only available on the following `target_os` environments:
    /// - `windows`
    /// - `macos`
    /// - `linux`
    /// - `freebsd`
    /// - `openbsd`
    /// - `netbsd`
    /// - `dragonfly`
    ///
    /// The runner will panic if this is set to `true` on other platforms.
    ///
    /// [`run`]: https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html#method.run
    /// [`run_return`]: https://docs.rs/winit/latest/winit/platform/run_return/trait.EventLoopExtRunReturn.html#tymethod.run_return
    pub return_on_loop_exit: bool,
    /// Determines how frequently the application can update when it has focus.
    pub focused_mode: UpdateMode,
    /// Determines how frequently the application can update when it's out of focus.
    pub unfocused_mode: UpdateMode,
}

impl WinitSettings {
    /// Default settings for games.
    ///
    /// [`Continuous`](UpdateMode::Continuous) if windows have focus or not.
    pub fn game() -> Self {
        WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
            ..Default::default()
        }
    }

    /// Default settings for desktop applications.
    ///
    /// [`Reactive`](UpdateMode::Reactive) if windows have focus,
    /// [`ReactiveLowPower`](UpdateMode::ReactiveLowPower) otherwise.
    pub fn desktop_app() -> Self {
        WinitSettings {
            focused_mode: UpdateMode::Reactive {
                min_wait: Duration::from_secs(5),
            },
            unfocused_mode: UpdateMode::ReactiveLowPower {
                min_wait: Duration::from_secs(60),
            },
            ..Default::default()
        }
    }

    /// Returns the focused or unfocused [`UpdateMode`].
    pub fn update_mode(&self, focused: bool) -> &UpdateMode {
        match focused {
            true => &self.focused_mode,
            false => &self.unfocused_mode,
        }
    }
}

impl Default for WinitSettings {
    fn default() -> Self {
        WinitSettings {
            return_on_loop_exit: false,
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        }
    }
}

/// Determines how frequently an app can update.
///
/// **NOTE:** This setting is independent of VSync. VSync is controlled by a window's
/// [`PresentMode`](bevy_window::PresentMode) setting. If an app can update faster than
/// the refresh rate, but VSync is enabled, the update rate will be indirectly limited
/// by the renderer.
#[derive(Debug)]
pub enum UpdateMode {
    /// The app will update over and over, as fast as it possibly can.
    Continuous,
    /// The app will update when:
    /// - enough time has elapsed since the previous update
    /// - a redraw is requested
    /// - new window or device events have appeared
    Reactive {
        /// The minimum time to wait from the start of one update to the next.
        ///
        /// **Note:** This has no upper limit.
        /// Bevy will wait forever if you set this to [`Duration::MAX`].
        min_wait: Duration,
    },
    /// The app will update when:
    /// - enough time has elapsed since the previous update
    /// - a redraw is requested
    /// - new window events have appeared
    ///
    /// **Note:** Unlike [`Reactive`](`UpdateMode::Reactive`), this mode ignores device events.
    /// Use this mode if, for example, you only want your app to update when the mouse cursor is
    /// moving over a window, not just moving in general. This can greatly reduce power consumption.
    ReactiveLowPower {
        /// The minimum time to wait from the start of one update to the next.
        ///
        /// **Note:** This has no upper limit.
        /// Bevy will wait forever if you set this to [`Duration::MAX`].
        min_wait: Duration,
    },
}
