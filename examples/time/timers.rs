//! Illustrates how `Timer`s can be used both as resources and components.

use bevy::{log::info, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Countdown>()
        .add_systems(Startup, setup)
        .add_systems(Update, (countdown, print_when_completed))
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct PrintOnCompletionTimer(Timer);

#[derive(Resource)]
struct Countdown {
    percent_trigger: Timer,
    main_timer: Timer,
}

impl Countdown {
    pub fn new() -> Self {
        Self {
            percent_trigger: Timer::from_seconds(4.0, TimerMode::Repeating),
            main_timer: Timer::from_seconds(20.0, TimerMode::Once),
        }
    }
}

impl Default for Countdown {
    fn default() -> Self {
        Self::new()
    }
}

fn setup(mut commands: Commands) {
    // Add an entity to the world with a timer
    commands.spawn(PrintOnCompletionTimer(Timer::from_seconds(
        5.0,
        TimerMode::Once,
    )));
}

/// This system ticks the `Timer` on the entity with the `PrintOnCompletionTimer`
/// component using bevy's `Time` resource to get the delta between each update.
fn print_when_completed(time: Res<Time>, mut query: Query<&mut PrintOnCompletionTimer>) {
    for mut timer in &mut query {
        if timer.tick(time.delta()).just_finished() {
            info!("Entity timer just finished");
        }
    }
}

/// This system controls ticking the timer within the countdown resource and
/// handling its state.
fn countdown(time: Res<Time>, mut countdown: ResMut<Countdown>) {
    countdown.main_timer.tick(time.delta());

    // The API encourages this kind of timer state checking (if you're only checking for one value)
    // Additionally, `finished()` would accomplish the same thing as `just_finished` due to the
    // timer being repeating, however this makes more sense visually.
    if countdown.percent_trigger.tick(time.delta()).just_finished() {
        if !countdown.main_timer.finished() {
            // Print the percent complete the main timer is.
            info!(
                "Timer is {:0.0}% complete!",
                countdown.main_timer.fraction() * 100.0
            );
        } else {
            // The timer has finished so we pause the percent output timer
            countdown.percent_trigger.pause();
            info!("Paused percent trigger timer");
        }
    }
}
