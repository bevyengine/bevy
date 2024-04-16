//! Animates a sprite in response to a keyboard event.
//!
//! See sprite_sheet.rs for an example where the sprite animation loops indefinitely.

use std::time::Duration;

use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest())) // prevents blurry sprites
        .add_systems(Startup, setup)
        .add_systems(Update, execute_animations)
        // press the left arrow key to animate the left sprite
        .add_systems(
            Update,
            trigger_animation::<LeftSprite>.run_if(input_just_pressed(KeyCode::ArrowLeft)),
        )
        // press the right arrow key to animate the right sprite
        .add_systems(
            Update,
            trigger_animation::<RightSprite>.run_if(input_just_pressed(KeyCode::ArrowRight)),
        )
        .run();
}

fn trigger_animation<S: Component>(mut query: Query<&mut AnimationConfig, With<S>>) {
    // we expect the Component of type S to be used as a marker Component by only a single entity
    let mut animation = query.single_mut();
    animation.frame_timer = AnimationConfig::timer_from_fps(animation.fps);
}

#[derive(Component)]
struct AnimationConfig {
    first_sprite_index: usize,
    last_sprite_index: usize,
    fps: u8,
    frame_timer: Timer,
}

impl AnimationConfig {
    fn new(first: usize, last: usize, fps: u8) -> Self {
        Self {
            first_sprite_index: first,
            last_sprite_index: last,
            fps,
            frame_timer: Self::timer_from_fps(fps),
        }
    }

    fn timer_from_fps(fps: u8) -> Timer {
        Timer::new(Duration::from_secs_f32(1.0 / (fps as f32)), TimerMode::Once)
    }
}

fn execute_animations(
    time: Res<Time>,
    mut query: Query<(&mut AnimationConfig, &mut TextureAtlas)>,
) {
    for (mut config, mut atlas) in &mut query {
        config.frame_timer.tick(time.delta());
        if config.frame_timer.just_finished() {
            if atlas.index == config.last_sprite_index {
                atlas.index = config.first_sprite_index;
            } else {
                atlas.index += 1;
                config.frame_timer = AnimationConfig::timer_from_fps(config.fps);
            }
        }
    }
}

#[derive(Component)]
struct LeftSprite;

#[derive(Component)]
struct RightSprite;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let texture = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let animation_config_1 = AnimationConfig::new(1, 6, 10);

    commands.spawn(Camera2dBundle::default());

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(-50.0, 0.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: animation_config_1.first_sprite_index,
        },
        LeftSprite,
        animation_config_1,
    ));

    let animation_config_2 = AnimationConfig::new(1, 6, 20);

    commands.spawn((
        SpriteBundle {
            transform: Transform::from_scale(Vec3::splat(6.0))
                .with_translation(Vec3::new(50.0, 0.0, 0.0)),
            texture: texture.clone(),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout.clone(),
            index: animation_config_2.first_sprite_index,
        },
        RightSprite,
        animation_config_2,
    ));
}
