//! This is a `bevy` plugin that adds framepacing and framelimiting to improve input latency and
//! power use.
//!
//! # How it works
//!
//! This works by sleeping the app immediately before the event loop starts. In doing so, this
//! minimizes the time from when user input is captured (start of event loop), to when the frame is
//! presented on screen. Graphically, it looks like this:
//!
//! ```none
//!           /-- latency --\             /-- latency --\
//!  sleep -> input -> render -> sleep -> input -> render
//!  \----- event loop -----/    \----- event loop -----/
//! ```
//!
//! One of the interesting benefits of this is that you can keep latency low even if the framerate
//! is limited to a low value. Assuming you are able to reach the target frametime, there should be
//! no difference in motion-to-photon latency when limited to 10fps or 120fps.
//!
//! ```none
//!                same                                              same
//!           /-- latency --\                                   /-- latency --\
//!  sleep -> input -> render -> sleeeeeeeeeeeeeeeeeeeeeeeep -> input -> render
//!  \----- event loop -----/    \---------------- event loop ----------------/
//!           60 fps                           limited to 10 fps
//! ```

#![deny(missing_docs)]

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_platform::{
    sync::{Arc, Mutex},
    time::Instant,
};
use bevy_reflect::prelude::*;
use crate::{Render, RenderApp, RenderSystems};
#[cfg(not(target_arch = "wasm32"))]
use bevy_window::{Monitor, OnMonitor, Window};

use core::time::Duration;

#[cfg(feature = "framepace_debug")]
pub mod debug;

/// Adds frame-pacing and frame-limiting functionality to your [`App`].
#[derive(Debug, Clone, Component)]
pub struct FramepacePlugin;
impl Plugin for FramepacePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<FramepaceSettings>();

        let limit = FrametimeLimit::default();
        let settings = FramepaceSettings::default();
        let settings_proxy = FramepaceSettingsProxy::default();
        let stats = FramePaceStats::default();

        app.insert_resource(settings)
            .insert_resource(settings_proxy.clone())
            .insert_resource(limit.clone())
            .insert_resource(stats.clone())
            .add_systems(Update, update_proxy_resources);

        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, get_display_refresh_rate);

        app.sub_app_mut(RenderApp)
            .insert_resource(FrameTimer::default())
            .insert_resource(settings_proxy)
            .insert_resource(limit)
            .insert_resource(stats)
            .add_systems(
                Render,
                framerate_limiter
                    .in_set(RenderSystems::Cleanup)
                    .after(World::clear_entities),
            );
    }
}

/// Framepacing plugin configuration.
#[derive(Debug, Clone, Resource, Reflect)]
#[reflect(Resource)]
pub struct FramepaceSettings {
    /// Configures the framerate limiting strategy.
    pub limiter: Limiter,
}
impl FramepaceSettings {
    /// Builds plugin settings with the specified [`Limiter`] configuration.
    pub fn with_limiter(mut self, limiter: Limiter) -> Self {
        self.limiter = limiter;
        self
    }
}
impl Default for FramepaceSettings {
    fn default() -> FramepaceSettings {
        FramepaceSettings {
            limiter: Limiter::Auto,
        }
    }
}

#[derive(Default, Debug, Clone, Resource)]
struct FramepaceSettingsProxy {
    /// Configures the framerate limiting strategy.
    limiter: Arc<Mutex<Limiter>>,
}

#[cfg(not(target_arch = "wasm32"))]
impl FramepaceSettingsProxy {
    fn is_enabled(&self) -> bool {
        self.limiter.try_lock().iter().any(|l| l.is_enabled())
    }
}

fn update_proxy_resources(settings: Res<FramepaceSettings>, proxy: Res<FramepaceSettingsProxy>) {
    if settings.is_changed()
        && let Ok(mut limiter) = proxy.limiter.try_lock()
    {
        *limiter = settings.limiter.clone();
    }
}

/// Configures the framelimiting technique for the app.
#[derive(Debug, Default, Clone, Reflect)]
pub enum Limiter {
    /// Uses the window's refresh rate to set the frametime limit, updating when the window changes
    /// monitors.
    #[default]
    Auto,
    /// Set a fixed manual frametime limit. This should be greater than the monitors frametime
    /// (`1.0 / monitor frequency`).
    Manual(Duration),
    /// Disables frame limiting
    Off,
}

impl Limiter {
    /// Returns `true` if the [`Limiter`] is enabled.
    pub fn is_enabled(&self) -> bool {
        !matches!(self, Limiter::Off)
    }

