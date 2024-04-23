//! Constructs a large entity composed of multiple sprites

use bevy::input::common_conditions::input_pressed;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_systems(Startup, setup)
        // the user can press the arrow keys to translate the parent entity in the window
        .add_systems(
            Update,
            (
                translate::<-5, 0>.run_if(input_pressed(KeyCode::ArrowLeft)),
                translate::<5, 0>.run_if(input_pressed(KeyCode::ArrowRight)),
                translate::<0, 5>.run_if(input_pressed(KeyCode::ArrowUp)),
                translate::<0, -5>.run_if(input_pressed(KeyCode::ArrowDown)),
            ),
        )
        .run();
}

#[derive(Component)]
struct ParentSprite;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // the entity is composed of four tiles arranged in a square
    commands
        .spawn((ParentSprite, SpatialBundle::default()))
        .with_children(|parent| {
            const SIZE: f32 = 16.0; // tiles are 16x16...
            const SCALE: f32 = 5.0; // ...scale up to 80x80

            // for positioning the tiles
            let offset = SIZE * SCALE / 2.0;

            parent.spawn(SpriteBundle {
                texture: asset_server.load("textures/rpg/tiles/generic-rpg-tile59.png"),
                transform: Transform {
                    translation: Vec3::new(-offset, offset, 0.0),
                    scale: Vec3::splat(SCALE),
                    ..default()
                },
                ..default()
            });

            parent.spawn(SpriteBundle {
                texture: asset_server.load("textures/rpg/tiles/generic-rpg-tile60.png"),
                transform: Transform {
                    translation: Vec3::new(offset, offset, 0.0),
                    scale: Vec3::splat(SCALE),
                    ..default()
                },
                ..default()
            });

            parent.spawn(SpriteBundle {
                texture: asset_server.load("textures/rpg/tiles/generic-rpg-tile61.png"),
                transform: Transform {
                    translation: Vec3::new(-offset, -offset, 0.0),
                    scale: Vec3::splat(SCALE),
                    ..default()
                },
                ..default()
            });

            parent.spawn(SpriteBundle {
                texture: asset_server.load("textures/rpg/tiles/generic-rpg-tile62.png"),
                transform: Transform {
                    translation: Vec3::new(offset, -offset, 0.0),
                    scale: Vec3::splat(SCALE),
                    ..default()
                },
                ..default()
            });
        });

    // create a minimal UI explaining how to interact with the example
    commands.spawn(TextBundle {
        text: Text::from_section(
            "Use the arrow keys to move the block around the window",
            TextStyle {
                font_size: 20.0,
                ..default()
            },
        ),
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ..default()
    });
}

fn translate<const X: i8, const Y: i8>(mut parent: Query<&mut Transform, With<ParentSprite>>) {
    let mut parent = parent.single_mut();
    parent.translation.x += X as f32;
    parent.translation.y += Y as f32;
}
