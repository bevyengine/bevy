//! minimal text input example

use bevy::clipboard::ClipboardPlugin;
use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::input_focus::tab_navigation::TabGroup;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::LineHeight;
use bevy::text::TextInputValue;
use bevy::ui::widget::TextField;
use bevy::ui::widget::TextInputPlugin;
use bevy::window::WindowResolution;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(500., 500.),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            TextInputPlugin,
            InputDispatchPlugin,
            TabNavigationPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2d);
    let id = commands
        .spawn((
            TextField::default(),
            TabIndex(0),
            TextColor(RED.into()),
            TextInputValue::new("ماذا يعني لوريم إيبسوم الم؟"),
            TextFont {
                font: asset_server.load("fonts/NotoNaskhArabic-Medium.ttf"),
                font_size: 30.,
                line_height: LineHeight::Px(50.),
                ..Default::default()
            },
            Node {
                width: Val::Percent(50.),
                ..default()
            },
            BackgroundColor(NAVY.into()),
        ))
        .id();

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(10.),
                ..Default::default()
            },
            TabGroup::default(),
        ))
        .add_child(id);

    commands.insert_resource(InputFocus(Some(id)));
}
