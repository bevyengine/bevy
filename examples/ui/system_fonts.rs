//! This example demonstrates using system fonts.

use bevy::{prelude::*, text::TextPipeline};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(PostStartup, setup2)
        .run()
}

fn setup(mut commands: Commands, mut text_pipeline: ResMut<TextPipeline>) {
    bevy::log::info!("setup");
    text_pipeline.load_system_fonts();
    commands.spawn(Camera2dBundle::default());
}

fn setup2(mut commands: Commands) {
    bevy::log::info!("setup2");
    commands.spawn(TextBundle::from_sections([TextSection {
        value: "Test".into(),
        style: TextStyle {
            font: bevy::text::FontRef::Query(bevy::text::FontQuery::default()),
            font_size: 50.0,
            color: Color::WHITE,
        },
    }]));
}
