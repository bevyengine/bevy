//! This example demonstrates how to apply a grayscale effect to an animation in a sprite sheet using 
//! a custom shader. It renders an animated sprite by loading all animation frames from a single image 
//! (a sprite sheet) into a texture atlas, and changing the displayed image periodically.

use bevy::prelude::*;
use bevy_internal::{
    render::render_resource::{AsBindGroup, ShaderRef},
    sprite::{SpriteMaterial, SpriteMaterialPlugin, SpriteSheetWithMaterialBundle},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_plugins(SpriteMaterialPlugin::<GrayScale>::default()) // Add the grayscale material plugin to the app
        .add_systems(Startup, setup)
        .add_systems(Update, animate_sprite)
        .run();
}

// Component to store the indices of the first and last frames of an animation in the sprite sheet
#[derive(Component)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

// Component to store a timer for animating the sprite
#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(
        &AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
    )>,
) {
    for (indices, mut timer, mut sprite) in &mut query {
        timer.tick(time.delta());

        // If the timer has just finished, advance to the next frame of the animation
        if timer.just_finished() {
            sprite.index = if sprite.index == indices.last {
                indices.first
            } else {
                sprite.index + 1
            };
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
    mut sprite_materials: ResMut<Assets<GrayScale>>,
) {
    // Load the sprite sheet texture
    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");

    // Create a texture atlas from the sprite sheet
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Define the indices of the frames in the sprite sheet that make up the run animation
    let animation_indices = AnimationIndices { first: 1, last: 6 };

    // Spawn a camera and a sprite with the grayscale material and the run animation
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        SpriteSheetWithMaterialBundle {
            texture_atlas: texture_atlas_handle,
            sprite: TextureAtlasSprite::new(animation_indices.first),
            transform: Transform::from_scale(Vec3::splat(6.0)),
            material: sprite_materials.add(GrayScale {}),
            ..default()
        },
        animation_indices,
        AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating)),
    ));
}

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone)]
struct GrayScale {}

impl SpriteMaterial for GrayScale {
    fn fragment_shader() -> ShaderRef {
        "shaders/grayscale.wgsl".into()
    }
}