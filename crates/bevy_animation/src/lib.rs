//! Animation for the game engine Bevy

#![warn(missing_docs)]

use std::ops::Deref;

use bevy_app::{App, CoreStage, Plugin};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::Component,
    reflect::ReflectComponent,
    schedule::ParallelSystemDescriptorCoercion,
    system::{Query, Res},
};
use bevy_hierarchy::Children;
use bevy_math::{Quat, Vec3};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::{tracing::warn, Duration, HashMap};

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AnimationClip, AnimationPlayer, AnimationPlugin, EntityPath, Keyframes, VariableCurve,
    };
}

/// List of keyframes for one of the attribute of a [`Transform`].
#[derive(Clone, Debug)]
pub enum Keyframes {
    /// Keyframes for rotation.
    Rotation(Vec<Quat>),
    /// Keyframes for translation.
    Translation(Vec<Vec3>),
    /// Keyframes for scale.
    Scale(Vec<Vec3>),
}

/// Describes how an attribute of a [`Transform`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes,
}

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Clone, TypeUuid, Debug, Default)]
#[uuid = "d81b7179-0448-4eb0-89fe-c067222725bf"]
pub struct AnimationClip {
    curves: HashMap<EntityPath, Vec<VariableCurve>>,
    duration: f32,
}

impl AnimationClip {
    #[inline]
    /// Hashmap of the [`VariableCurve`]s per [`EntityPath`].
    pub fn curves(&self) -> &HashMap<EntityPath, Vec<VariableCurve>> {
        &self.curves
    }

    /// Duration of the clip, represented in seconds
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Add a [`VariableCurve`] to an [`EntityPath`].
    pub fn add_curve_to_path(&mut self, path: EntityPath, curve: VariableCurve) {
        // Update the duration of the animation by this curve duration if it's longer
        self.duration = self
            .duration
            .max(*curve.keyframe_timestamps.last().unwrap_or(&0.0));
        self.curves.entry(path).or_default().push(curve);
    }
}

/// The data needed to transition to a new animation
///
/// Stored in an [`AnimationPlayer`]
#[derive(Clone, Debug, Reflect)]
pub struct AnimationTransition {
    next_animation_clip: Handle<AnimationClip>,
    repeat: bool,
    speed: f32,
    /// The elapsed time of the target clip (is scaled by speed and reset on clip end)
    clip_elapsed: Duration,
    /// The actual time that the transition is running in seconds
    transition_elapsed: Duration,
    /// The desired duration of the transition
    transition_time: Duration,
}

impl Default for AnimationTransition {
    fn default() -> Self {
        Self {
            next_animation_clip: Default::default(),
            repeat: false,
            speed: 1.0,
            clip_elapsed: Duration::ZERO,
            transition_time: Duration::ZERO,
            transition_elapsed: Duration::ZERO,
        }
    }
}

impl AnimationTransition {
    /// Set the next animation to repeat
    pub fn repeat(&mut self) -> &mut Self {
        self.repeat = true;
        self
    }

    /// Speed of the animation playback for the next animation
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Set the speed of the next animation playback
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Seek to a specific time in the next animation
    pub fn set_elapsed(&mut self, elapsed: Duration) -> &mut Self {
        self.clip_elapsed = elapsed;
        self
    }
}

/// Animation controls
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayer {
    paused: bool,
    repeat: bool,
    speed: f32,
    elapsed: Duration,
    animation_clip: Handle<AnimationClip>,
    transition: AnimationTransition,
    in_transition: bool,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: Duration::ZERO,
            animation_clip: Default::default(),
            transition: AnimationTransition::default(),
            in_transition: false,
        }
    }
}

impl AnimationPlayer {
    /// Start playing an animation, resetting state of the player
    pub fn play(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        *self = Self {
            animation_clip: handle,
            ..Default::default()
        };
        self
    }

