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

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,

            ..Default::default()
        },
        children![(
            Node {
                width: Val::Px(400.),
                border: UiRect::all(Val::Px(5.)),
                padding: UiRect::all(Val::Px(5.)),
                row_gap: Val::Px(20.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BorderColor::all(YELLOW.into()),
            BackgroundColor(GREEN.into()),
            children![
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextFont {
                        font: asset_server.load("fonts/Orbitron-Medium.ttf"),
                        font_size: 30.,
                        ..Default::default()
                    },
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextFont {
                        font: asset_server.load("fonts/Orbitron-Medium.ttf"),
                        font_size: 30.,
                        line_height: bevy::text::LineHeight::RelativeToFont(2.),
                        ..Default::default()
                    },
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextFont {
                        font: asset_server.load("fonts/Orbitron-Medium.ttf"),
                        ..Default::default()
                    },
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextInputFilter::Integer,
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextInputFilter::Decimal,
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextInputFilter::Hex,
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextInputFilter::Alphanumeric,
                    BackgroundColor(NAVY.into()),
                ),
                (
                    LineInputNode::default(),
                    TabIndex(0),
                    TextInputFilter::custom(|text: &str| !text.contains('b')),
                    BackgroundColor(NAVY.into()),
                ),
            ],
        )],
    ));
}
