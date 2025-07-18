//! minimal text input example

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::ui::widget::TextBox;
use bevy::ui::widget::TextCursorStyle;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);

    let tid = commands.spawn(Text::new("hello")).id();
    let id = commands
        .spawn((
            TextBox::default(),
            TabIndex(0),
            TextColor(RED.into()),
            TextFont {
                font_size: 35.,
                ..Default::default()
            },
            Node {
                width: Val::Percent(50.),
                height: Val::Percent(50.),
                ..default()
            },
            BackgroundColor(NAVY.into()),
        ))
        .id();

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(10.),
            ..Default::default()
        })
        .add_child(id)
        .add_child(tid);

    commands.insert_resource(InputFocus(Some(id)));
}
