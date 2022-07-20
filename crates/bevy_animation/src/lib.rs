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
use bevy_hierarchy::{Children, HierarchySystem};
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

/// Animation controls
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayer {
    paused: bool,
    repeat: bool,
    speed: f32,
    elapsed: f32,
    animation_clip: Handle<AnimationClip>,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: 0.0,
            animation_clip: Default::default(),
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
    for (entity, mut player) in animation_players.iter_mut() {
        if let Some(animation_clip) = animations.get(&player.animation_clip) {
            // Continue if paused unless the `AnimationPlayer` was changed
            // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
            if player.paused && !player.is_changed() {
                continue;
            }
            if !player.paused {
                player.elapsed += time.delta_seconds() * player.speed;
            }
            let mut elapsed = player.elapsed;
            if player.repeat {
                elapsed %= animation_clip.duration;
            }
            if elapsed < 0.0 {
                elapsed += animation_clip.duration;
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
                    for curve in curves {
                        // Some curves have only one keyframe used to set a transform
                        if curve.keyframe_timestamps.len() == 1 {
                            match &curve.keyframes {
                                Keyframes::Rotation(keyframes) => transform.rotation = keyframes[0],
                                Keyframes::Translation(keyframes) => {
                                    transform.translation = keyframes[0];
                                }
                                Keyframes::Scale(keyframes) => transform.scale = keyframes[0],
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
                                transform.rotation =
                                    rot_start.normalize().slerp(rot_end.normalize(), lerp);
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
                animation_player
                    .before(TransformSystem::TransformPropagate)
                    .after(HierarchySystem::ParentUpdate),
            );
    }
}