    /// Crosfade from the current to the next animation
    ///
    /// - `handle` a handle to the target animation clip
    /// - `transition_time` determines the duration of the transition in seconds
    pub fn cross_fade(
        &mut self,
        handle: Handle<AnimationClip>,
        transition_time: Duration,
    ) -> &mut AnimationTransition {
        self.resume();
        self.transition = AnimationTransition {
            next_animation_clip: handle,
            transition_time,
            ..Default::default()
        };
        self.in_transition = true;
        &mut self.transition
    }

    /// Set the animation to repeat
    pub fn repeat(&mut self) -> &mut Self {
        self.repeat = true;
        self
    }

    /// Stop the animation from repeating
    pub fn stop_repeating(&mut self) -> &mut Self {
        self.repeat = false;
        self
    }

    /// Pause the animation
    pub fn pause(&mut self) {
        self.paused = true;
    }

    /// Unpause the animation
    pub fn resume(&mut self) {
        self.paused = false;
    }

    /// Is the animation paused
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Speed of the animation playback
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Set the speed of the animation playback
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Time elapsed playing the animation
    pub fn elapsed(&self) -> Duration {
        self.elapsed
    }

    /// Seek to a specific time in the animation
    pub fn set_elapsed(&mut self, elapsed: Duration) -> &mut Self {
        self.elapsed = elapsed;
        self
    }

    /// Is the animation in transition
    pub fn is_in_transition(&self) -> bool {
        self.in_transition
    }
}

/// System that will play all animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    mut animation_players: Query<(Entity, &mut AnimationPlayer)>,
    names: Query<&Name>,
    mut transforms: Query<&mut Transform>,
    children: Query<&Children>,
) {
    for (entity, mut player) in &mut animation_players {
        if let Some(animation_clip) = animations.get(&player.animation_clip) {
            let next_clip = if player.in_transition {
                animations.get(&player.transition.next_animation_clip)
            } else {
                None
            };

            let mut transition_just_finished = false;

            // Continue if paused unless the `AnimationPlayer` was changed
            // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
            if player.paused && !player.is_changed() {
                continue;
            }
            if !player.paused {
                let delta_seconds = time.delta_seconds();
                if player.in_transition {
                    player.transition.transition_elapsed += Duration::from_secs_f32(delta_seconds);
                    let t_speed = player.transition.speed;
                    player.transition.clip_elapsed +=
                        Duration::from_secs_f32(delta_seconds * t_speed);
                }
                let p_speed = player.speed;
                player.elapsed += Duration::from_secs_f32(delta_seconds * p_speed);
            }

            let mut transition_lerp = 0.0;
            if player.in_transition {
                transition_lerp = player.transition.transition_elapsed.as_secs_f32()
                    / player.transition.transition_time.as_secs_f32();
            }

            if transition_lerp >= 1.0 {
                // set to exactly one so the last step of the interpolation is exact
                transition_lerp = 1.0;
                transition_just_finished = true;
            }

            let mut current_elapsed = player.elapsed.as_secs_f32();
            if player.repeat {
                current_elapsed %= animation_clip.duration;
            }
            if current_elapsed < 0.0 {
                current_elapsed += animation_clip.duration;
            }

            let mut next_elapsed = 0.0;
            if player.in_transition {
                if let Some(next_clip) = next_clip {
                    next_elapsed = player.transition.clip_elapsed.as_secs_f32();
                    if player.transition.repeat {
                        next_elapsed %= next_clip.duration;
                    }
                    if next_elapsed < 0.0 {
                        next_elapsed += next_clip.duration;
                    }
                }
            }

            'entity: for (path, curves) in &animation_clip.curves {
                // PERF: finding the target entity can be optimised
                let mut current_entity = entity;
                // Ignore the first name, it is the root node which we already have
                for part in path.parts.iter().skip(1) {
                    let mut found = false;
                    if let Ok(children) = children.get(current_entity) {
                        for child in children.deref() {
                            if let Ok(name) = names.get(*child) {
                                if name == part {
                                    // Found a children with the right name, continue to the next part
                                    current_entity = *child;
                                    found = true;
                                    break;
                                }
                            }
                        }
                    }
                    if !found {
                        warn!("Entity not found for path {:?} on part {:?}", path, part);
                        continue 'entity;
                    }
                }
                if let Ok(mut transform) = transforms.get_mut(current_entity) {
                    if !player.in_transition {
                        update_transform(curves, current_elapsed, &mut transform);
                    } else if let Some(next_clip) = next_clip {
                        if let Some(next_curves) = next_clip.curves.get(path) {
                            let mut from = *transform;
                            let mut to = *transform;

                            update_transform(curves, current_elapsed, &mut from);
                            update_transform(next_curves, next_elapsed, &mut to);

                            transform.rotation = from.rotation.slerp(to.rotation, transition_lerp);
                            transform.translation =
                                from.translation.lerp(to.translation, transition_lerp);
                            transform.scale = from.scale.lerp(to.scale, transition_lerp);
                        }
                    }
                }
            }

            // Execute if transition to next clip has finished
            if transition_just_finished {
                let next_clip = player.transition.next_animation_clip.clone_weak();
                let next_speed = player.transition.speed;
                let repeat = player.transition.repeat;
                player
                    .play(next_clip)
                    .set_elapsed(Duration::from_secs_f32(next_elapsed))
                    .set_speed(next_speed);
                if repeat {
                    player.repeat();
                }
            }
        }
    }
}

