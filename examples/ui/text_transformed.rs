//! Demonstrates rotated and flipped text

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2dBundle::default());

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: Val::Px(20.),
                column_gap: Val::Px(20.),
                ..default()
            },
            ..default()
        })
        .id();

    let mut content_transform = UiContentTransform::default();
    for i in 0..8 {
        let text = commands
            .spawn(
                TextBundle::from_section(
                    "hello\n   bevy!",
                    TextStyle {
                        font_size: 20.0,
                        color: Color::WHITE,
                        ..default()
                    },
                )
                .with_background_color(Color::NAVY)
                .with_content_transform(content_transform),
            )
            .id();
        commands.entity(root).add_child(text);
        content_transform = content_transform.rotate_left();
        if i == 3 {
            content_transform = content_transform.flip_x();
        }
    }
}
