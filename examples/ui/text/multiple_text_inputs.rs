//! Demonstrates multiple text inputs
//!
//! This example arranges three text inputs in a 3x3 grid layout.  The first column of each row is an [`EditableText`] text input node, the second column is a `Text` node
//! that is kept synchronized with the [`EditableText`]'s contents by the [`synchronize_output_text`] system, and the third column is updated
//! by the [`submit_text`] system when the user submits the [`EditableText`]'s text by pressing `Ctrl` + `Enter`.

use bevy::color::palettes::tailwind::SLATE_300;
use bevy::input::keyboard::Key;
use bevy::input_focus::AutoFocus;
use bevy::input_focus::{
    tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin},
    InputFocus,
};
use bevy::prelude::*;
use bevy::text::{EditableText, TextCursorStyle};

fn main() {
    App::new()
        // `EditableTextInputPlugin` is part of `DefaultPlugins`
        .add_plugins((DefaultPlugins, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                synchronize_output_text,
                submit_text,
                update_row_border_colors,
            ),
        )
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
                grid_template_rows: RepeatedGridTrack::auto(6),
                row_gap: px(8.),
                column_gap: px(8.),
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
                    margin: px(16).bottom(),
                    ..default()
                },
                TextColor::WHITE,
                font.clone(),
            ));

            let label_font = font.clone().with_font_size(14.);
            for label in ["EditableText", "value", "submission"] {
                parent.spawn((
                    Text::new(label),
                    label_font.clone(),
                    Node {
                        justify_self: JustifySelf::Center,
                        margin: px(-4).bottom(),
                        ..default()
                    },
                ));
            }

            for row in 0..3 {
                let mut input = parent.spawn((
                    Node {
                        border: px(4.).all(),
                        padding: px(4.).all(),
                        ..default()
                    },
                    EditableText::new(format!("Initial text {row}")),
                    TextCursorStyle::default(),
                    font.clone(),
                    BackgroundColor(bevy::color::palettes::css::DARK_GREY.into()),
                    TextInputRow(row),
                    TabIndex(row as i32),
                    BorderColor::all(SLATE_300),
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
                        border: px(4.).all(),
                        padding: px(4.).all(),
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
                        border: px(4.).all(),
                        padding: px(4.).all(),
                        ..default()
                    },
                    font.clone(),
                    BackgroundColor(bevy::color::palettes::css::DARK_SLATE_BLUE.into()),
                    BorderColor::all(Color::WHITE),
                    TextInputRow(row),
                    SubmitOutput,
                ));
            }

            parent.spawn((
                Text::new("Press Ctrl + Enter to submit"),
                Node {
                    grid_column: GridPlacement::span(3),
                    justify_self: JustifySelf::Center,
                    margin: px(16).top(),
                    ..default()
                },
                font.clone(),
            ));
        });
}

/// This system keeps the text of the [`TextOutput`] [`Text`] nodes synchronized with the text
/// of the [`EditableText`] node on the same row.
fn synchronize_output_text(
    changed_inputs: Query<(&EditableText, &TextInputRow), Changed<EditableText>>,
    mut outputs: Query<(&mut Text, &TextInputRow), With<TextOutput>>,
) {
    for (editable_text, input_row) in &changed_inputs {
        for (mut text, output_row) in &mut outputs {
            if output_row.0 == input_row.0 {
                // `EditableText::value()` returns a `SplitString` because Parley may keep IME preedit text
                // in a contiguous range of the editor’s internal `String` buffer during composition.
                // The returned `SplitString` omits that preedit range, exposing only the text before and after it.
                //
                // To avoid allocating a new `String`, we reserve the total length of the `SplitString`'s slices,
                // then append them to the output `Text`.
                text.0.clear();
                text.0
                    .reserve(editable_text.value().into_iter().map(str::len).sum());
                for sub_str in editable_text.value() {
                    text.0.push_str(sub_str);
                }
            }
        }
    }
}

// Submit the focused input's text when Ctrl+Enter is pressed.
fn submit_text(
    input_focus: Res<InputFocus>,
    keyboard_input: Res<ButtonInput<Key>>,
    mut text_input: Query<(&mut EditableText, &TextInputRow)>,
    mut text_output: Query<(&mut Text, &TextInputRow), With<SubmitOutput>>,
) {
    if keyboard_input.just_pressed(Key::Enter)
        && keyboard_input.pressed(Key::Control)
        && let Some(focused_entity) = input_focus.get()
        && let Ok((mut editable_text, input_row)) = text_input.get_mut(focused_entity)
    {
        for (mut text, output_row) in &mut text_output {
            if input_row.0 == output_row.0 {
                text.0.clear();
                text.0
                    .reserve(editable_text.value().into_iter().map(str::len).sum());
                for sub_str in editable_text.value() {
                    text.0.push_str(sub_str);
                }
                break;
            }
        }
        editable_text.clear();
    }
}

/// Dim a row's border colors when its [`EditableText`] does not have input focus.
fn update_row_border_colors(
    input_focus: Res<InputFocus>,
    input_rows: Query<&TextInputRow, With<EditableText>>,
    mut row_borders: Query<(&TextInputRow, &mut BorderColor, Has<EditableText>)>,
) {
    if !input_focus.is_changed() {
        return;
    }

    let focused_row = input_focus
        .get()
        .and_then(|focused_entity| input_rows.get(focused_entity).ok())
        .map(|row| row.0);

    for (row, mut border_color, is_input) in &mut row_borders {
        let mut color = if is_input {
            SLATE_300.into()
        } else {
            Color::WHITE
        };
        if Some(row.0) != focused_row {
            color = color.darker(0.75);
        }
        border_color.set_all(color);
    }
}
