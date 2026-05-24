//! Virtual keyboard example

use bevy::{
    color::palettes::css::NAVY,
    feathers::{
        controls::{VirtualKeyPressed, VirtualKeyboard},
        dark_theme::create_dark_theme,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FeathersPlugins))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, scene.spawn())
        .run();
}

fn on_virtual_key_pressed(virtual_key_pressed: On<VirtualKeyPressed<&'static str>>) {
    println!("key pressed: {}", virtual_key_pressed.key);
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, keyboard()]
}

fn keyboard() -> impl Scene {
    let keys = [
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", ".", ","],
        vec!["Q", "W", "E", "R", "T", "Y", "U", "I", "O", "P"],
        vec!["A", "S", "D", "F", "G", "H", "J", "K", "L", "'"],
        vec!["Z", "X", "C", "V", "B", "N", "M", "-", "/"],
        vec!["space", "enter", "backspace"],
        vec!["left", "right", "up", "down", "home", "end"],
    ];

    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::End,
            justify_content: JustifyContent::Center,
        }
        Children [(
            Node {
                flex_direction: FlexDirection::Column,
                border: px(5),
                row_gap: px(5),
                padding: px(5),
                align_items: AlignItems::Center,
                margin: px(25),
                border_radius: BorderRadius::all(px(10)),
            }
            BackgroundColor(NAVY)
            BorderColor::all(Color::WHITE)
            Children [
                Text("virtual keyboard"),
                (
                    :VirtualKeyboard::<&str> { @keys: keys }
                    on(on_virtual_key_pressed)
                )
            ]
        )]
    }
}
