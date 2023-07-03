use bevy_app::{UpdateFlow, RenderFlow};
use bevy_ecs::{schedule::BoxedScheduleLabel, system::Resource};

/// A resource for configuring usage of the [`winit`] library.
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
    /// The frequency at which schedule `main_schedule_label` runs per second.
    /// Also, determines the virtual time that elapses for each tick.
    ///
    /// The default is `125.`.
    /// This value is consistent with polling rates for typical input devices,
    /// thus optimizing input responsiveness.
    pub tick_rate: f64,
    /// Allow tick skipping if runs are overdue.
    ///
    /// If this is enabled, may slow down the game simulation, so it is recommended to disable this for action game.
    ///
    /// If this is disabled, may result in instantaneous game simulation when tick running load is variable,
    /// so it is recommended to enable this for not action game.
    ///
    /// The default is `false`.
    pub allow_tick_skip: bool,
    /// Limit on the frequency at which schedule `render_schedule_label` runs per second.
    /// Even if this is set to a higher value, the monitor refresh rate will be capped.
    ///
    /// Redrawing will not be performed unless requested for each frame.
    /// Redraw requests are made when:
    ///
    /// - From the OS as necessary.
    /// - Called `WinitHandler::redraw`.
    /// - The events specified by `redraw_when_tick`, `redraw_when_window_event`, and `redraw_when_device_event` occurred.
    ///
    /// The default is `f64::INFINITY`.
    pub frame_rate_limit: f64,
    /// Request redraw after running ticks.
    ///
    /// The default is `true`.
    pub redraw_when_tick: bool,
    /// Request redraw after receiving window events.
    ///
    /// The default is `true`.
    pub redraw_when_window_event: bool,
    /// Request redraw after receiving device events.
    ///
    /// The default is `true`.
    pub redraw_when_device_event: bool,
    /// The main schedule to be run for each tick.
    ///
    /// The default is [`Main`].
    pub main_schedule_label: BoxedScheduleLabel,
    /// The render schedule to be run for each frame.
    ///
    /// The default is [`Render`].
    pub render_schedule_label: BoxedScheduleLabel,
}

impl WinitSettings {
    /// Configure winit with basic settings.
    pub fn new(tick_rate: f64, allow_tick_skip: bool) -> Self {
        Self {
            tick_rate,
            allow_tick_skip,
            ..Default::default()
        }
    }

    /// Configure winit with common settings for a game.
    pub fn game() -> Self {
        unimplemented!()
    }

    /// Configure winit with common settings for a desktop application.
    pub fn desktop_app() -> Self {
        unimplemented!()
    }
}

impl Default for WinitSettings {
    fn default() -> Self {
        WinitSettings {
            return_from_run: false,
            tick_rate: 125.,
            allow_tick_skip: false,
            frame_rate_limit: f64::INFINITY,
            redraw_when_tick: true,
            redraw_when_window_event: true,
            redraw_when_device_event: true,
            main_schedule_label: Box::new(UpdateFlow),
            render_schedule_label: Box::new(RenderFlow),
        }
    }
}
