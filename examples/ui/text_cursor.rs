//! This example illustrates how to create and control a UI text cursor

use bevy::{
    color::palettes::css::GOLDENROD,
    diagnostic::FrameTimeDiagnosticsPlugin,
    prelude::*,
    ui::widget::{TextCursor, TextCursorStyle, TextCursorWidth},
};

const CURSOR_WIDTH: f32 = 4.;

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

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            row_gap: Val::Px(50.),
            flex_direction: FlexDirection::Column,
            ..Default::default()
        })
        .with_child((
            Text::new(
                "Lorem ipsum dolor sit amet,\n\
                consectetur adipiscing elit,\n\
                sed do eiusmod tempor incididunt\n\
                ut labore et dolore magna aliqua.",
            ),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 30.0,
                ..default()
            },
            TextColor(GOLDENROD.into()),
            TextCursor { line: 0, index: 0 },
            TextLayout::new_with_justify(JustifyText::Center),
            Outline {
                color: Color::WHITE,
                width: Val::Px(2.),
                offset: Val::Px(25.),
            },
        ))
        .with_child((
            Text::new(
                "Lorem ipsum dolor sit amet,\n\
                consectetur adipiscing elit,\n\
                sed do eiusmod tempor incididunt\n\
                ut labore et dolore magna aliqua.",
            ),
            TextFont {
                font_size: 30.0,
                ..default()
            },
            TextCursor { line: 0, index: 0 },
            TextColor(GOLDENROD.into()),
            TextLayout::new_with_justify(JustifyText::Center),
            Outline {
                color: Color::WHITE,
                width: Val::Px(2.),
                ..Default::default()
            },
        ))
        .with_child(Text::new(
            "Press arrow keys to move the cursors and space to toggle the cursor style.",
        ));
}

fn move_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut cursors: Query<(&mut TextCursor, &mut TextCursorStyle)>,
) {
    for (mut cursor, mut style) in &mut cursors {
        if keys.just_pressed(KeyCode::ArrowLeft) {
            cursor.index = cursor.index.saturating_sub(1);
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            cursor.index += 1;
        }
        if keys.just_pressed(KeyCode::KeyQ) {
            cursor.line = cursor.line.saturating_sub(1);
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            cursor.line += 1;
        }
        if keys.just_pressed(KeyCode::Space) {
            style.width = match style.width {
                TextCursorWidth::All => TextCursorWidth::Px(CURSOR_WIDTH),
                TextCursorWidth::Px(_) => TextCursorWidth::All,
            };
        }
    }
}
