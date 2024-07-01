//! Showcases sprite 9 slice scaling and tiling features, enabling usage of
//! sprites in multiple resolutions while keeping it in proportion
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn spawn_sprites(
    commands: &mut Commands,
    texture_handle: Handle<Image>,
    mut position: Vec3,
    slice_border: f32,
    style: TextStyle,
    gap: f32,
) {
    let cases = [
        // Reference sprite
        ("Original", style.clone(), Vec2::splat(100.0), None),
        // Scaled regular sprite
        ("Stretched", style.clone(), Vec2::new(100.0, 200.0), None),
        // Stretched Scaled sliced sprite
        (
            "With Slicing",
            style.clone(),
            Vec2::new(100.0, 200.0),
            Some(ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Stretch,
                ..default()
            })),
        ),
        // Scaled sliced sprite
        (
            "With Tiling",
            style.clone(),
            Vec2::new(100.0, 200.0),
            Some(ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.5 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                ..default()
            })),
        ),
        // Scaled sliced sprite horizontally
        (
            "With Tiling",
            style.clone(),
            Vec2::new(300.0, 200.0),
            Some(ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.3 },
                ..default()
            })),
        ),
        // Scaled sliced sprite horizontally with max scale
        (
            "With Corners Constrained",
            style,
            Vec2::new(300.0, 200.0),
            Some(ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.1 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                max_corner_scale: 0.2,
            })),
        ),
    ];

    for (label, text_style, size, scale_mode) in cases {
        position.x += 0.5 * size.x;
        let mut cmd = commands.spawn(SpriteBundle {
            transform: Transform::from_translation(position),
            texture: texture_handle.clone(),
            sprite: Sprite {
                custom_size: Some(size),
                ..default()
            },
            ..default()
        });
        if let Some(scale_mode) = scale_mode {
            cmd.insert(scale_mode);
        }
        cmd.with_children(|builder| {
            builder.spawn(Text2dBundle {
                text: Text::from_section(label, text_style).with_justify(JustifyText::Center),
                transform: Transform::from_xyz(0., -0.5 * size.y - 10., 0.0),
                text_anchor: bevy::sprite::Anchor::TopCenter,
                ..default()
            });
        });
        position.x += 0.5 * size.x + gap;
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    let font = asset_server.load("fonts/FiraSans-Bold.ttf");
    let style = TextStyle {
        font: font.clone(),
        ..default()
    };

    // Load textures
    let handle_1 = asset_server.load("textures/slice_square.png");
    let handle_2 = asset_server.load("textures/slice_square_2.png");

    spawn_sprites(
        &mut commands,
        handle_1,
        Vec3::new(-600.0, 200.0, 0.0),
        200.0,
        style.clone(),
        40.,
    );

    spawn_sprites(
        &mut commands,
        handle_2,
        Vec3::new(-600.0, -200.0, 0.0),
        80.0,
        style,
        40.,
    );
}
