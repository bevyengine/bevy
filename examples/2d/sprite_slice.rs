//! Showcases sprite 9 slice scaling and tiling features, enabling usage of
//! sprites in multiple resolutions while keeping it in proportion
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
    mut position: Vec3,
    slice_border: f32,
    style: TextStyle,
    gap: f32,
) {
    let cases = [
        // Reference sprite
        (
            "Original texture",
            style.clone(),
            Vec2::splat(100.0),
            ImageScaleMode::default(),
        ),
        // Scaled regular sprite
        (
            "Stretched texture",
            style.clone(),
            Vec2::new(100.0, 200.0),
            ImageScaleMode::default(),
        ),
        // Stretched Scaled sliced sprite
        (
            "Stretched and sliced",
            style.clone(),
            Vec2::new(100.0, 200.0),
            ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Stretch,
                ..default()
            }),
        ),
        // Scaled sliced sprite
        (
            "Sliced and Tiled",
            style.clone(),
            Vec2::new(100.0, 200.0),
            ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.5 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                ..default()
            }),
        ),
        // Scaled sliced sprite horizontally
        (
            "Sliced and Tiled",
            style.clone(),
            Vec2::new(300.0, 200.0),
            ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.3 },
                ..default()
            }),
        ),
        // Scaled sliced sprite horizontally with max scale
        (
            "Sliced and Tiled with corner constraint",
            style,
            Vec2::new(300.0, 200.0),
            ImageScaleMode::Sliced(TextureSlicer {
                border: BorderRect::square(slice_border),
                center_scale_mode: SliceScaleMode::Tile { stretch_value: 0.1 },
                sides_scale_mode: SliceScaleMode::Tile { stretch_value: 0.2 },
                max_corner_scale: 0.2,
            }),
        ),
    ];

    for (label, text_style, size, scale_mode) in cases {
        position.x += 0.5 * size.x;
        commands
            .spawn(SpriteBundle {
                transform: Transform::from_translation(position),
                texture: texture_handle.clone(),
                sprite: Sprite {
                    custom_size: Some(size),
                    ..default()
                },
                scale_mode,
                ..default()
            })
            .with_children(|builder| {
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
        font_size: 16.0,
        color: Color::WHITE,
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
        50.,
    );

    spawn_sprites(
        &mut commands,
        handle_2,
        Vec3::new(-600.0, -200.0, 0.0),
        80.0,
        style,
        50.,
    );
}