#[inline(always)]
fn update_transform(curves: &Vec<VariableCurve>, elapsed: f32, mut transform: &mut Transform) {
    for curve in curves {
        // Some curves have only one keyframe used to set a transform
        if curve.keyframe_timestamps.len() == 1 {
            match &curve.keyframes {
                Keyframes::Rotation(keyframes) => {
                    transform.rotation = keyframes[0];
                }
                Keyframes::Translation(keyframes) => {
                    transform.translation = keyframes[0];
                }
                Keyframes::Scale(keyframes) => {
                    transform.scale = keyframes[0];
                }
            }
            continue;
        }

        // Find the current keyframe
        // PERF: finding the current keyframe can be optimised
        let step_start = match curve
            .keyframe_timestamps
            .binary_search_by(|probe| probe.partial_cmp(&elapsed).unwrap())
        {
            Ok(i) => i,
            Err(0) => continue, // this curve isn't started yet
            Err(n) if n > curve.keyframe_timestamps.len() - 1 => continue, // this curve is finished
            Err(i) => i - 1,
        };
        let ts_start = curve.keyframe_timestamps[step_start];
        let ts_end = curve.keyframe_timestamps[step_start + 1];
        let lerp = (elapsed - ts_start) / (ts_end - ts_start);

        // Apply the keyframe
        match &curve.keyframes {
            Keyframes::Rotation(keyframes) => {
                let rot_start = keyframes[step_start];
                let mut rot_end = keyframes[step_start + 1];
                // Choose the smallest angle for the rotation
                if rot_end.dot(rot_start) < 0.0 {
                    rot_end = -rot_end;
                }

                // Rotations are using a spherical linear interpolation
                let result = rot_start.normalize().slerp(rot_end.normalize(), lerp);
                transform.rotation = result;
            }
            Keyframes::Translation(keyframes) => {
                let translation_start = keyframes[step_start];
                let translation_end = keyframes[step_start + 1];
                let result = translation_start.lerp(translation_end, lerp);
                transform.translation = result;
            }
            Keyframes::Scale(keyframes) => {
                let scale_start = keyframes[step_start];
                let scale_end = keyframes[step_start + 1];
                let result = scale_start.lerp(scale_end, lerp);
                transform.scale = result;
            }
        }
    }
}

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin {}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                animation_player.before(TransformSystem::TransformPropagate),
            );
    }
}
