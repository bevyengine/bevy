//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(SliderBundle {
            style: Style {
                size: Size::new(Val::Px(200.), Val::Px(20.)),
                // center slider
                margin: UiRect::all(Val::Auto),
                ..default()
            },
            background_color: Color::rgb(0.8, 0.8, 0.8).into(),
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(SliderHandleBundle {
                style: Style {
                    size: Size::new(Val::Px(15.), Val::Px(20.)),
                    ..default()
                },
                ..default()
            });
        });
    commands.spawn(TextBundle::from_section(
        "0",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: Color::rgb(0.9, 0.9, 0.9),
        },
    ));
}
