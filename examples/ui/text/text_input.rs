//! Demonstrates a simple, unstyled [`EditableText`] widget.
//!
//! [`EditableText`] is a basic primitive for text input in Bevy UI.
//! In most cases, this should be combined with other entities to create a compound widget
//! that includes e.g. a background, border, and text label.
//!
//! Note that while Bevy does offer clipboard support, access to the system clipboard is gated
//! behind an off-by-default feature (`system_clipboard` on `bevy_clipboard`).
//! When this is disabled, clipboard operations (copy, cut, paste) will operate on a simple in-memory buffer
//! that is not shared with the operating system.
//! This means that, unless you enable this feature,
//! you will not be able to copy text from your application and paste it into another application, or vice versa.
//!
//! Most applications that use text input will want to enable system clipboard support to meet user expectations for copy/paste behavior.
//! It is off by default to avoid forcing clipboard permissions on applications that do not need it but wish to use Bevy's UI solution for other widgets,
//! and to avoid including the `arboard` dependency on platforms where it is not supported or where clipboard access is not desired.
//! While desktop platforms generally support clipboard access without special permissions, some platforms (notably web and mobile)
//! may require additional permissions or user gestures to allow clipboard access;
//! this approach allows developers to opt in to full clipboard support only when they genuinely need it.
//!
//! To test this example using the system feature, run `cargo run --example text_input --features="system_clipboard"`.
//! To enable this feature in your own project, add the `system_clipboard` feature to your list of enabled features for `bevy` in your `Cargo.toml`.
//!
//! See the module documentation for [`editable_text`](bevy::ui_widgets::editable_text) for more details.
use bevy::color::palettes::css::DARK_GREY;
use bevy::color::palettes::tailwind::SLATE_300;
use bevy::input_focus::AutoFocus;
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let root = commands
        .spawn(Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            padding: px(20).all(),
            row_gap: px(16),
            ..default()
        })
        .id();

    let text_instructions = commands
        .spawn((
            Text::new("Ctrl+Enter to submit text"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(30.0),
                ..default()
            },
        ))
        .id();

    let text_input_left = build_input_text(&mut commands, true, 24.0);
    let text_input_right = build_input_text(&mut commands, false, 24.0);

    let input_container = commands
        .spawn((
            Node {
                display: Display::Flex,
                align_items: AlignItems::Start,
                column_gap: px(16),
                ..default()
            },
            AutoFocus,
            TabGroup::new(0),
        ))
        .id();

    // Set up a text output to see the result of our text input
    let text_output = commands
        .spawn((
            Node {
                width: px(416),
                border: px(2).all(),
                padding: px(8).all(),
                ..Default::default()
            },
            BorderColor::from(Color::from(SLATE_300)),
            Text::new(""),
            TextOutput,
            TextFont {
                font_size: FontSize::Px(24.0),
                ..default()
            },
        ))
        .id();

    commands
        .entity(input_container)
        .add_children(&[text_input_left, text_input_right]);

    commands
        .entity(root)
        .add_children(&[text_instructions, input_container, text_output]);
}

fn build_input_text(commands: &mut Commands, is_left: bool, font_size: f32) -> Entity {
    commands
        .spawn((
            Node {
                width: px(200),
                border: px(2).all(),
                padding: px(8).all(),
                ..Default::default()
            },
            BorderColor::from(Color::from(SLATE_300)),
            Name::new(if is_left { "Left" } else { "Right" }),
            EditableText {
                max_characters: (!is_left).then_some(7),
                ..Default::default()
            },
            TextFont {
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextCursorStyle::default(),
            TabIndex(if is_left { 0 } else { 1 }),
            BackgroundColor(DARK_GREY.into()),
        ))
        .id()
}

// Submit the text when Ctrl+Enter is pressed
fn text_submission(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut text_input: Query<(&mut EditableText, &Name)>,
    mut text_output: Single<&mut Text, With<TextOutput>>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter)
        && (keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight))
        && let Some(focused_entity) = input_focus.get()
        && let Ok((mut text_input, name)) = text_input.get_mut(focused_entity)
    {
        text_output.0 = format!("{:}: {:}", name, text_input.value());

        text_input.clear();
    }
}
