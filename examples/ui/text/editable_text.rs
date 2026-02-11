//! Demonstrates a simple, unstyled [`EditableText`] widget.
//!
//! [`EditableText`] is a basic primitive for text input in Bevy UI.
//! In most cases, this should be combined with other entities to create a compound widget
//! that includes e.g. a background, border, and text label.
//!
//! See the module documentation for [`editable_text`](bevy::ui_widgets::editable_text) for more details.
use bevy::input_focus::{InputDispatchPlugin, InputFocus};
use bevy::prelude::*;
use bevy::text::EditableText;
use bevy::ui_widgets::EditableTextInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            // This is also part of UiWidgetsPlugins, but we only need EditableText for this example
            EditableTextInputPlugin,
            // Input focus is required to direct keyboard input to the correct EditableText
            InputDispatchPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, text_submission)
        .run();
}
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_focus: ResMut<InputFocus>,
) {
    // Set up a camera
    // We need a camera to see the UI
    commands.spawn(Camera2d::default());

    // Create a root UI node, so we can place the input above the output in a column
    // TODO: center things nicely
    let root = commands.spawn(Node { ..default() }).id();

    // Set up an EditableText widget
    let text_input = commands
        .spawn((
            EditableText::default(),
            TextFont {
                font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                font_size: FontSize::Px(70.0),
                ..default()
            },
        ))
        .id();

    // Set the focus to our text input so we can start typing right away
    input_focus.set(text_input);

    // Set up a text output to see the result of our text input
    let text_output = commands
        .spawn((
            Text::new("testing"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(70.0),
                ..default()
            },
        ))
        .id();

    // Assemble our hierarchy
    commands
        .entity(root)
        .add_children(&[text_input, text_output]);
}

// Submit the text when Ctrl+Enter is pressed
fn text_submission(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut text_input: Query<&mut EditableText>,
    mut text_output: Single<&mut Text>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter)
        && (keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight))
    {
        if let Some(focused_entity) = input_focus.get() {
            if let Some(mut text_input) = text_input.get_mut(focused_entity).ok() {
                text_output.0 = text_input.value().clone().to_string();
                text_input.clear();
            }
        }
    }
}
