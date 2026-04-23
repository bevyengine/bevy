//! Demonstrates a single, minimal multiline [`EditableText`] widget.

use bevy::color::palettes::css::{DARK_SLATE_GRAY, YELLOW};
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin};
use bevy::input_focus::{AutoFocus, FocusedInput};
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct MultilineInput;

#[derive(Component)]
struct VisibleLinesInput;

#[derive(Component)]
struct FontSizeInput;

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
            parent
                .spawn((
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::End,
                        row_gap: px(10.),
                        ..default()
                    },
                    TabGroup::default(),
                ))
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: px(450.),
                                border: px(2.).all(),
                                padding: px(8.).all(),
                                ..default()
                            },
                            EditableText {
                                visible_lines: Some(8.),
                                allow_newlines: true,
                                ..default()
                            },
                            TextLayout {
                                linebreak: LineBreak::AnyCharacter,
                                ..default()
                            },
                            TextCursorStyle {
                                selected_text_color: Some(Color::BLACK),
                                ..default()
                            },
                            TextFont {
                                font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                font_size: FontSize::Px(30.),
                                ..default()
                            },
                            BackgroundColor(DARK_SLATE_GRAY.into()),
                            BorderColor::all(YELLOW),
                            MultilineInput,
                            TabIndex(0),
                            AutoFocus,
                        ))
                        .observe(
                            |on: On<FocusedInput<KeyboardInput>>,
                             keys: Res<ButtonInput<Key>>,
                             input_query: Query<&EditableText, With<MultilineInput>>| {
                                if !(on.input.state.is_pressed()
                                    && on.input.logical_key == Key::Enter
                                    && keys.pressed(Key::Control))
                                {
                                    return;
                                }
                                let Ok(input) = input_query.get(on.focused_entity) else {
                                    return;
                                };

                                let mut output = String::new();
                                output.reserve(input.value().into_iter().map(str::len).sum());
                                for sub_str in input.value() {
                                    output.push_str(sub_str);
                                }

                                info!("{output}"                                    );
                            },
                        );

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: px(10.),
                                ..default()
                            },
                            children![
                                (
                                    Text::new("visible lines:"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                        font_size: FontSize::Px(30.),
                                        ..default()
                                    },
                                ),
                                (
                                    Node {
                                        width: px(100.),
                                        border: px(2.).all(),
                                        ..default()
                                    },
                                    TextFont {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                        font_size: FontSize::Px(30.),
                                        ..default()
                                    },
                                    TextLayout {
                                        justify: Justify::End,
                                        ..default()
                                    },
                                    BackgroundColor(DARK_SLATE_GRAY.into()),
                                    BorderColor::all(YELLOW),
                                    EditableText::new("8"),
                                    EditableTextFilter::new(|c| c.is_ascii_digit()),
                                    TextCursorStyle {
                                        selected_text_color: Some(Color::BLACK),
                                        ..default()
                                    },
                                    VisibleLinesInput,
                                    TabIndex(1),
                                )
                            ],
                        ))
                        .observe(
                            |on: On<FocusedInput<KeyboardInput>>,
                             mut query_set: ParamSet<(
                                Query<&EditableText, With<VisibleLinesInput>>,
                                Query<&mut EditableText, With<MultilineInput>>,
                            )>| {
                                if !(on.input.state.is_pressed()
                                    && on.input.logical_key == Key::Enter)
                                {
                                    return;
                                }

                                let visible_lines_query = query_set.p0();
                                let Ok(input) = visible_lines_query.get(on.original_event_target())
                                else {
                                    return;
                                };

                                let mut output = String::new();
                                output.reserve(input.value().into_iter().map(str::len).sum());
                                for sub_str in input.value() {
                                    output.push_str(sub_str);
                                }

                                let Ok(lines) = output.parse::<f32>() else {
                                    return;
                                };

                                let mut multiline_query = query_set.p1();
                                let Ok(mut multiline_input) = multiline_query.single_mut() else {
                                    return;
                                };

                                multiline_input.visible_lines = Some(lines.clamp(1., 10.));
                            },
                        );

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: px(10.),
                                ..default()
                            },
                            children![
                                (
                                    Text::new("font size:"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                        font_size: FontSize::Px(30.),
                                        ..default()
                                    },
                                ),
                                (
                                    Node {
                                        width: px(100.),
                                        border: px(2.).all(),
                                        ..default()
                                    },
                                    TextFont {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                        font_size: FontSize::Px(30.),
                                        ..default()
                                    },
                                    TextLayout {
                                        justify: Justify::End,
                                        ..default()
                                    },
                                    BackgroundColor(DARK_SLATE_GRAY.into()),
                                    BorderColor::all(YELLOW),
                                    EditableText::new("30"),
                                    EditableTextFilter::new(|c| c.is_ascii_digit()),
                                    TextCursorStyle {
                                        selected_text_color: Some(Color::BLACK),
                                        ..default()
                                    },
                                    FontSizeInput,
                                    TabIndex(2),
                                )
                            ],
                        ))
                        .observe(
                            |on: On<FocusedInput<KeyboardInput>>,
                             font_size_input_query: Query<&EditableText, With<FontSizeInput>>,
                             mut multiline_input_font: Single<
                                &mut TextFont,
                                With<MultilineInput>,
                            >| {
                                if !(on.input.state.is_pressed()
                                    && on.input.logical_key == Key::Enter)
                                {
                                    return;
                                }

                                let Ok(input) =
                                    font_size_input_query.get(on.original_event_target())
                                else {
                                    return;
                                };

                                let mut output = String::new();
                                output.reserve(input.value().into_iter().map(str::len).sum());
                                for sub_str in input.value() {
                                    output.push_str(sub_str);
                                }

                                let Ok(font_size) = output.parse::<f32>() else {
                                    return;
                                };

                                multiline_input_font.font_size =
                                    FontSize::Px(font_size.clamp(5., 50.));
                            },
                        );
                });
        });
}
