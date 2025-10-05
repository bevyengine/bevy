//! Uses a system font to display text
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, mut fonts: ResMut<Assets<Font>>) {
    commands.spawn(Camera2d);

    let font = fonts.add(Font::Query {
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
        Node {
            width: percent(100.),
            height: percent(100.),
            display: Display::Flex,
            padding: UiRect::all(px(20.)),
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Text::new("System Font UI Text"),
            TextFont::default().with_font(font.clone()),
        )],
    ));

    commands.spawn((
        Text2d::new("System Font 2D Text"),
        TextFont::default().with_font(font),
    ));
}
