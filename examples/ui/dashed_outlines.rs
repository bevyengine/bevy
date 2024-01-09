//! Demonstrates how to create a node with a dashed outline

use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_grow: 1.,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            background_color: Color::BLACK.into(),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Percent(50.),
                        height: Val::Percent(50.),
                        border_radius: BorderRadius::all(Val::Percent(10.)),
                        ..Default::default()
                    },
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                },
                Outline {
                    width: Val::Px(10.),
                    offset: Val::Px(5.),
                    color: Color::WHITE,
                },
                OutlineStyle::Dashed {
                    dash_length: Val::Px(40.),
                    break_length: Val::Px(40.),
                },
            ));
        });
}

fn update(time: Res<Time>, mut query: Query<&mut Style, With<Outline>>) {
    let d = (0.25 * time.elapsed_seconds()).sin() * 25.;
    for mut style in &mut query {
        style.width = Val::Percent(d + 50.);
        style.height = Val::Percent(d + 50.);
    }
}
