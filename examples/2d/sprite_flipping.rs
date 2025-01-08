//! Displays a single [`Sprite`], created from an image, but flipped on one axis.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a camera
    commands.spawn(Camera2d);

    // Create a minimal UI explaining how to interact with the example
    commands.spawn((
        Text::new("Sprite with flipped image on one axis."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.spawn(Sprite {
        image: asset_server.load("branding/bevy_bird_dark.png"),
        // Flip the logo to the left
        flip_x: true,
        // And don't flip it upside-down ( the default )
        flip_y: false,
        ..Default::default()
    });
}
