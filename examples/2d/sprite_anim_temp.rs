//! Animates a sprite in response to a keyboard event.
//!
//! See `sprite_sheet.rs` for an example where the sprite animation loops indefinitely.

use bevy::prelude::*;
use bevy::sprite::clip::{ClipFrames, ClipLoopMode};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    commands.spawn(Camera2dBundle::default());
    let clip = SpriteClip {
        // Handle ima
        clip: ClipFrames::from([0, 1, 2, 3, 4, 5, 6]),
        speed: 1.0,
        fps: 1,
        clip_loop: ClipLoopMode::Infinite,
    };

    let texture = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");

    let layout = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    // Clip is an asset so you can change the behaviour of an animation for all entities
    let clip = asset_server.add(clip);

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(-50.0, 0.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 0,
        },
        SpriteAnimationPlayer::new(clip.clone()),
    ));

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(50.0, 0.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 0,
        },
        // Overwrite clip settings without changing it for everyone else
        SpriteAnimationPlayer::new(clip.clone().overwrite().backward().fps(2)),
    ));

    // We can also share animation players so multiple entities are synced using Handle<SpriteAnimationPlayer>
    // even if they are spawned at different times
    let player = asset_server.add(SpriteAnimationPlayer::new(clip.clone().overwrite().fps(4)));
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(-50.0, 150.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 0,
        },
        // Synced player
        player.clone(),
    ));
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(50.0, 150.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: 0,
        },
        // Synced player
        player
    ));

}
