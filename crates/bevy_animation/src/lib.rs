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
use bevy_utils::{tracing::warn, HashMap};

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

/// A representation of the data needed transition to a new animation
#[derive(Clone, Debug, Reflect)]
pub struct AnimationTransition {
    next_animation_clip: Handle<AnimationClip>,
    repeat: bool,
    speed: f32,
    /// The elapsed time of the target clip (is scaled by speed and reset on clip end)
    clip_elapsed: f32,
    /// The actual time that the transition is running in seconds
    transition_elapsed: f32,
    /// The desired duration of the transition
    transition_time: f32,
}

impl Default for AnimationTransition {
    fn default() -> Self {
        Self {
            next_animation_clip: Default::default(),
            repeat: false,
            speed: 1.0,
            clip_elapsed: 0.0,
            transition_time: 0.0,
            transition_elapsed: 0.0,
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
    pub fn set_elapsed(&mut self, elapsed: f32) -> &mut Self {
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
    elapsed: f32,
    animation_clip: Handle<AnimationClip>,
    transition: AnimationTransition,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: 0.0,
            animation_clip: Default::default(),
            transition: AnimationTransition::default(),
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
        transition_time: f32,
    ) -> &mut AnimationTransition {
        self.resume();
        self.transition = AnimationTransition {
            next_animation_clip: handle,
            transition_time,
            ..Default::default()
        };
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
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Seek to a specific time in the animation
    pub fn set_elapsed(&mut self, elapsed: f32) -> &mut Self {
        self.elapsed = elapsed;
        self
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
            // If next_clip is a valid animation the player is in transition
            let next_clip = animations.get(&player.transition.next_animation_clip);
            let in_transition = next_clip.is_some();

            // Continue if paused unless the `AnimationPlayer` was changed
            // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
            if player.paused && !player.is_changed() {
                continue;
            }
            if !player.paused {
                if in_transition {
                    player.transition.transition_elapsed += time.delta_seconds();
                    player.transition.clip_elapsed +=
                        time.delta_seconds() * player.transition.speed;
                    player.elapsed += time.delta_seconds() * player.speed;
                } else {
                    player.elapsed += time.delta_seconds() * player.speed;
                }
            }

            let mut transition_lerp = 0.0;
            if in_transition {
                transition_lerp =
                    player.transition.transition_elapsed / player.transition.transition_time;
            }

            if transition_lerp >= 1.0 {
                // set to exactly one so the last step of the interpolation is exact
                transition_lerp = 1.0;
            }

            let mut current_elapsed = player.elapsed;
            if player.repeat {
                current_elapsed %= animation_clip.duration;
            }
            if current_elapsed < 0.0 {
                current_elapsed += animation_clip.duration;
            }

            let mut next_elapsed = player.transition.clip_elapsed;
            if let Some(next_clip) = next_clip {
                if player.transition.repeat {
                    next_elapsed %= next_clip.duration;
                }
                if next_elapsed < 0.0 {
                    next_elapsed += next_clip.duration;
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
                    let mut clip_curves = vec![curves];
                    let mut updated_transforms = vec![*transform];

                    // If in transition also push the target clip to the array
                    if let Some(next_clip) = next_clip {
                        if let Some(next_curves) = next_clip.curves.get(path) {
                            clip_curves.push(next_curves);
                            updated_transforms.push(*transform);
                        }
                    }

                    for (index, curves) in clip_curves.iter().enumerate() {
                        let local_elapsed = if index == 0 {
                            current_elapsed
                        } else {
                            next_elapsed
                        };
                        for curve in *curves {
                            // Some curves have only one keyframe used to set a transform
                            if curve.keyframe_timestamps.len() == 1 {
                                match &curve.keyframes {
                                    Keyframes::Rotation(keyframes) => {
                                        updated_transforms[index].rotation = keyframes[0];
                                    }
                                    Keyframes::Translation(keyframes) => {
                                        updated_transforms[index].translation = keyframes[0];
                                    }
                                    Keyframes::Scale(keyframes) => {
                                        updated_transforms[index].scale = keyframes[0];
                                    }
                                }
                                continue;
                            }

                            // Find the current keyframe
                            // PERF: finding the current keyframe can be optimised
                            let step_start =
                                match curve.keyframe_timestamps.binary_search_by(|probe| {
                                    probe.partial_cmp(&local_elapsed).unwrap()
                                }) {
                                    Ok(i) => i,
                                    Err(0) => continue, // this curve isn't started yet
                                    Err(n) if n > curve.keyframe_timestamps.len() - 1 => continue, // this curve is finished
                                    Err(i) => i - 1,
                                };
                            let ts_start = curve.keyframe_timestamps[step_start];
                            let ts_end = curve.keyframe_timestamps[step_start + 1];
                            let lerp = (local_elapsed - ts_start) / (ts_end - ts_start);

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
                                    let result =
                                        rot_start.normalize().slerp(rot_end.normalize(), lerp);
                                    updated_transforms[index].rotation = result;
                                }
                                Keyframes::Translation(keyframes) => {
                                    let translation_start = keyframes[step_start];
                                    let translation_end = keyframes[step_start + 1];
                                    let result = translation_start.lerp(translation_end, lerp);
                                    updated_transforms[index].translation = result;
                                }
                                Keyframes::Scale(keyframes) => {
                                    let scale_start = keyframes[step_start];
                                    let scale_end = keyframes[step_start + 1];
                                    let result = scale_start.lerp(scale_end, lerp);
                                    updated_transforms[index].scale = result;
                                }
                            }
                        }
                    }

                    // if updated_transforms has length 2 the animation is in transition and we use the computed transforms
                    // from both the current and the target curve and interpolate between them using the transition_lerp factor
                    if updated_transforms.len() == 1 {
                        *transform = updated_transforms[0];
                    } else if updated_transforms.len() == 2 {
                        let from = updated_transforms[0];
                        let to = updated_transforms[1];
                        transform.rotation = from.rotation.slerp(to.rotation, transition_lerp);
                        transform.translation =
                            from.translation.lerp(to.translation, transition_lerp);
                        transform.scale = from.scale.lerp(to.scale, transition_lerp);
                    }
                }
            }

            // Transition to next clip has finished
            if transition_lerp == 1.0 {
                let next_clip = player.transition.next_animation_clip.clone_weak();
                let next_speed = player.transition.speed;
                let repeat = player.transition.repeat;
                player
                    .play(next_clip)
                    .set_elapsed(next_elapsed)
                    .set_speed(next_speed);
                if repeat {
                    player.repeat();
                }
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
