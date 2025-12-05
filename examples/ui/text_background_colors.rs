//! This example demonstrates UI text with a background color

use bevy::{
    color::palettes::css::{BLUE, GREEN, PURPLE, RED, YELLOW},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, cycle_text_background_colors)
        .run();
}

const PALETTE: [Color; 5] = [
    Color::Srgba(RED),
    Color::Srgba(GREEN),
    Color::Srgba(BLUE),
    Color::Srgba(YELLOW),
    Color::Srgba(PURPLE),
];

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    let message_text = [
        "T", "e", "x", "t\n", "B", "a", "c", "k", "g", "r", "o", "u", "n", "d\n", "C", "o", "l",
        "o", "r", "s", "!",
    ];

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn((
                    Text::default(),
                    TextLayout {
                        justify: Justify::Center,
                        ..Default::default()
                    },
                ))
                .with_children(|commands| {
                    for (i, section_str) in message_text.iter().enumerate() {
                        commands.spawn((
                            TextSpan::new(*section_str),
                            TextColor::BLACK,
                            TextFont {
                                font_size: 100.,
                                ..default()
                            },
                            TextBackgroundColor(PALETTE[i % PALETTE.len()]),
                        ));
                    }
                });
        });
}

fn cycle_text_background_colors(
    time: Res<Time>,
    children_query: Query<&Children, With<Text>>,
    mut text_background_colors_query: Query<&mut TextBackgroundColor>,
) {
    let n = time.elapsed_secs() as usize;
    let children = children_query.single().unwrap();

    for (i, child) in children.iter().enumerate() {
        text_background_colors_query.get_mut(child).unwrap().0 = PALETTE[(i + n) % PALETTE.len()];
    }
}
