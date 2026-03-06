use bevy::prelude::*;
use bevy::window::{Monitor, PrimaryWindow};
use bevy::window::OnMonitor;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, print_monitor)
        .run();
}

fn print_monitor(primary_window: Single<&OnMonitor, With<PrimaryWindow>>, monitors: Query<(Entity, &Monitor)>) {
    println!("{:?}", monitors.iter().find(|(e, ..)| *e == primary_window.0).unwrap().1.name)
}