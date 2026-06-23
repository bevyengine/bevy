//! Demonstrates a minimal [`EditableTextFilter`] with an 8-character hex input.

use bevy::color::palettes::css::DARK_SLATE_GRAY;
use bevy::color::palettes::tailwind::SLATE_300;
use bevy::input_focus::AutoFocus;
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
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
                    width: px(240.),
                    border: px(2.).all(),
                    padding: px(8.).all(),
                    ..default()
                },
                EditableText {
                    max_characters: Some(8),
                    ..default()
                },
                TextCursorStyle::default(),
                EditableTextFilter::new(|c| c.is_ascii_hexdigit()),
                TextFont::from_font_size(32.),
                BackgroundColor(DARK_SLATE_GRAY.into()),
                BorderColor::all(SLATE_300),
                AutoFocus,
            ));
        });
}
