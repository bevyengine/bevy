//! Demonstrates IME (Input Method Editor) support for text input.
//!
//! IME allows users to input characters that aren't directly on their keyboard,
//! such as Chinese, Japanese, and Korean characters.
//!
//! To use IME input, the system must have fonts installed that support the target script.
//! This example uses [`FontSource::SansSerif`], which resolves to a system sans-serif font.
//! On systems without e.g. CJK fonts installed, CJK input will render as boxes or question marks.
use bevy::color::palettes::css::{DARK_GREY, YELLOW};
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputFocus,
};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TabNavigationPlugin)
        .add_systems(Startup, setup)
        .add_systems(Update, text_submission)
        .run();
}

#[derive(Component)]
struct TextOutput;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let instructions = commands
        .spawn((
            Text::new("Type using your IME, then press Ctrl+Enter to submit. Your system default sans-serif font will be used, so make sure you have fonts installed that support the characters you want to input!"),
            TextFont {
                font_size: FontSize::Px(20.0),
                ..default()
            },
            Node {
                margin: UiRect::bottom(px(16.0)),
                ..default()
            },
        ))
        .id();

    let text_input = commands
        .spawn((
            Node {
                width: px(400),
                border: px(3).all(),
                padding: px(8).all(),
                ..default()
            },
            // SansSerif resolves to a system sans-serif font, which on most CJK systems
            // includes support for Chinese, Japanese, and Korean characters.
            // Note that using system fonts requires the "bevy/system-fonts" feature to be enabled.
            TextFont {
                font: FontSource::SansSerif,
                font_size: FontSize::Px(32.0),
                ..default()
            },
            BorderColor::from(Color::from(YELLOW)),
            EditableText::default(),
            TextCursorStyle::default(),
            TabIndex(0),
            BackgroundColor(DARK_GREY.into()),
        ))
        .id();

    let text_output = commands
        .spawn((
            Text::new("Your text here!"),
            TextFont {
                font: FontSource::SansSerif,
                font_size: FontSize::Px(32.0),
                ..default()
            },
            TextOutput,
            Node {
                margin: UiRect::top(px(16.0)),
                ..default()
            },
        ))
        .id();

    commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: px(24.0).all(),
                ..default()
            },
            TabGroup::new(0),
        ))
        .add_children(&[instructions, text_input, text_output]);
}

fn text_submission(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut text_input: Query<&mut EditableText>,
    mut text_output: Single<&mut Text, With<TextOutput>>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter)
        && (keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight))
        && let Some(focused_entity) = input_focus.get()
        && let Ok(mut input) = text_input.get_mut(focused_entity)
    {
        text_output.0 = input.value().to_string();
        input.clear();
    }
}
