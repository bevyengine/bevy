//! multiple text inputs example

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::YELLOW;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;
use bevy::text::TextInputFilter;
use bevy::ui::widget::TextInput;
use bevy_ecs::relationship::RelatedSpawnerCommands;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .insert_resource(UiScale(2.))
        .run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        })
        .with_children(|commands| {
            commands
                .spawn((
                    Node {
                        display: Display::Grid,
                        border: UiRect::all(Val::Px(5.)),
                        padding: UiRect::all(Val::Px(5.)),
                        row_gap: Val::Px(20.),
                        column_gap: Val::Px(20.),
                        ..default()
                    },
                    BorderColor::all(YELLOW.into()),
                    BackgroundColor(NAVY.into()),
                ))
                .with_children(|commands| {
                    inputs_grid(commands);
                });
        });
}

fn inputs_grid(commands: &mut RelatedSpawnerCommands<ChildOf>) {
    for (n, (label, input_filter)) in [
        ("alphanumeric", TextInputFilter::Alphanumeric),
        ("decimal", TextInputFilter::Decimal),
        ("hex", TextInputFilter::Hex),
        ("integer", TextInputFilter::Integer),
        (
            "not bevy",
            TextInputFilter::custom(|text| !text.contains("bevy")),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        commands.spawn((
            Node {
                display: Display::Grid,
                grid_row: GridPlacement::start(n as i16 + 1),
                grid_column: GridPlacement::start(1),
                justify_content: JustifyContent::End,
                ..Default::default()
            },
            children![(Text::new(label), TextColor(YELLOW.into()),)],
        ));

        commands.spawn((
            TextInput {
                justify: Justify::Left,
            },
            input_filter,
            TextColor(Color::WHITE),
            TabIndex(0),
            Node {
                width: Val::Px(200.),
                grid_row: GridPlacement::start(n as i16 + 1),
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

        commands.spawn((
            Node {
                display: Display::Grid,
                width: Val::Px(200.),
                overflow: Overflow::clip(),
                grid_row: GridPlacement::start(n as i16 + 1),
                grid_column: GridPlacement::start(3),
                justify_content: JustifyContent::End,
                ..Default::default()
            },
            children![(Text::new(".."), TextColor(Color::WHITE),)],
        ));
    }
}
