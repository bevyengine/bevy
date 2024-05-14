//! This example illustrates how to force a web tab to run in the background.
//!
//! This can be useful for a multiplayer game where you don't want to the client to time out
//! even if the tab is in the background.

use bevy::{
    prelude::*,
    utils::Duration,
    winit::{WinitSettings},
};
use bevy::time::common_conditions::on_real_timer;
use bevy::winit::UpdateMode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Continuous rendering for games - bevy's default.
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Reactive {
                wait: Duration::from_secs_f32(1.0 / 60.0),
            },
        })
        .add_systems(Update, test_setup::increment_counter.run_if(on_real_timer(Duration::from_secs(1))))
        .run();
}

/// Everything in this module is for setting up and animating the scene, and is not important to the
/// demonstrated features.
pub(crate) mod test_setup {
    use bevy::prelude::*;
    /// Rotate the cube to make it clear when the app is updating
    pub(crate) fn increment_counter(
        mut counter: Local<usize>,
    ) {
        *counter += 1;
        info!("Counter: {}", *counter);
    }
}
