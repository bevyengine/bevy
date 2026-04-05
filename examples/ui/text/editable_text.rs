//! Demonstrates a simple, unstyled [`EditableText`] widget.
//!
//! [`EditableText`] is a basic primitive for text input in Bevy UI.
//! In most cases, this should be combined with other entities to create a compound widget
//! that includes e.g. a background, border, and text label.
//!
//! See the module documentation for [`editable_text`](bevy::ui_widgets::editable_text) for more details.
use bevy::color::palettes::css::{DARK_GREY, YELLOW};
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputFocus,
};
use bevy::prelude::*;
use bevy::text::{EditableText, FontCx, LayoutCx, TextCursorStyle};

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

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut input_focus: ResMut<InputFocus>,
) {
    // Set up a camera
    // We need a camera to see the UI
    commands.spawn(Camera2d);

    // Create a root UI node, so we can place the input above the output in a column
    let root = commands
        .spawn(Node {
            display: Display::Block,
            ..default()
        })
        .id();

    let font: FontSource = asset_server.load("fonts/FiraMono-Medium.ttf").into();

    // Instructions
    let text_instructions = commands
        .spawn((
            Node {
                width: px(400),
                height: px(100),
                ..Default::default()
            },
            BorderColor::from(Color::from(YELLOW)),
            Text::new("Ctrl+Enter to submit text"),
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(30.0),
                ..default()
            },
            UiTransform::from_translation(Val2::ZERO),
        ))
        .id();

    // Set up an EditableText widget
    let text_input_left = build_input_text(&mut commands, &font, true, 30.0);
    let text_input_right = build_input_text(&mut commands, &font, false, 50.0);

    // Set the focus to our text input so we can start typing right away
    input_focus.set(text_input_left);

    let input_container = commands
        .spawn((
            Node {
                display: Display::Flex,
                align_items: AlignItems::Start,
                ..default()
            },
            TabGroup::new(0),
        ))
        .id();

    // Set up a text output to see the result of our text input
    let text_output = commands
        .spawn((
            Node {
                width: px(400),
                height: px(100),
                border: px(5).all(),
                ..Default::default()
            },
            BorderColor::from(Color::from(YELLOW)),
            Text::new("testing"),
            TextOutput,
            TextFont {
                font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                font_size: FontSize::Px(70.0),
                ..default()
            },
            UiTransform::from_translation(Val2::px(5.0, 200.0)),
        ))
        .id();

    // Assemble our hierarchy
    commands
        .entity(input_container)
        .add_children(&[text_input_left, text_input_right]);

    commands
        .entity(root)
        .add_children(&[text_instructions, input_container, text_output]);
}

fn build_input_text(
    commands: &mut Commands,
    font: &FontSource,
    is_left: bool,
    font_size: f32,
) -> Entity {
    commands
        .spawn((
            Node {
                width: px(200),
                border: px(5).all(),
                padding: px(5).all(),
                ..Default::default()
            },
            BorderColor::from(Color::from(YELLOW)),
            Name::new(if is_left { "Left" } else { "Right" }),
            EditableText {
                max_characters: (!is_left).then_some(7),
                ..Default::default()
            },
            TextFont {
                font: font.clone(),
                font_size: FontSize::Px(font_size),
                ..default()
            },
            TextCursorStyle::default(),
            TabIndex(if is_left { 0 } else { 1 }),
            BackgroundColor(DARK_GREY.into()),
            UiTransform::from_translation(Val2::px(if is_left { 0.0 } else { 300.0 }, 50.0)),
        ))
        .id()
}

// Submit the text when Ctrl+Enter is pressed
fn text_submission(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut text_input: Query<(&mut EditableText, &Name)>,
    mut text_output: Single<&mut Text, With<TextOutput>>,
    mut font_context: ResMut<FontCx>,
    mut layout_context: ResMut<LayoutCx>,
) {
    if keyboard_input.just_pressed(KeyCode::Enter)
        && (keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight))
        && let Some(focused_entity) = input_focus.get()
        && let Ok((mut text_input, name)) = text_input.get_mut(focused_entity)
    {
        text_output.0 = format!("{:}: {:}", name, text_input.value());

        text_input.clear(&mut font_context.0, &mut layout_context.0);
    }
}
