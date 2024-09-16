//! This example shows how to create a node with a shadow

use bevy::color::palettes::css::LIGHT_CORAL;
use bevy::prelude::*;
use bevy::winit::WinitSettings;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: BackgroundColor(LIGHT_CORAL.into()),
            ..Default::default()
        })
        .with_children(|commands| {
            commands.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(300.),
                        height: Val::Px(200.),
                        ..default()
                    },
                    background_color: BackgroundColor(Color::WHITE),
                    ..Default::default()
                },
                BoxShadow {
                    color: Color::BLACK,
                    x_offset: Val::Percent(50.),
                    y_offset: Val::Percent(50.),
                    blur_radius: Val::Px(5.),
                    ..Default::default()
                },
            ));
        });
}
