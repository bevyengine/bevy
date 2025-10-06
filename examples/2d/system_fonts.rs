//! Uses a system font to display text
use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::once_after_delay};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            add_default_font_text.run_if(once_after_delay(Duration::from_secs(1))),
        )
        .run();
}

fn setup(mut commands: Commands, mut fonts: ResMut<Assets<Font>>, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let system_font = fonts.add(Font::Query {
        families: vec![
            Family::Name("Liberation Sans".to_string()),
            Family::Name("Ubuntu".to_string()),
            Family::Name("Noto Sans".to_string()),
        ],
        weight: Weight::NORMAL,
        stretch: Stretch::Normal,
        style: Style::Normal,
    });

    commands.spawn((
        Text2d::new("System Font Text"),
        TextFont::default().with_font(system_font),
        Transform::from_xyz(0., 100., 0.),
    ));

    commands.spawn((
        Text2d::new("Fira Sans Bold Text"),
        TextFont::default().with_font(asset_server.load("fonts/FiraSans-Bold.ttf")),
    ));
}

fn add_default_font_text(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    let default_font = fonts.add(Font::Query {
        families: vec![Family::Name("Fira Sans".to_string())],
        weight: Weight::BOLD,
        stretch: Stretch::Normal,
        style: Style::Normal,
    });

    commands.spawn((
        Text2d::new("Queried Fira Sans Text"),
        TextFont::default().with_font(default_font),
        Transform::from_xyz(0., -100., 0.),
    ));
}
