//! Demonstrates a single, minimal multiline [`EditableText`] widget.

use bevy::color::palettes::css::{DARK_SLATE_GRAY, YELLOW};
use bevy::input_focus::{AutoFocus, InputDispatchPlugin};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};
use bevy::ui_widgets::EditableTextInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            EditableTextInputPlugin,
            // Required so keyboard input is sent to the focused `EditableText`.
            InputDispatchPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: percent(100.),
            height: percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .with_children(|parent| {
            parent.spawn((
                Node {
                    width: px(300.),
                    border: px(2.).all(),
                    padding: px(8.).all(),
                    ..default()
                },
                EditableText {
                    visible_lines: Some(8.),
                    ..default()
                },
                TextLayout {
                    linebreak: LineBreak::AnyCharacter,
                    ..default()
                },
                TextFont {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                    font_size: FontSize::Px(30.),
                    ..default()
                },
                TextCursorStyle::default(),
                BackgroundColor(DARK_SLATE_GRAY.into()),
                BorderColor::all(YELLOW),
                AutoFocus,
            ));
        });
}
