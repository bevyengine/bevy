//! This example illustrates how to create and control a UI text cursor

use bevy::{
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    ui::widget::{TextCursor, TextCursorStyle, TextCursorWidth},
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FrameTimeDiagnosticsPlugin::default()))
        .add_systems(Startup, setup)
        .add_systems(Update, move_cursor)
        .run();
}
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);
    // Text with one section
    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        })
        .with_child((
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            Text::new(
                "Lorem ipsum dolor sit amet,\nconsectetur adipiscing elit,\nsed do eiusmod tempor incididunt\nut labore et dolore magna aliqua."),
            TextFont {
                // This font is loaded and will be used instead of the default font.
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                ..default()
            },
            TextCursor {
                index: 5,
            },
            TextCursorStyle {
                color: Color::WHITE,
                width: TextCursorWidth::Px(4.),
                radius: 2.,
                height: 1.,
            },
            // Set the justification of the Text
            TextLayout::new_with_justify(JustifyText::Center),
            // Set the style of the Node itself.
            Outline {
                color: Color::WHITE,
                width: Val::Px(2.),
                offset: Val::Px(25.),
            },
        ));
}

fn move_cursor(buttons: Res<ButtonInput<KeyCode>>, mut cursors: Query<&mut TextCursor>) {
    for mut cursor in &mut cursors {
        if buttons.just_pressed(KeyCode::ArrowLeft) {
            cursor.index = cursor.index.saturating_sub(1);
        }
        if buttons.just_pressed(KeyCode::ArrowRight) {
            cursor.index += 1;
        }
    }
}
