//! Animation Example
//! ```rust
//! use bevy::prelude::*;
//!
//! fn main() {
//!     App::build()
//!         .add_plugins(DefaultPlugins)
//!         .add_startup_system(setup.system())
//!         .add_system(bevy::sprite::sprite_animation_system.system())
//!         .run();
//! }
//!
//! fn setup(
//!     mut commands: Commands,
//!     asset_server: Res<AssetServer>,
//!     mut texture_atlases: ResMut<Assets<TextureAtlas>>,
//! ) {
//!     use bevy_sprite::SpriteAnimationBundle;
//! let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
//!     let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1);
//!     let texture_atlas_handle = texture_atlases.add(texture_atlas);
//!
//!     commands.spawn_bundle(OrthographicCameraBundle::new_2d());
//!     commands
//!         .spawn_bundle(SpriteSheetBundle {
//!             texture_atlas: texture_atlas_handle,
//!             transform: Transform::from_scale(Vec3::splat(6.0)),
//!             ..Default::default()
//!         })
//!         .with_bundle(SpriteAnimationBundle{
//!             animations: SpriteAnimations{
//!                 animations: vec![SpriteAnimation {
//!                     frames: vec![
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 0,
//!                             duration: 0.2,
//!                         },
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 1,
//!                             duration: 0.2,
//!                         }
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 2,
//!                             duration: 0.2,
//!                         },
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 3,
//!                             duration: 0.2,
//!                         },
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 4,
//!                             duration: 0.2,
//!                         },
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 5,
//!                             duration: 0.2,
//!                         },
//!                         SpriteFrame {
//!                             atlas_handle: texture_atlas_handle.clone(),
//!                             atlas_index: 6,
//!                             duration: 0.2,
//!                         },
//!                     ]
//!                 }]
//!             },
//!             ..Default::default()
//!         })
//!         .insert(Timer::from_seconds(0.1, true));
//! }
//! ```
//!

use crate::{TextureAtlas, TextureAtlasSprite};
use bevy_asset::Handle;
use bevy_core::{Time, Timer};
use bevy_ecs::bundle::Bundle;
use bevy_ecs::system::{Query, Res};
use bevy_utils::Duration;

/// A Bundle of components for drawing an animated sequence of sprites from one or more sprite sheets
/// (also referred to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct SpriteAnimationBundle {
    /// The list of animations for this Entity
    pub animations: SpriteAnimations,
    /// Settings and context for the animator
    pub animator: SpriteAnimator,
}

/// A Collection of Sprite Animations and state
#[derive(Clone)]
pub struct SpriteAnimator {
    /// Current Animation Sequence
    pub current_animation: usize,
    /// Current Animation Frame
    pub current_frame: usize,
    /// Control How Sprite Animation Plays
    /// TODO: Should this be moved to SpriteAnimation?
    pub animation_mode: SpriteAnimationMode,
    /// Should Sprite Animation Repeat
    /// TODO: Should this be moved to SpriteAnimation?
    pub animation_repeat: bool,
    /// Current Animation Timer
    pub sprite_timer: Timer,
}

impl Default for SpriteAnimator {
    fn default() -> SpriteAnimator {
        SpriteAnimator {
            current_animation: 0,
            current_frame: 0,
            animation_mode: SpriteAnimationMode::Disabled,
            animation_repeat: false,
            sprite_timer: Timer::new(Default::default(), false),
        }
    }
}

#[derive(Clone)]
pub enum SpriteAnimationMode {
    /// Remain on current frame
    Disabled,
    /// Play Animation from 0..len
    Forward,
    /// Play Animation from len..0
    Reverse,
    /// Play Animation from 0..len..0
    PingPong,
}

impl Default for SpriteAnimationMode {
    fn default() -> SpriteAnimationMode {
        SpriteAnimationMode::Forward
    }
}

/// A Collection of Sprite Animations
#[derive(Clone)]
pub struct SpriteAnimations {
    pub animations: Vec<SpriteAnimation>,
}

/// A Sequence of Sprite Frames
#[derive(Clone)]
pub struct SpriteAnimation {
    pub frames: Vec<SpriteAnimationFrame>,
}

/// An Individual Sprite Frame
#[derive(Clone)]
pub struct SpriteAnimationFrame {
    /// Handle for Texture Atlas
    pub atlas_handle: Handle<TextureAtlas>,
    /// Texture Atlas Sprite Index
    pub atlas_index: u32,
    /// Duration to set for frame timer
    pub duration: f32,
}

pub fn sprite_animation_system(
    time: Res<Time>,
    mut query: Query<(
        &SpriteAnimations,
        &mut SpriteAnimator,
        &mut Handle<TextureAtlas>,
        &mut TextureAtlasSprite,
    )>,
) {
    for (animations, mut animator, mut atlas, mut sprite) in &mut query.iter_mut() {
        animator.sprite_timer.tick(time.delta());
        if animator.sprite_timer.finished() {
            if let Some(animation) = animations.animations.get(animator.current_animation) {
                animator.current_frame = match animator.animation_mode {
                    SpriteAnimationMode::Disabled => animator.current_frame,
                    SpriteAnimationMode::Forward => {
                        if animator.current_frame + 1 < animation.frames.len() {
                            animator.current_frame + 1
                        } else if animator.animation_repeat {
                            0
                        } else {
                            animator.animation_mode = SpriteAnimationMode::Disabled;
                            animator.current_frame
                        }
                    }
                    SpriteAnimationMode::Reverse => {
                        if animator.current_frame > 0 {
                            animator.current_frame - 1
                        } else if animator.animation_repeat {
                            animation.frames.len() - 1
                        } else {
                            animator.current_frame
                        }
                    }
                    SpriteAnimationMode::PingPong => {
                        let next_frame = animator.current_frame + 1;
                        if next_frame < animation.frames.len() {
                            // Going up
                            next_frame
                        } else if next_frame < animation.frames.len() * 2 - 1 {
                            // Going Down
                            animation.frames.len() * 2 - next_frame - 2
                        } else {
                            // Back at 0
                            if animator.animation_repeat {
                                0
                            } else {
                                animator.animation_mode = SpriteAnimationMode::Disabled;
                                animator.current_frame
                            }
                        }
                    }
                };
                if let Some(frame) = animation.frames.get(animator.current_frame) {
                    animator
                        .sprite_timer
                        .set_duration(Duration::from_secs_f32(frame.duration));
                    *atlas = frame.atlas_handle.clone();
                    sprite.index = frame.atlas_index;
                } else {
                    bevy_utils::tracing::event!(
                        bevy_utils::tracing::Level::DEBUG,
                        "Frame {} not found for animation {}.",
                        animator.current_frame,
                        animator.current_animation
                    );
                }
            } else {
                bevy_utils::tracing::event!(
                    bevy_utils::tracing::Level::DEBUG,
                    "Animation {} not found.",
                    animator.current_animation
                );
            }
        }
    }
}
