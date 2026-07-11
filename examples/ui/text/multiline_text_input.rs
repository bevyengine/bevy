//! Demonstrates a single, minimal multiline [`EditableText`] widget.

use bevy::color::palettes::css::DARK_SLATE_GRAY;
use bevy::color::palettes::tailwind::SLATE_300;
use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input_focus::pointer_focus::PointerFocusPlugin;
use bevy::input_focus::tab_navigation::{TabGroup, TabIndex, TabNavigationPlugin};
use bevy::input_focus::{AutoFocus, FocusCause, FocusedInput, InputFocus};
use bevy::prelude::*;
use bevy::text::{EditableText, EditableTextFilter, TextCursorStyle};
use bevy::ui_widgets::{
    popover::{Popover, PopoverAlign, PopoverPlacement, PopoverSide},
    Activate, Button, MenuAction, MenuButton, MenuEvent, MenuFocusState, MenuItem, MenuPopup,
    SelectAllOnFocus,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, TabNavigationPlugin, PointerFocusPlugin))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct MultilineInput;

#[derive(Component)]
struct VisibleLinesInput;

#[derive(Component)]
struct FontSizeInput;

#[derive(Component)]
struct SelectionRadiusInput;

#[derive(Component)]
struct JustifyLabel;

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
                                linebreak: LineBreak::WordOrCharacter,
                                ..default()
                            },
                            TextCursorStyle {
                                color: Color::WHITE,
                                selected_text_color: Some(Color::BLACK),
                                ..default()
                            },
                            TextFont {
                                font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                font_size: FontSize::Px(30.),
                                ..default()
                            },
                            BackgroundColor(DARK_SLATE_GRAY.into()),
                            BorderColor::all(SLATE_300),
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
                                    BorderColor::all(SLATE_300),
                                    EditableText::new("8"),
                                    EditableTextFilter::new(|c| c.is_ascii_digit() || c == '.'),
                                    TextCursorStyle {
                                        color: Color::WHITE,
                                        selected_text_color: Some(Color::BLACK),
                                        unfocused_selection_color: Color::NONE,
                                        ..default()
                                    },
                                    SelectAllOnFocus,
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
                                    BorderColor::all(SLATE_300),
                                    EditableText::new("30"),
                                    EditableTextFilter::new(|c| c.is_ascii_digit()),
                                    TextCursorStyle {
                                        color: Color::WHITE,
                                        selected_text_color: Some(Color::BLACK),
                                        unfocused_selection_color: Color::NONE,
                                        ..default()
                                    },
                                    SelectAllOnFocus,
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

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Row,
                                column_gap: px(10.),
                                ..default()
                            },
                            children![
                                (
                                    Text::new("corner radius:"),
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
                                    BorderColor::all(SLATE_300),
                                    EditableText::new("0"),
                                    EditableTextFilter::new(|c| c.is_ascii_digit() || c == '.'),
                                    TextCursorStyle {
                                        color: Color::WHITE,
                                        selected_text_color: Some(Color::BLACK),
                                        ..default()
                                    },
                                    SelectionRadiusInput,
                                    TabIndex(2),
                                )
                            ],
                        ))
                        .observe(
                            |on: On<FocusedInput<KeyboardInput>>,
                             radius_input_query: Query<
                                &EditableText,
                                With<SelectionRadiusInput>,
                            >,
                             mut cursor_style: Single<
                                &mut TextCursorStyle,
                                With<MultilineInput>,
                            >| {
                                if !(on.input.state.is_pressed()
                                    && on.input.logical_key == Key::Enter)
                                {
                                    return;
                                }

                                let Ok(input) = radius_input_query.get(on.original_event_target())
                                else {
                                    return;
                                };

                                let mut output = String::new();
                                output.reserve(input.value().into_iter().map(str::len).sum());
                                for sub_str in input.value() {
                                    output.push_str(sub_str);
                                }

                                let Ok(radius) = output.parse::<f32>() else {
                                    return;
                                };

                                cursor_style.selection_radius = radius.clamp(0., 0.5);
                            },
                        );

                    parent
                        .spawn(Node::default())
                        .observe(
                            |on: On<MenuEvent>,
                             mut popup: Single<
                                (&mut Node, &mut MenuFocusState),
                                With<MenuPopup>,
                            >,
                             button: Single<Entity, With<MenuButton>>,
                             mut focus: ResMut<InputFocus>| {
                                match on.action {
                                    MenuAction::Open(direction) => {
                                        popup.0.display = Display::Flex;
                                        *popup.1 = MenuFocusState::Opening(direction);
                                    }
                                    MenuAction::Toggle => {
                                        if popup.0.display == Display::None {
                                            popup.0.display = Display::Flex;
                                            *popup.1 = MenuFocusState::Opening(
                                                bevy::input_focus::tab_navigation::NavAction::First,
                                            );
                                        } else {
                                            popup.0.display = Display::None;
                                        }
                                    }
                                    MenuAction::CloseAll => {
                                        popup.0.display = Display::None;
                                    }
                                    MenuAction::FocusRoot => {
                                        focus.set(*button, FocusCause::Navigated);
                                    }
                                }
                            },
                        )
                        .with_children(|parent| {
                            parent.spawn((
                                Node {
                                    border: px(2.).all(),
                                    padding: px(8.).horizontal(),
                                    ..default()
                                },
                                Button,
                                MenuButton,
                                TabIndex(4),
                                BackgroundColor(DARK_SLATE_GRAY.into()),
                                BorderColor::all(SLATE_300),
                                children![(
                                    Text::new("Justify::Left"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraMono-Medium.ttf").into(),
                                        font_size: FontSize::Px(24.),
                                        ..default()
                                    },
                                    JustifyLabel,
                                )],
                            ));

                            parent
                                .spawn((
                                    Node {
                                        display: Display::None,
                                        flex_direction: FlexDirection::Column,
                                        min_width: percent(100.),
                                        border: px(2.).all(),
                                        position_type: PositionType::Absolute,
                                        ..default()
                                    },
                                    MenuPopup::default(),
                                    Popover {
                                        positions: vec![PopoverPlacement {
                                            side: PopoverSide::Top,
                                            align: PopoverAlign::End,
                                            gap: 2.,
                                        }],
                                        ..default()
                                    },
                                    GlobalZIndex(1),
                                    BackgroundColor(DARK_SLATE_GRAY.into()),
                                    BorderColor::all(SLATE_300),
                                ))
                                .with_children(|parent| {
                                    for (label, justify) in [
                                        ("Justify::Left", Justify::Left),
                                        ("Justify::Center", Justify::Center),
                                        ("Justify::Right", Justify::Right),
                                        ("Justify::Justified", Justify::Justified),
                                        ("Justify::Start", Justify::Start),
                                        ("Justify::End", Justify::End),
                                    ] {
                                        parent
                                            .spawn((
                                                Node {
                                                    padding: px(8.).horizontal(),
                                                    ..default()
                                                },
                                                MenuItem,
                                                TabIndex(0),
                                                children![(
                                                    Text::new(label),
                                                    TextFont {
                                                        font: asset_server
                                                            .load("fonts/FiraMono-Medium.ttf",)
                                                            .into(),
                                                        font_size: FontSize::Px(24.),
                                                        ..default()
                                                    },
                                                )],
                                            ))
                                            .observe(
                                                move |_: On<Activate>,
                                                      mut layout: Single<
                                                    &mut TextLayout,
                                                    With<MultilineInput>,
                                                >,
                                                      mut selected: Single<
                                                    &mut Text,
                                                    With<JustifyLabel>,
                                                >| {
                                                    layout.justify = justify;
                                                    selected.0 = label.into();
                                                },
                                            );
                                    }
                                });
                        });
                });
        });
}
