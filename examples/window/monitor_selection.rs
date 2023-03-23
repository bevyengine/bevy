use bevy::{prelude::*, window::WindowMode};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, bevy::window::close_on_esc)
        .add_systems(Startup, spawn_window)
        .run();
}

fn spawn_window(mut commands: Commands) {
    commands.spawn(Window {
        monitor_selection: MonitorSelection::Index(1),
        mode: WindowMode::BorderlessFullscreen,
        ..default()
    });
}
