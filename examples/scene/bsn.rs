//! This example demonstrates how to use BSN to compose scenes.
use bevy::{prelude::*, text::FontSourceTemplate};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, scene.spawn())
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![Camera2d, ui()]
}

fn ui() -> impl Scene {
    bsn! {
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(5),
        }
        Children [
            (
                button("Ok")
                on(|_event: On<Pointer<Press>>| println!("Ok pressed!"))
            ),
            (
                button("Cancel")
                on(|_event: On<Pointer<Press>>| println!("Cancel pressed!"))
                BackgroundColor(Color::srgb(0.4, 0.15, 0.15))
            ),
        ]
    }
}

fn button(label: &str) -> impl Scene {
    bsn! {
        Button
        Node {
            width: px(150),
            height: px(65),
            border: px(5),
            border_radius: BorderRadius::MAX,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        Children [(
            Text(label)
            TextFont {
                font: FontSourceTemplate::Handle("fonts/FiraSans-Bold.ttf"),
                font_size: px(33.0),
            }
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
    }
}
