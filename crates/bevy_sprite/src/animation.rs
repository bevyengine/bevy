use crate::{TextureAtlas, TextureAtlasSprite};
use bevy_asset::Handle;
use bevy_core::{Time, Timer};
use bevy_ecs::system::{Query, Res};
use bevy_utils::Duration;

/// A Collection of Sprite Animations and state
pub struct SpriteAnimator {
    /// Current Animation Sequence
    pub current_animation: usize,
    /// Current Animation Frame
    pub current_frame: usize,
    /// Current Animation Timer
    pub sprite_timer: Timer,
}

/// A Collection of Sprite Animations
pub struct SpriteAnimations(Vec<SpriteAnimation>);

/// A Sequence of Sprite Frames
pub struct SpriteAnimation(Vec<SpriteFrame>);

/// An Individual Sprite Frame
pub struct SpriteFrame {
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
            if let Some(animation) = animations.0.get(animator.current_animation) {
                animator.current_frame = (animator.current_frame + 1) % animation.0.len();
                if let Some(frame) = animation.0.get(animator.current_frame) {
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
