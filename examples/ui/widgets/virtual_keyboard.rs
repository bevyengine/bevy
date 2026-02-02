//! Virtual keyboard example

use bevy::{
    color::palettes::css::NAVY,
    feathers::{
        controls::{virtual_keyboard, VirtualKeyPressed},
        dark_theme::create_dark_theme,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
    ui_widgets::observe,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, setup)
        .run();
}

fn on_virtual_key_pressed(virtual_key_pressed: On<VirtualKeyPressed<&'static str>>) {
    println!("key pressed: {}", virtual_key_pressed.key);
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2d);

    let layout = [
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ".", ","],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L", "'"],
        vec!["Z", "X", "C", "V", "B", "N", "M", "-", "/"],
        vec!["space", "enter", "backspace"],
        vec!["left", "right", "up", "down", "home", "end"],
    ];

    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            Node {
                flex_direction: FlexDirection::Column,
                border: px(5).into(),
                row_gap: px(5),
                padding: px(5).into(),
                align_items: AlignItems::Center,
                margin: px(25).into(),
                border_radius: BorderRadius::all(px(10)),
                ..Default::default()
            },
            BackgroundColor(NAVY.into()),
            BorderColor::all(Color::WHITE),
            children![
                Text::new("virtual keyboard"),
                (
                    virtual_keyboard(layout.into_iter()),
                    observe(on_virtual_key_pressed)
                )
            ]
        )],
    ));
}
