//! multiple text inputs example

use bevy::color::palettes::css::GREEN;
use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::YELLOW;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::prelude::*;
use bevy::text::TextInputFilter;
use bevy::ui::widget::LineInputNode;
use bevy_ecs::relationship::RelatedSpawnerCommands;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/Orbitron-Medium.ttf");

    // UI camera
    commands.spawn(Camera2d);

    commands.spawn(Text::new("HELLO!"));

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                Text::new("HELLO!"),
                TextFont {
                    font: font.clone(),
                    font_size: 30.,
                    line_height: bevy::text::LineHeight::RelativeToFont(1.5),
                    ..Default::default()
                },
                TextColor(Color::WHITE),
            ));
            commands
                .spawn((
                    Node {
                        display: Display::Grid,
                        width: Val::Px(400.),
                        border: UiRect::all(Val::Px(5.)),
                        padding: UiRect::all(Val::Px(5.)),
                        row_gap: Val::Px(20.),
                        column_gap: Val::Px(20.),
                        ..default()
                    },
                    BorderColor::all(YELLOW.into()),
                    BackgroundColor(GREEN.into()),
                ))
                .with_children(|commands| {
                    inputs_grid(commands, font.clone());
                });
        });
}

fn inputs_grid(commands: &mut RelatedSpawnerCommands<ChildOf>, font: Handle<Font>) {
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
            Text::new(label),
            TextFont {
                font: font.clone(),
                font_size: 30.,
                line_height: bevy::text::LineHeight::RelativeToFont(1.5),
                ..Default::default()
            },
            TextColor(Color::WHITE),
            Node {
                display: Display::Grid,
                width: Val::Px(100.),
                grid_row: GridPlacement::start(n as i16 + 1),
                grid_column: GridPlacement::start(1),
                ..Default::default()
            },
        ));

        commands.spawn((
            LineInputNode {
                justify: Justify::Left,
            },
            input_filter,
            TextFont {
                font: font.clone(),
                font_size: 30.,
                line_height: bevy::text::LineHeight::RelativeToFont(1.5),
                ..Default::default()
            },
            TextColor(Color::WHITE),
            TabIndex(0),
            Node {
                grid_row: GridPlacement::start(n as i16 + 1),
                grid_column: GridPlacement::start(2),
                ..Default::default()
            },
        ));
    }
}
