//! Showcases sprite 9 slice scaling
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: (1350.0, 700.0).into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
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
            ..default()
        },
        scale_mode: ImageScaleMode::Sliced(TextureSlicer {
            border: BorderRect::square(slice_border),
            center_scale_mode: SliceScaleMode::Stretch,
            ..default()
        }),
        ..default()
    });

    // Scaled sliced sprite
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 450.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(100.0, 200.0)),
            ..default()
        },
        scale_mode: ImageScaleMode::Sliced(TextureSlicer {
            border: BorderRect::square(slice_border),
            center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.5 },
            sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
            ..default()
        }),
        ..default()
    });

    // Scaled sliced sprite horizontally
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 700.0),
        texture: texture_handle.clone(),
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0, 200.0)),
            ..default()
        },
        scale_mode: ImageScaleMode::Sliced(TextureSlicer {
            border: BorderRect::square(slice_border),
            center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
            sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.3 },
            ..default()
        }),
        ..default()
    });

    // Scaled sliced sprite horizontally with max scale
    commands.spawn(SpriteBundle {
        transform: Transform::from_translation(base_pos + Vec3::X * 1050.0),
        texture: texture_handle,
        sprite: Sprite {
            custom_size: Some(Vec2::new(300.0, 200.0)),
            ..default()
        },
        scale_mode: ImageScaleMode::Sliced(TextureSlicer {
            border: BorderRect::square(slice_border),
            center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.1 },
            sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
            max_corner_scale: 0.2,
        }),
        ..default()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    // Load textures
    let handle_1 = asset_server.load("textures/slice_square.png");
    let handle_2 = asset_server.load("textures/slice_square_2.png");

    spawn_sprites(
        &mut commands,
        handle_1,
        Vec3::new(-600.0, 200.0, 0.0),
        200.0,
    );
    spawn_sprites(
        &mut commands,
        handle_2,
        Vec3::new(-600.0, -200.0, 0.0),
        80.0,
    );

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextStyle {
        font: font.clone(),
        font_size: 30.0,
        color: Color::WHITE,
    };
    let alignment = TextAlignment::Center;
    // Spawn text
    commands.spawn(Text2dBundle {
        text: Text::from_section("Original texture", style.clone()).with_alignment(alignment),
        transform: Transform::from_xyz(-550.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(Text2dBundle {
        text: Text::from_section("Stretched texture", style.clone()).with_alignment(alignment),
        transform: Transform::from_xyz(-400.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(Text2dBundle {
        text: Text::from_section("Stretched and sliced", style.clone()).with_alignment(alignment),
        transform: Transform::from_xyz(-250.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(Text2dBundle {
        text: Text::from_section("Sliced and Tiled", style.clone()).with_alignment(alignment),
        transform: Transform::from_xyz(-100.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(Text2dBundle {
        text: Text::from_section("Sliced and Tiled", style.clone()).with_alignment(alignment),
        transform: Transform::from_xyz(150.0, 0.0, 0.0),
        ..default()
    });
    commands.spawn(Text2dBundle {
        text: Text::from_section("Sliced and Tiled with corner constraint", style.clone())
            .with_alignment(alignment),
        transform: Transform::from_xyz(550.0, 0.0, 0.0),
        ..default()
    });
}
