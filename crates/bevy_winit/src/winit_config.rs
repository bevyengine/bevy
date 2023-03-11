use bevy_ecs::system::Resource;
use bevy_utils::Duration;

/// A resource for configuring usage of the `rust_winit` library.
#[derive(Debug, Resource)]
pub struct WinitSettings {
    /// Configures `winit` to return control to the caller after exiting the
    /// event loop, enabling [`App::run()`](bevy_app::App::run()) to return.
    ///
    /// By default, [`return_from_run`](Self::return_from_run) is `false` and *Bevy*
    /// will use `winit`'s
    /// [`EventLoop::run()`](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html#method.run)
    /// to initiate the event loop.
    /// [`EventLoop::run()`](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoop.html#method.run)
    /// will never return but will terminate the process after the event loop exits.
    ///
    /// Setting [`return_from_run`](Self::return_from_run) to `true` will cause *Bevy*
    /// to use `winit`'s
    /// [`EventLoopExtRunReturn::run_return()`](https://docs.rs/winit/latest/winit/platform/run_return/trait.EventLoopExtRunReturn.html#tymethod.run_return)
    /// instead which is strongly discouraged by the `winit` authors.
    ///
    /// # Supported platforms
    ///
    /// This feature is only available on the following desktop `target_os` configurations:
    /// `windows`, `macos`, `linux`, `dragonfly`, `freebsd`, `netbsd`, and `openbsd`.
    ///
    /// Setting [`return_from_run`](Self::return_from_run) to `true` on
    /// unsupported platforms will cause [`App::run()`](bevy_app::App::run()) to panic!
    pub return_from_run: bool,
    /// Configures how the winit event loop updates while the window is focused.
    pub focused_mode: UpdateMode,
    /// Configures how the winit event loop updates while the window is *not* focused.
    pub unfocused_mode: UpdateMode,
}
impl WinitSettings {
    /// Configure winit with common settings for a game.
    pub fn game() -> Self {
        WinitSettings::default()
    }

    /// Configure winit with common settings for a desktop application.
    pub fn desktop_app() -> Self {
        WinitSettings {
            focused_mode: UpdateMode::Reactive {
                max_wait: Duration::from_secs(5),
            },
            unfocused_mode: UpdateMode::ReactiveLowPower {
                max_wait: Duration::from_secs(60),
            },
            ..Default::default()
        }
    }

    /// Gets the configured `UpdateMode` depending on whether the window is focused or not
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
            return_from_run: false,
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
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
    /// no way to force the app to wake and update again, unless a `winit` event (such as user
    /// input, or the window being resized) is received or the time limit is reached.
    Reactive {
        /// The maximum time to wait before the event loop runs again.
        ///
        /// Note that Bevy will wait indefinitely if the duration is too high (such as [`Duration::MAX`]).
        max_wait: Duration,
    },
    /// The event loop will only update if there is a winit event from direct interaction with the
    /// window (e.g. mouseover), a redraw is requested, or the maximum wait time has elapsed.
    ///
    /// ## Note
    ///
    /// Once the app has executed all bevy systems and reaches the end of the event loop, there is
    /// no way to force the app to wake and update again, unless a `winit` event (such as user
    /// input, or the window being resized) is received or the time limit is reached.
    ///
    /// ## Differences from [`UpdateMode::Reactive`]
    ///
    /// Unlike [`UpdateMode::Reactive`], this mode will ignore winit events that aren't directly
    /// caused by interaction with the window. For example, you might want to use this mode when the
    /// window is not focused, to only re-draw your bevy app when the cursor is over the window, but
    /// not when the mouse moves somewhere else on the screen. This helps to significantly reduce
    /// power consumption by only updated the app when absolutely necessary.
    ReactiveLowPower {
        /// The maximum time to wait before the event loop runs again.
        ///
        /// Note that Bevy will wait indefinitely if the duration is too high (such as [`Duration::MAX`]).
        max_wait: Duration,
    },
}
