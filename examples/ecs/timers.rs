//! Illustrates how `Timer`s can be used both as resources and components.

use bevy::{log::info, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<Countdown>()
        .add_startup_system(setup)
        .add_system(countdown)
        .add_system(print_when_completed)
        .run();
}

#[derive(Component, Deref, DerefMut)]
pub struct PrintOnCompletionTimer(Timer);

pub struct Countdown {
    pub percent_trigger: Timer,
    pub main_timer: Timer,
}

impl Countdown {
    pub fn new() -> Self {
        Self {
            percent_trigger: Timer::from_seconds(4.0, true),
            main_timer: Timer::from_seconds(20.0, false),
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
    commands
        .spawn()
        .insert(PrintOnCompletionTimer(Timer::from_seconds(5.0, false)));
}

/// This system ticks all the `Timer` components on entities within the scene
/// using bevy's `Time` resource to get the delta between each update.
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
                countdown.main_timer.percent() * 100.0
            );
        } else {
            // The timer has finished so we pause the percent output timer
            countdown.percent_trigger.pause();
            info!("Paused percent trigger timer");
        }
    }
}
