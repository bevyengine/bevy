//! Demonstrates how to use `PickingInteraction` without using events and observers.

use bevy::{
    picking::focus::{PickingInteraction, PressedButtons},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_sprite, picking))
        .run();
}

// Move the sprite for variety.
fn move_sprite(time: Res<Time>, mut sprite: Query<&mut Transform, With<Sprite>>) {
    let t = time.elapsed_secs() * 0.1;
    for mut transform in &mut sprite {
        let new = Vec2 {
            x: 50.0 * ops::sin(t),
            y: 50.0 * ops::sin(t * 2.0),
        };
        transform.translation.x = new.x;
        transform.translation.y = new.y;
    }
}

// Display the current picking state.
fn picking(sprite: Query<(&PickingInteraction, &Children)>, mut text: Query<&mut Text2d>) {
    for (interaction, children) in sprite.iter() {
        let mut iter = text.iter_many_mut(children);
        while let Some(mut text) = iter.fetch_next() {
            match interaction {
                PickingInteraction::Pressed(pressed_buttons)
                    if pressed_buttons.contains(PressedButtons::PRIMARY) =>
                {
                    text.0 = "Left Clicked!".into();
                }
                PickingInteraction::Pressed(pressed_buttons)
                    if pressed_buttons.contains(PressedButtons::SECONDARY) =>
                {
                    text.0 = "Right Clicked!".into();
                }
                PickingInteraction::Pressed(pressed_buttons)
                    if pressed_buttons.contains(PressedButtons::TOUCH) =>
                {
                    text.0 = "Touched!".into();
                }
                // We choose to ignore other mouse buttons like MMB,
                // in this pattern we treat them as hover.
                PickingInteraction::Hovered | PickingInteraction::Pressed(..) => {
                    text.0 = "Hovered!".into();
                }
                PickingInteraction::None => {
                    text.0 = "Hover Me!".into();
                }
            }
        }
    }
}

/// Set up the scene.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands
        .spawn((
            Sprite {
                custom_size: Some(Vec2::new(200., 50.)),
                color: Color::BLACK,
                ..Default::default()
            },
            PickingInteraction::None,
        ))
        .with_children(|s| {
            s.spawn((Text2d::new("Hover Me!"), Transform::from_xyz(0., 0., 1.)));
        });
}
