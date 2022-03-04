use bevy_utils::Duration;

/// A resource for configuring usage of the `rust_winit` library.
#[derive(Debug)]
pub struct WinitConfig {
    /// Configures the winit library to return control to the main thread after
    /// the [run](bevy_app::App::run) loop is exited. Winit strongly recommends
    /// avoiding this when possible. Before using this please read and understand
    /// the [caveats](winit::platform::run_return::EventLoopExtRunReturn::run_return)
    /// in the winit documentation.
    ///
    /// This feature is only available on desktop `target_os` configurations.
    /// Namely `windows`, `macos`, `linux`, `dragonfly`, `freebsd`, `netbsd`, and
    /// `openbsd`. If set to true on an unsupported platform
    /// [run](bevy_app::App::run) will panic.
    pub return_from_run: bool,
    /// Configures how the winit event loop updates while the window is focused.
    pub focused_mode: UpdateMode,
    /// Configures how the winit event loop updates while the window is *not* focused.
    pub not_focused_mode: UpdateMode,
}
impl WinitConfig {
    /// Configure winit with common settings for a game.
    pub fn game() -> Self {
        WinitConfig::default()
    }
    /// Configure winit with common settings for a desktop application.
    pub fn desktop_app() -> Self {
        WinitConfig {
            focused_mode: UpdateMode::Reactive {
                max_wait: Duration::from_secs(60),
            },
            not_focused_mode: UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_secs(300),
            },
            ..Default::default()
        }
    }
    pub fn update_mode(&self, focused: bool) -> &UpdateMode {
        match focused {
            true => &self.focused_mode,
            false => &self.not_focused_mode,
        }
    }
}
impl Default for WinitConfig {
    fn default() -> Self {
        WinitConfig {
            return_from_run: false,
            focused_mode: UpdateMode::Continuous,
            not_focused_mode: UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_millis(100),
            },
        }
    }
}

/// Configure how the winit event loop should update.
#[derive(Debug)]
pub enum UpdateMode {
    /// The event loop will update continuously, running as fast as possible.
    Continuous,
    /// The event loop will only update if there is a winit event, a redraw is requested, or the
    /// maximum wait time has elapsed.
    ///
    /// ## Note
    ///
    /// Once the app has executed all bevy systems and reaches the end of the event loop, there is
    /// no way to force the app wake and update again, unless a winit event is received or the time
    /// limit is reached.
    Reactive { max_wait: Duration },
    /// The event loop will only update if there is a winit event from direct interaction with the
    /// window (e.g. mouseover), a redraw is requested, or the maximum wait time has elapsed.
    ///
    /// ## Note
    ///
    /// Once the app has executed all bevy systems and reaches the end of the event loop, there is
    /// no way to force the app wake and update again, unless a winit event is received or the time
    /// limit is reached.
    ///
    /// ## Differences from [`UpdateMode::Reactive`]
    ///
    /// Unlike [`UpdateMode::Reactive`], this mode will ignore winit events that aren't directly
    /// caused by interaction with the window. For example, you might want to use this mode when the
    /// window is not focused, to only re-draw your bevy app when the cursor is over the window, but
    /// not when the mouse moves somewhere else on the screen. This helps to significantly reduce
    /// power consumption by only updated the app when absolutely necessary.
    ReactiveLowPower { max_wait: Duration },
}
