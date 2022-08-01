use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

#[derive(Component)]
pub struct Nothing;

#[derive(Bundle)]
pub struct NoBundle {
    nothing: Nothing,
}

fn startup(mut commands: Commands) {
    let mut entities = Vec::new();
    for _ in 0..40_000_000 {
        entities.push(NoBundle { nothing: Nothing });
    }

    commands.spawn_batch(entities);
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: 1270.0,
            height: 720.0,
            title: String::from("Bug"),
            ..Default::default()
        })
        .insert_resource(ClearColor(Color::rgb(0.211, 0.643, 0.949)))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(startup)
        .run();
}
