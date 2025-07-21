//! multiple text inputs example

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::YELLOW;
use bevy::core_widgets::Activate;
use bevy::core_widgets::Callback;
use bevy::core_widgets::CoreButton;
use bevy::core_widgets::CoreRadio;
use bevy::core_widgets::CoreRadioGroup;
use bevy::core_widgets::CoreWidgetsPlugins;
use bevy::core_widgets::TrackClick;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::text::TextInputFilter;
use bevy::text::TextInputPasswordMask;
use bevy::ui::widget::TextInput;
use bevy_ecs::relationship::RelatedSpawnerCommands;
use bevy_ecs::system::command::spawn_batch;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InputDispatchPlugin,
            TabNavigationPlugin,
            CoreWidgetsPlugins,
        ))
        .add_systems(Startup, setup)
        .run();
}

#[derive(Component)]
struct DemoInput;

const FONT_OPTIONS: [[&'static str; 2]; 3] = [
    ["fonts/FiraMono-Medium.ttf", "FiraMono"],
    ["fonts/FiraSans-Bold.ttf", "FiraSans"],
    ["fonts/Orbitron-Medium.ttf", "Orbitron"],
];

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.),
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn((
                    Node {
                        display: Display::Grid,
                        border: UiRect::all(Val::Px(5.)),
                        padding: UiRect::all(Val::Px(25.)),
                        row_gap: Val::Px(20.),
                        column_gap: Val::Px(20.),
                        grid_template_columns: vec![GridTrack::fr(1.); 4],
                        ..default()
                    },
                    BorderColor::all(YELLOW.into()),
                    BackgroundColor(NAVY.into()),
                ))
                .with_children(|commands| {
                    for (i, label) in ["Type", "Input", "Value", "Submission"]
                        .into_iter()
                        .enumerate()
                    {
                        commands.spawn((
                            Text::new(label),
                            Node {
                                grid_column: GridPlacement::start(i as i16 + 1),
                                grid_row: GridPlacement::start(1),
                                justify_self: JustifySelf::Center,
                                ..Default::default()
                            },
                        ));
                    }

                    inputs_grid(commands);
                });
            commands
                .spawn((Node {
                    column_gap: Val::Px(20.),
                    ..Default::default()
                },))
                .with_children(|commands| {
                    for [font, label] in FONT_OPTIONS.iter() {
                        let font = asset_server.load(*font);
                        spawn_font_button(commands, font, label);
                    }
                });
        });
}

fn inputs_grid(commands: &mut RelatedSpawnerCommands<ChildOf>) {
    for (n, (label, input_filter, password)) in [
        ("alphanumeric", TextInputFilter::Alphanumeric, false),
        ("decimal", TextInputFilter::Decimal, false),
        ("hex", TextInputFilter::Hex, false),
        ("integer", TextInputFilter::Integer, false),
        (
            "not bevy",
            TextInputFilter::custom(|text| !text.contains("bevy")),
            false,
        ),
        ("password", TextInputFilter::Alphanumeric, true),
    ]
    .into_iter()
    .enumerate()
    {
        let row = n as i16 * 2 + 2;
        commands.spawn((
            Node {
                display: Display::Grid,
                height: Val::Px(2.),
                grid_column: GridPlacement::start_end(1, 5),
                grid_row: GridPlacement::start(row),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ));

        let row = row + 1;

        commands.spawn((
            Node {
                display: Display::Grid,
                grid_row: GridPlacement::start(row),
                grid_column: GridPlacement::start(1),
                justify_content: JustifyContent::End,
                ..Default::default()
            },
            children![(Text::new(label), TextColor(YELLOW.into()),)],
        ));

        let mut input = commands.spawn((
            TextInput {
                justify: Justify::Left,
            },
            input_filter,
            DemoInput,
            TextColor(Color::WHITE),
            TabIndex(0),
            Node {
                width: Val::Px(200.),
                grid_row: GridPlacement::start(row),
                grid_column: GridPlacement::start(2),
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
            Outline {
                width: Val::Px(1.),
                color: Color::WHITE,
                ..Default::default()
            },
        ));

        if password {
            input.insert(TextInputPasswordMask::default());
        }

        commands.spawn((
            Node {
                display: Display::Grid,
                width: Val::Px(200.),
                overflow: Overflow::clip(),
                grid_row: GridPlacement::start(row),
                grid_column: GridPlacement::start(3),
                justify_content: JustifyContent::End,
                ..Default::default()
            },
            children![(Text::new(format!("..3, {row}")), TextColor(Color::WHITE),)],
        ));

        commands.spawn((
            Node {
                display: Display::Grid,
                width: Val::Px(200.),
                overflow: Overflow::clip(),
                grid_row: GridPlacement::start(row),
                grid_column: GridPlacement::start(4),
                justify_content: JustifyContent::End,
                ..Default::default()
            },
            children![(Text::new(format!("..4, {row}")), TextColor(Color::WHITE),)],
        ));
    }
}

fn spawn_row(
    commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    grid_row: GridPlacement,
    label: &str,
    input_filter: TextInputFilter,
    is_password: bool,
) {
    commands.spawn((
        Node {
            display: Display::Grid,
            grid_row,
            grid_column: GridPlacement::start(1),
            justify_content: JustifyContent::End,
            ..Default::default()
        },
        children![(Text::new(label), TextColor(YELLOW.into()),)],
    ));

    commands.spawn((
        Node {
            display: Display::Grid,
            width: Val::Px(200.),
            overflow: Overflow::clip(),
            grid_row,
            grid_column: GridPlacement::start(3),
            justify_content: JustifyContent::End,
            ..Default::default()
        },
        children![(Text::new(format!("contents")), TextColor(Color::WHITE),)],
    ));

    let mut input = commands.spawn((
        TextInput {
            justify: Justify::Left,
        },
        input_filter,
        TextColor(Color::WHITE),
        TabIndex(0),
        Node {
            width: Val::Px(200.),
            grid_row,
            grid_column: GridPlacement::start(2),
            ..Default::default()
        },
        BackgroundColor(Color::BLACK),
        Outline {
            width: Val::Px(1.),
            color: Color::WHITE,
            ..Default::default()
        },
    ));

    if is_password {
        input.insert(TextInputPasswordMask::default());
    }

    commands.spawn((
        Node {
            display: Display::Grid,
            width: Val::Px(200.),
            overflow: Overflow::clip(),
            grid_row,
            grid_column: GridPlacement::start(4),
            justify_content: JustifyContent::End,
            ..Default::default()
        },
        children![(Text::new(format!("sub")), TextColor(Color::WHITE),)],
    ));
}

fn spawn_font_button(
    commands: &mut RelatedSpawnerCommands<'_, ChildOf>,
    font: Handle<Font>,
    label: &str,
) {
    let on_activate = commands.commands().register_system(
        move |_: In<Activate>, mut query: Query<&mut TextFont, With<DemoInput>>| {
            for mut text_input_font in query.iter_mut() {
                text_input_font.font = font.clone();
            }
        },
    );

    commands.spawn((
        Node {
            padding: UiRect::all(Val::Px(5.)),
            border: UiRect::all(Val::Px(2.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        CoreButton {
            on_activate: Callback::System(on_activate),
        },
        Hovered::default(),
        TabIndex(0),
        BorderColor::all(Color::BLACK),
        BorderRadius::MAX,
        BackgroundColor(NAVY.into()),
        children![(Text::new(label),)],
    ));
}