    /// Constructs a new [`Limiter`] from the provided `framerate`.
    pub fn from_framerate(framerate: f64) -> Self {
        Limiter::Manual(Duration::from_secs_f64(1.0 / framerate))
    }
}

impl std::fmt::Display for Limiter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Limiter::Auto => write!(f, "Auto"),
            Limiter::Manual(t) => write!(f, "{:.2} fps", 1.0 / t.as_secs_f32()),
            Limiter::Off => write!(f, "Off"),
        }
    }
}

/// Current frametime limit based on settings and monitor refresh rate.
#[derive(Debug, Default, Clone, Resource)]
pub struct FrametimeLimit(pub Arc<Mutex<Duration>>);

/// Tracks the instant of the end of the previous frame.
#[derive(Debug, Clone, Resource, Reflect)]
pub struct FrameTimer {
    sleep_end: Instant,
}
impl Default for FrameTimer {
    fn default() -> Self {
        FrameTimer {
            sleep_end: Instant::now(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn get_display_refresh_rate(
    settings: Res<FramepaceSettings>,
    windows: Query<&OnMonitor, With<Window>>,
    monitors: Query<&Monitor>,
    frame_limit: Res<FrametimeLimit>,
) {
    let new_frametime = match settings.limiter {
        Limiter::Auto => match detect_frametime(&windows, &monitors) {
            Some(frametime) => frametime,
            None => return,
        },
        Limiter::Manual(frametime) => frametime,
        Limiter::Off => {
            #[cfg(feature = "framepace_debug")]
            if settings.is_changed() {
                bevy_log::info!("Frame limiter disabled");
            }
            return;
        }
    };

    if let Ok(mut limit) = frame_limit.0.try_lock()
        && new_frametime != *limit
    {
        #[cfg(feature = "framepace_debug")]
        bevy_log::info!("Frametime limit changed to: {:?}", new_frametime);
        *limit = new_frametime;
    }
}

/// Autodetect frame time from the refresh rate of the [`Monitor`] that each [`Window`] is on.
#[cfg(not(target_arch = "wasm32"))]
fn detect_frametime(
    windows: &Query<&OnMonitor, With<Window>>,
    monitors: &Query<&Monitor>,
) -> Option<Duration> {
    let best_framerate = {
        windows
            .iter()
            .filter_map(|on_monitor| monitors.get(on_monitor.0).ok()?.refresh_rate_millihertz)
            .min()? as f64
            / 1000.0
            - 0.5 // Winit only provides integer refresh rate values. We need to round down to handle the worst case scenario of a rounded refresh rate.
    };

    let best_frametime = Duration::from_secs_f64(1.0 / best_framerate);
    Some(best_frametime)
}

/// Holds frame time measurements for framepacing diagnostics
#[derive(Clone, Debug, Default, Resource)]
pub struct FramePaceStats {
    frametime: Arc<Mutex<Duration>>,
    oversleep: Arc<Mutex<Duration>>,
}

/// Accurately sleeps until it's time to start the next frame.
///
/// The `spin_sleep` dependency makes it possible to get extremely accurate sleep times across
/// platforms. Using `std::thread::sleep()` will not be precise enough, especially windows. Using a
/// spin lock, even with `std::hint::spin_loop()`, will result in significant power usage.
///
/// `spin_sleep` sleeps as long as possible given the platform's sleep accuracy, and spins for the
/// remainder. The dependency is however not WASM compatible, which is fine, because frame limiting
/// should not be used in a browser; this would compete with the browser's frame limiter.
#[cfg_attr(
    target_arch = "wasm32",
    expect(
        unused_variables,
        reason = "`settings` is only used when frame limiting is available"
    )
)]
fn framerate_limiter(
    mut timer: ResMut<FrameTimer>,
    target_frametime: Res<FrametimeLimit>,
    stats: Res<FramePaceStats>,
    settings: Res<FramepaceSettingsProxy>,
) {
    if let Ok(limit) = target_frametime.0.try_lock() {
        let frame_time = timer.sleep_end.elapsed();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let oversleep = stats
                .oversleep
                .try_lock()
                .as_deref()
                .cloned()
                .unwrap_or_default();
            let sleep_time = limit.saturating_sub(frame_time + oversleep);
            if settings.is_enabled() {
                spin_sleep::sleep(sleep_time);
            }
        }

        let frame_time_total = timer.sleep_end.elapsed();
        timer.sleep_end = Instant::now();
        if let Ok(mut frametime) = stats.frametime.try_lock() {
            *frametime = frame_time;
        }
        if let Ok(mut oversleep) = stats.oversleep.try_lock() {
            *oversleep = frame_time_total.saturating_sub(*limit);
        }
    };
}
