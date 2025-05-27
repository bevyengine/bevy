//! This example shows how to render 2D objects on top of Bevy UI, by using a second camera with a higher `order` than the UI camera.

use bevy::{color::palettes::tailwind, prelude::*, render::view::RenderLayers};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_sprite)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // the default camera. we explicitly set that this is the Ui render camera. you can also use `UiTargetCamera` on each entity.
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // the second camera, with a higher order, will be drawn after the first camera. we will render to this camera to draw on top of the UI.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            // dont draw anything in the background, to see the previous cameras.
            clear_color: ClearColorConfig::None,
            ..default()
        },
        // this camera will only render entity which are on the same render layer.
        RenderLayers::layer(1),
    ));

    commands
        .spawn((
            // here we could also use a `UiTargetCamera` component instead of the general `IsDefaultUiCamera`
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                display: Display::Flex,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(tailwind::ROSE_400.into()),
        ))
        .with_children(|p| {
            p.spawn((
                Node {
                    height: Val::Percent(30.),
                    width: Val::Percent(20.),
                    min_height: Val::Px(150.),
                    min_width: Val::Px(150.),
                    border: UiRect::all(Val::Px(2.)),
                    ..default()
                },
                BorderRadius::all(Val::Percent(25.0)),
                BorderColor(Color::WHITE),
            ));
        });

    // this 2d object, will be rendered on the second camera, on top of the default camera where the ui is rendered.
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
    // can use 2d concept things like Transform
    sprite.rotation *=
        Quat::from_rotation_z(time.delta_secs() * 0.5) * Quat::from_rotation_y(time.delta_secs());
}
