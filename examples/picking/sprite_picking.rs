//! Demonstrates picking for sprites and sprite atlases.
//! By default, the sprite picking backend considers a sprite only when a pointer is over an opaque pixel.

use bevy::{prelude::*, sprite::Anchor};
use std::fmt::Debug;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (setup, setup_atlas))
        .add_systems(Update, (move_sprite, animate_sprite))
        .run();
}

fn move_sprite(
    time: Res<Time>,
    mut sprite: Query<&mut Transform, (Without<Sprite>, With<Children>)>,
) {
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

/// Set up a scene that tests all sprite anchor types.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let len = 128.0;
    let sprite_size = Vec2::splat(len / 2.0);

    commands
        .spawn((Transform::default(), Visibility::default()))
        .with_children(|commands| {
            for (anchor_index, anchor) in [
                Anchor::TOP_LEFT,
                Anchor::TOP_CENTER,
                Anchor::TOP_RIGHT,
                Anchor::CENTER_LEFT,
                Anchor::CENTER,
                Anchor::CENTER_RIGHT,
                Anchor::BOTTOM_LEFT,
                Anchor::BOTTOM_CENTER,
                Anchor::BOTTOM_RIGHT,
            ]
            .iter()
            .enumerate()
            {
                let i = (anchor_index % 3) as f32;
                let j = (anchor_index / 3) as f32;

                // Spawn black square behind sprite to show anchor point
                commands
                    .spawn((
                        Sprite::from_color(Color::BLACK, sprite_size),
                        Transform::from_xyz(i * len - len, j * len - len, -1.0),
                        Pickable::default(),
                    ))
                    .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
                    .observe(recolor_on::<Pointer<Out>>(Color::BLACK))
                    .observe(recolor_on::<Pointer<Press>>(Color::srgb(1.0, 1.0, 0.0)))
                    .observe(recolor_on::<Pointer<Release>>(Color::srgb(0.0, 1.0, 1.0)));

                commands
                    .spawn((
                        Sprite {
                            image: asset_server.load("branding/bevy_bird_dark.png"),
                            custom_size: Some(sprite_size),
                            color: Color::srgb(1.0, 0.0, 0.0),
                            ..default()
                        },
                        anchor.to_owned(),
                        // 3x3 grid of anchor examples by changing transform
                        Transform::from_xyz(i * len - len, j * len - len, 0.0)
                            .with_scale(Vec3::splat(1.0 + (i - 1.0) * 0.2))
                            .with_rotation(Quat::from_rotation_z((j - 1.0) * 0.2)),
                        Pickable::default(),
                    ))
                    .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 0.0)))
                    .observe(recolor_on::<Pointer<Out>>(Color::srgb(1.0, 0.0, 0.0)))
                    .observe(recolor_on::<Pointer<Press>>(Color::srgb(0.0, 0.0, 1.0)))
                    .observe(recolor_on::<Pointer<Release>>(Color::srgb(0.0, 1.0, 0.0)));
            }
        });
}

#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut Sprite)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        let Some(texture_atlas) = &mut sprite.texture_atlas else {
            continue;
        };

        timer.tick(time.delta());

        if timer.just_finished() {
            texture_atlas.index = if texture_atlas.index == indices.last {
                indices.first
            } else {
                texture_atlas.index + 1
            };
        }
    }
}

fn setup_atlas(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::new(24, 24), 7, 1, None, None);
    let texture_atlas_layout_handle = texture_atlas_layouts.add(layout);
    // Use only the subset of sprites in the sheet that make up the run animation
    let animation_indices = AnimationIndices { first: 1, last: 6 };
    commands
        .spawn((
            Sprite::from_atlas_image(
                texture_handle,
                TextureAtlas {
                    layout: texture_atlas_layout_handle,
                    index: animation_indices.first,
                },
            ),
            Transform::from_xyz(300.0, 0.0, 0.0).with_scale(Vec3::splat(6.0)),
            animation_indices,
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
            Pickable::default(),
        ))
        .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
        .observe(recolor_on::<Pointer<Out>>(Color::srgb(1.0, 1.0, 1.0)))
        .observe(recolor_on::<Pointer<Press>>(Color::srgb(1.0, 1.0, 0.0)))
        .observe(recolor_on::<Pointer<Release>>(Color::srgb(0.0, 1.0, 1.0)));
}

// An observer that changes the target entity's color.
fn recolor_on<E: EntityEvent + Debug + Clone + Reflect>(
    color: Color,
) -> impl Fn(On<E>, Query<&mut Sprite>) {
    move |ev, mut sprites| {
        let Ok(mut sprite) = sprites.get_mut(ev.target()) else {
            return;
        };
        sprite.color = color;
    }
}
