//! Showcases sprite 9 slice scaling

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                width: 1350.0,
                height: 700.0,
                ..default()
            },
            ..default()
        }))
        .add_startup_system(setup)
        .run();
}

fn spawn_sprites(
    commands: &mut Commands,
    texture_handle: Handle<Image>,
    base_pos: Vec3,
    slice_border: f32,
) {
    // Reference sprite
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::splat(100.0)),
            ..default()
        },
        ..default()
    });

    // Scaled regular sprite
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 150.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(100.0, 200.0)),
            ..default()
        },
        ..default()
    });

    // Stretched Scaled sliced sprite
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 300.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(100.0, 200.0)),
            draw_mode: SpriteDrawMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Stretch,
                ..default()
            }),
            ..default()
        },
        ..default()
    });

    // Scaled sliced sprite
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 450.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(100.0, 200.0)),
            draw_mode: SpriteDrawMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.5 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                ..default()
            }),
            ..default()
        },
        ..default()
    });

    // Scaled sliced sprite horizontally
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 700.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0, 200.0)),
            draw_mode: SpriteDrawMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.3 },
                ..default()
            }),
            ..default()
        },
        ..default()
    });

    // Scaled sliced sprite horizontally with max scale
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 1050.0),
        texture: texture_handle,
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0, 200.0)),
            draw_mode: SpriteDrawMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.1 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                max_corner_scale: 0.2,
            }),
            ..default()
        },
        ..default()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    // Load textures
    let handle_1 = asset_server.load("textures/slice_square.png");
    let handle_2 = asset_server.load("textures/slice_square_2.png");
    let handle_3 = asset_server.load("textures/slice_sprite.png");

    spawn_sprites(
        &mut commands,
        handle_1,
        Vec3::new(-600.0, 200.0, 0.0),
        200.0,
    );
    spawn_sprites(&mut commands, handle_2, Vec3::new(-600.0, 0.0, 0.0), 80.0);
    spawn_sprites(
        &mut commands,
        handle_3,
        Vec3::new(-600.0, -200.0, 0.0),
        55.0,
    );
}
