//! This example illustrates how to create and control a UI text cursor

use bevy::{
    color::palettes::css::GOLDENROD,
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
            // Add `TextCursor` to a `Text` entity to display a cursor.
            TextCursor { line: 0, index: 0 },
            Outline {
                color: Color::WHITE,
                width: Val::Px(2.),
                offset: Val::Px(15.),
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
            // Default is line = 0, index = 0
            TextCursor::default(),
            TextColor(GOLDENROD.into()),
            Outline {
                color: Color::WHITE,
                width: Val::Px(2.),
                offset: Val::Px(25.),
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
        // This function assumes the text consists of single-byte characters
        // and doesn't work correctly with multi-byte UTF-8.
        if keys.just_pressed(KeyCode::ArrowLeft) {
            cursor.index = cursor.index.saturating_sub(1);
        }
        if keys.just_pressed(KeyCode::ArrowRight) {
            // `cursor.index is a byte offset into the string,
            // with multi-byte characters the key will need to
            // be pressed multiple times to advance the cursor.
            cursor.index += 1;
        }
        if keys.just_pressed(KeyCode::ArrowUp) {
            cursor.line = cursor.line.saturating_sub(1);
        }
        if keys.just_pressed(KeyCode::ArrowDown) {
            // No out-of-bounds checks are performed.
            cursor.line += 1;
        }
        if keys.just_pressed(KeyCode::Space) {
            // Toggle between a narrow line cursor and a block
            // cursor that covers the entire glyph.
            style.width = match style.width {
                TextCursorWidth::All => TextCursorWidth::Px(3.),
                TextCursorWidth::Px(_) => TextCursorWidth::All,
            };
        }
    }
}
