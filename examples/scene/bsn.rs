//! This example demonstrates how to use BSN to compose scenes.
use bevy::{prelude::*, text::FontSourceTemplate};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(world: &mut World) -> Result {
    world.spawn_scene_list(bsn_list![Camera2d, ui()])?;
    Ok(())
}

fn ui() -> impl Scene {
    bsn! {
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: Val::Px(5.),
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

fn button(label: &'static str) -> impl Scene {
    bsn! {
        Button
        Node {
            width: Val::Px(150.0),
            height: Val::Px(65.0),
            border: UiRect::all(Val::Px(5.0)),
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
                font_size: FontSize::Px(33.0),
            }
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
    }
}
