//! Demonstrates multiple text inputs

use bevy::color::palettes::css::YELLOW;
use bevy::input::keyboard::Key;
use bevy::input_focus::AutoFocus;
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputDispatchPlugin, InputFocus,
};
use bevy::prelude::*;
use bevy::text::{EditableText, FontCx, LayoutCx, TextCursorStyle};
use bevy::ui_widgets::EditableTextInputPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins((
            // This is also part of UiWidgetsPlugins, but we only need EditableText for this example
            EditableTextInputPlugin,
            // Input focus is required to direct keyboard input to the correct EditableText
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (update_output, text_submission))
        .run();
}

#[derive(Component)]
struct TextOutput;

#[derive(Component)]
struct SubmitOutput;

#[derive(Component)]
struct TextInputRow(usize);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let font = TextFont {
        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
        font_size: FontSize::Px(24.),
        ..default()
    };

    commands
        .spawn((
            Node {
                width: percent(100.),
                height: percent(100.),
                display: Display::Grid,
                justify_content: JustifyContent::Center,
                align_content: AlignContent::Center,
                grid_template_columns: RepeatedGridTrack::px(3, 320.),
                grid_template_rows: RepeatedGridTrack::auto(5),
                row_gap: px(16.),
                column_gap: px(16.),
                ..default()
            },
            TabGroup::default(),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Multiple Text Inputs Example"),
                Node {
                    grid_column: GridPlacement::span(3),
                    justify_self: JustifySelf::Center,
                    row_gap: px(6),
                    column_gap: px(6),
                    ..default()
                },
                TextColor::WHITE,
                font.clone(),
            ));

            parent.spawn((
                Text::new("Press Ctrl + Enter to submit"),
                Node {
                    grid_column: GridPlacement::span(3),
                    justify_self: JustifySelf::Center,
                    ..default()
                },
                font.clone(),
            ));

            for row in 0..3 {
                let mut input = parent.spawn((
                    Node {
                        border: px(5.).all(),
                        padding: px(5.).all(),
                        ..default()
                    },
                    EditableText::default(),
                    TextCursorStyle::default(),
                    font.clone(),
                    BackgroundColor(bevy::color::palettes::css::DARK_GREY.into()),
                    TextInputRow(row),
                    TabIndex(row as i32),
                    BorderColor::all(YELLOW),
                ));
                if row == 0 {
                    input.insert(AutoFocus);
                }

                parent.spawn((
                    Text::default(),
                    TextLayout {
                        linebreak: LineBreak::AnyCharacter,
                        ..default()
                    },
                    Node {
                        border: px(5.).all(),
                        padding: px(5.).all(),
                        ..default()
                    },
                    font.clone(),
                    BackgroundColor(bevy::color::palettes::css::DARK_SLATE_GRAY.into()),
                    BorderColor::all(Color::WHITE),
                    TextInputRow(row),
                    TextOutput,
                ));

                parent.spawn((
                    Text::default(),
                    TextLayout {
                        linebreak: LineBreak::AnyCharacter,
                        ..default()
                    },
                    Node {
                        border: px(5.).all(),
                        padding: px(5.).all(),
                        ..default()
                    },
                    font.clone(),
                    BackgroundColor(bevy::color::palettes::css::DARK_SLATE_BLUE.into()),
                    BorderColor::all(Color::WHITE),
                    TextInputRow(row),
                    SubmitOutput,
                ));
            }
        });
}

fn update_output(
    changed_inputs: Query<(&EditableText, &TextInputRow), Changed<EditableText>>,
    mut outputs: Query<(&mut Text, &TextInputRow), With<TextOutput>>,
) {
    for (editable_text, input_row) in &changed_inputs {
        for (mut text, output_row) in &mut outputs {
            if output_row.0 == input_row.0 {
                text.0 = editable_text.value().to_string();
            }
        }
    }
}

// Submit the text when Ctrl+Enter is pressed
fn text_submission(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<Key>>,
    mut text_input: Query<(&mut EditableText, &TextInputRow)>,
    mut text_output: Query<(&mut Text, &TextInputRow), With<SubmitOutput>>,
    mut font_context: ResMut<FontCx>,
    mut layout_context: ResMut<LayoutCx>,
) {
    if keyboard_input.just_pressed(Key::Enter)
        && keyboard_input.pressed(Key::Control)
        && let Some(focused_entity) = input_focus.get()
        && let Ok((mut text_input, input_row)) = text_input.get_mut(focused_entity)
    {
        for (mut text, output_row) in &mut text_output {
            if input_row.0 == output_row.0 {
                text.0 = text_input.value().to_string();
                break;
            }
        }

        text_input.clear(&mut font_context.0, &mut layout_context.0);
    }
}
