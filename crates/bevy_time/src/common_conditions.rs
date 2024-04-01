use crate::{Real, Time, Timer, TimerMode, Virtual};
use bevy_ecs::system::Res;
use bevy_utils::Duration;

/// Run condition that is active on a regular time interval, using [`Time`] to advance
/// the timer. The timer ticks at the rate of [`Time::relative_speed`].
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::on_timer;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(on_timer(Duration::from_secs(1))),
///         )
///     .run();
/// }
/// fn tick() {
///     // ran once a second
/// }
/// ```
///
/// Note that this does **not** guarantee that systems will run at exactly the
/// specified interval. If delta time is larger than the specified `duration` then
/// the system will only run once even though the timer may have completed multiple
/// times. This condition should only be used with large time durations (relative to
/// delta time).
///
/// For more accurate timers, use the [`Timer`] class directly (see
/// [`Timer::times_finished_this_tick`] to address the problem mentioned above), or
/// use fixed timesteps that allow systems to run multiple times per frame.
pub fn on_timer(duration: Duration) -> impl FnMut(Res<Time>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Repeating);
    move |time: Res<Time>| {
        timer.tick(time.delta());
        timer.just_finished()
    }
}

/// Run condition that is active on a regular time interval,
/// using [`Time<Real>`] to advance the timer.
/// The timer ticks are not scaled.
///
/// ```no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::on_real_timer;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(on_real_timer(Duration::from_secs(1))),
///         )
///         .run();
/// }
/// fn tick() {
///     // ran once a second
/// }
/// ```
///
/// Note that this does **not** guarantee that systems will run at exactly the
/// specified interval. If delta time is larger than the specified `duration` then
/// the system will only run once even though the timer may have completed multiple
/// times. This condition should only be used with large time durations (relative to
/// delta time).
///
/// For more accurate timers, use the [`Timer`] class directly (see
/// [`Timer::times_finished_this_tick`] to address the problem mentioned above), or
/// use fixed timesteps that allow systems to run multiple times per frame.
pub fn on_real_timer(duration: Duration) -> impl FnMut(Res<Time<Real>>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Repeating);
    move |time: Res<Time<Real>>| {
        timer.tick(time.delta());
        timer.just_finished()
    }
}

/// Run condition that is active *once* after the specified delay,
/// using [`Time`] to advance the timer.
/// The timer ticks at the rate of [`Time::relative_speed`].
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::once_after_delay;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(once_after_delay(Duration::from_secs(1))),
///         )
///     .run();
/// }
/// fn tick() {
///     // ran once, after a second
/// }
/// ```
pub fn once_after_delay(duration: Duration) -> impl FnMut(Res<Time>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Once);
    move |time: Res<Time>| {
        timer.tick(time.delta());
        timer.just_finished()
    }
}

/// Run condition that is active *once* after the specified delay,
/// using [`Time<Real>`] to advance the timer.
/// The timer ticks are not scaled.
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::once_after_delay;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(once_after_delay(Duration::from_secs(1))),
///         )
///     .run();
/// }
/// fn tick() {
///     // ran once, after a second
/// }
/// ```
pub fn once_after_real_delay(duration: Duration) -> impl FnMut(Res<Time<Real>>) -> bool + Clone {
    let mut timer = Timer::new(duration, crate::TimerMode::Once);
    move |time: Res<Time<Real>>| {
        timer.tick(time.delta());
        timer.just_finished()
    }
}

/// Run condition that is active *indefinitely* after the specified delay,
/// using [`Time`] to advance the timer.
/// The timer ticks at the rate of [`Time::relative_speed`].
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::repeating_after_delay;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(repeating_after_delay(Duration::from_secs(1))),
///         )
///     .run();
/// }
/// fn tick() {
///     // ran every frame, after a second
/// }
/// ```
pub fn repeating_after_delay(duration: Duration) -> impl FnMut(Res<Time>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Once);
    move |time: Res<Time>| {
        timer.tick(time.delta());
        timer.finished()
    }
}

/// Run condition that is active *indefinitely* after the specified delay,
/// using [`Time<Real>`] to advance the timer.
/// The timer ticks are not scaled.
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::IntoSystemConfigs;
/// # use bevy_utils::Duration;
/// # use bevy_time::common_conditions::repeating_after_real_delay;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             tick.run_if(repeating_after_real_delay(Duration::from_secs(1))),
///         )
///     .run();
/// }
/// fn tick() {
///     // ran every frame, after a second
/// }
/// ```
pub fn repeating_after_real_delay(
    duration: Duration,
) -> impl FnMut(Res<Time<Real>>) -> bool + Clone {
    let mut timer = Timer::new(duration, TimerMode::Once);
    move |time: Res<Time<Real>>| {
        timer.tick(time.delta());
        timer.finished()
    }
}

/// Run condition that is active when the [`Time<Virtual>`] clock is paused.
/// Use [`bevy_ecs::schedule::common_conditions::not`] to make it active when
/// it's not paused.
///
/// ```rust,no_run
/// # use bevy_app::{App, NoopPluginGroup as DefaultPlugins, PluginGroup, Update};
/// # use bevy_ecs::schedule::{common_conditions::not, IntoSystemConfigs};
/// # use bevy_time::common_conditions::paused;
/// fn main() {
///     App::new()
///         .add_plugins(DefaultPlugins)
///         .add_systems(
///             Update,
///             (
///                 is_paused.run_if(paused),
///                 not_paused.run_if(not(paused)),
///             )
///         )
///     .run();
/// }
/// fn is_paused() {
///     // ran when time is paused
/// }
///
/// fn not_paused() {
///     // ran when time is not paused
/// }
/// ```
pub fn paused(time: Res<Time<Virtual>>) -> bool {
    time.is_paused()
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::schedule::{IntoSystemConfigs, Schedule};

    fn test_system() {}

    // Ensure distributive_run_if compiles with the common conditions.
    #[test]
    fn distributive_run_if_compiles() {
        Schedule::default().add_systems(
            (test_system, test_system)
                .distributive_run_if(on_timer(Duration::new(1, 0)))
                .distributive_run_if(paused),
        );
    }
}
