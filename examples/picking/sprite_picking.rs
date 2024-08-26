//! Demonstrates picking for sprites and sprite atlases. The picking backend only tests against the
//! sprite bounds, so the sprite atlas can be picked by clicking on its trnasparent areas.

use bevy::{prelude::*, sprite::Anchor};
use std::fmt::Debug;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, setup_atlas))
        .add_systems(Update, (move_sprite, animate_sprite))
        .run();
}

fn move_sprite(
    time: Res<Time>,
    mut sprite: Query<&mut Transform, (Without<Sprite>, With<Children>)>,
) {
    let t = time.elapsed_seconds() * 0.1;
    for mut transform in &mut sprite {
        let new = Vec2 {
            x: 50.0 * t.sin(),
            y: 50.0 * (t * 2.0).sin(),
        };
        transform.translation.x = new.x;
        transform.translation.y = new.y;
    }
}

/// Set up a scene that tests all sprite anchor types.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let len = 128.0;
    let sprite_size = Some(Vec2::splat(len / 2.0));

    commands
        .spawn(SpatialBundle::default())
        .with_children(|commands| {
            for (anchor_index, anchor) in [
                Anchor::TopLeft,
                Anchor::TopCenter,
                Anchor::TopRight,
                Anchor::CenterLeft,
                Anchor::Center,
                Anchor::CenterRight,
                Anchor::BottomLeft,
                Anchor::BottomCenter,
                Anchor::BottomRight,
                Anchor::Custom(Vec2::new(0.5, 0.5)),
            ]
            .iter()
            .enumerate()
            {
                let i = (anchor_index % 3) as f32;
                let j = (anchor_index / 3) as f32;

                // spawn black square behind sprite to show anchor point
                commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: sprite_size,
                            color: Color::BLACK,
                            ..default()
                        },
                        transform: Transform::from_xyz(i * len - len, j * len - len, -1.0),
                        ..default()
                    })
                    .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
                    .observe(recolor_on::<Pointer<Out>>(Color::BLACK))
                    .observe(recolor_on::<Pointer<Down>>(Color::srgb(1.0, 1.0, 0.0)))
                    .observe(recolor_on::<Pointer<Up>>(Color::srgb(0.0, 1.0, 1.0)));

                commands
                    .spawn(SpriteBundle {
                        sprite: Sprite {
                            custom_size: sprite_size,
                            color: Color::srgb(1.0, 0.0, 0.0),
                            anchor: anchor.to_owned(),
                            ..default()
                        },
                        texture: asset_server.load("branding/bevy_bird_dark.png"),
                        // 3x3 grid of anchor examples by changing transform
                        transform: Transform::from_xyz(i * len - len, j * len - len, 0.0)
                            .with_scale(Vec3::splat(1.0 + (i - 1.0) * 0.2))
                            .with_rotation(Quat::from_rotation_z((j - 1.0) * 0.2)),
                        ..default()
                    })
                    .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 0.0)))
                    .observe(recolor_on::<Pointer<Out>>(Color::srgb(1.0, 0.0, 0.0)))
                    .observe(recolor_on::<Pointer<Down>>(Color::srgb(0.0, 0.0, 1.0)))
                    .observe(recolor_on::<Pointer<Up>>(Color::srgb(0.0, 1.0, 0.0)));
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
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            sprite.index = if sprite.index == indices.last {
                indices.first
            } else {
                sprite.index + 1
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
            TextureAtlas {
                layout: texture_atlas_layout_handle,
                index: animation_indices.first,
            },
            SpriteBundle {
                texture: texture_handle,
                transform: Transform::from_xyz(300.0, 0.0, 0.0).with_scale(Vec3::splat(6.0)),
                ..default()
            },
            animation_indices,
            AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
        ))
        .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 1.0)))
        .observe(recolor_on::<Pointer<Out>>(Color::srgb(1.0, 1.0, 1.0)))
        .observe(recolor_on::<Pointer<Down>>(Color::srgb(1.0, 1.0, 0.0)))
        .observe(recolor_on::<Pointer<Up>>(Color::srgb(0.0, 1.0, 1.0)));
}

// An observer listener that changes the target entity's color.
fn recolor_on<E: Debug + Clone + Reflect>(color: Color) -> impl Fn(Trigger<E>, Query<&mut Sprite>) {
    move |ev, mut sprites| {
        let Ok(mut sprite) = sprites.get_mut(ev.entity()) else {
            return;
        };
        sprite.color = color;
    }
}
