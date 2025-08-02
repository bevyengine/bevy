#![allow(unused)]

//! This example illustrates constructing ui scenes
use bevy::{
    ecs::template::template,
    prelude::*,
    scene2::prelude::{Scene, SpawnScene, *},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(world: &mut World) {
    world.spawn(Camera2d);
    world.spawn_scene(ui());
}

fn ui() -> impl Scene {
    bsn! {
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
        } [
            :button("Button")
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
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
        }
        BorderColor::from(Color::BLACK)
        BorderRadius::MAX
        BackgroundColor(Color::srgb(0.15, 0.15, 0.15))
        on(|event: On<Pointer<Press>>| {
            println!("pressed");
        })
        [(
            Text(label)
            // The `template` wrapper can be used for types that can't implement or don't yet have a template
            template(|context| {
                Ok(TextFont {
                    font: context
                        .resource::<AssetServer>()
                        .load("fonts/FiraSans-Bold.ttf"),
                    font_size: 33.0,
                    ..default()
                })
            })
            TextColor(Color::srgb(0.9, 0.9, 0.9))
            TextShadow
        )]
    }
}
