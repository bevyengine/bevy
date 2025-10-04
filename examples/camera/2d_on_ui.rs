//! This example shows how to render 2D objects on top of Bevy UI, by using a second camera with a higher `order` than the UI camera.

use bevy::{camera::visibility::RenderLayers, color::palettes::tailwind, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_sprite)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // The default camera. `IsDefaultUiCamera` makes this the default camera to render UI elements to. Alternatively, you can add the `UiTargetCamera` component to root UI nodes to define which camera they should be rendered to.
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // The second camera. The higher order means that this camera will be rendered after the first camera. We will render to this camera to draw on top of the UI.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            // Don't draw anything in the background, to see the previous camera.
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // This camera will only render entities which are on the same render layer.
        RenderLayers::layer(1),
    ));

    commands.spawn((
        // We could also use a `UiTargetCamera` component here instead of the general `IsDefaultUiCamera`.
        Node {
            width: percent(100),
            height: percent(100),
            display: Display::Flex,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(tailwind::ROSE_400.into()),
        children![(
            Node {
                height: percent(30),
                width: percent(20),
                min_height: px(150),
                min_width: px(150),
                border: UiRect::all(px(2)),
                ..default()
            },
            BorderRadius::all(percent(25)),
            BorderColor::all(Color::WHITE),
        )],
    ));

    // This 2D object will be rendered on the second camera, on top of the default camera where the UI is rendered.
    commands.spawn((
        Sprite {
            image: asset_server.load("textures/rpg/chars/sensei/sensei.png"),
            custom_size: Some(Vec2::new(100., 100.)),
            ..default()
        },
        RenderLayers::layer(1),
    ));
}

fn rotate_sprite(time: Res<Time>, mut sprite: Single<&mut Transform, With<Sprite>>) {
    // Use any of the regular 2D rendering features, for example rotating a sprite via its `Transform`.
    sprite.rotation *=
        Quat::from_rotation_z(time.delta_secs() * 0.5) * Quat::from_rotation_y(time.delta_secs());
}
