//! This example illustrates how to play a pitch

use bevy::prelude::*;
use std::time::Duration;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut pitch_assets: ResMut<Assets<Pitch>>, mut commands: Commands) {
    commands.spawn(PitchBundle {
        source: pitch_assets.add(Pitch::new(220.0, Duration::new(1, 0))),
        ..default()
    });
}
