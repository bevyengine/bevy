//! minimal text input example

use bevy::color::palettes::css::LIGHT_BLUE;
use bevy::color::palettes::css::NAVY;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::input_focus::tab_navigation::TabNavigationPlugin;
use bevy::input_focus::InputDispatchPlugin;
use bevy::input_focus::InputFocus;
use bevy::prelude::*;
use bevy::text::Prompt;
use bevy::text::PromptColor;
use bevy::text::TextSelectionBlockColor;
use bevy::ui::widget::TextBox;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InputDispatchPlugin, TabNavigationPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // UI camera
    commands.spawn(Camera2d);

    let tid = commands.spawn(Text::new("TextBox example")).id();
    let id = commands
        .spawn((
            TextBox::default(),
            Prompt::new("please type here.."),
            PromptColor::new(LIGHT_BLUE),
            TextSelectionBlockColor::new(LIGHT_BLUE),
            TabIndex(0),
            TextColor(Color::WHITE),
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
        .add_child(tid)
        .add_child(id);

    commands.insert_resource(InputFocus(Some(id)));
}
