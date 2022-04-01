//! Animation for the game engine Bevy

#![warn(missing_docs)]

use std::ops::Deref;

use bevy_app::{App, Plugin};
use bevy_asset::{AddAsset, Assets, Handle};
use bevy_core::{Name, Time};
use bevy_ecs::{
    change_detection::DetectChanges,
    entity::Entity,
    prelude::{Bundle, Component},
    system::{Query, Res},
};
use bevy_hierarchy::Children;
use bevy_math::{Quat, Vec3};
use bevy_reflect::TypeUuid;
use bevy_transform::prelude::Transform;
use bevy_utils::{tracing::warn, HashMap};

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        AnimationBundle, AnimationClip, AnimationPlayer, AnimationPlugin, EntityPath, Keyframes,
        VariableCurve,
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
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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

    /// Add a [`VariableCurve`] to an [`EntityPath`].
    pub fn add_curve_to_path(&mut self, path: EntityPath, curve: VariableCurve) {
        let curve_duration = curve.keyframe_timestamps.last().unwrap_or(&0.0);
        self.duration = self.duration.max(*curve_duration);
        self.curves.entry(path).or_default().push(curve);
    }
}

/// Animation controls
#[derive(Component)]
pub struct AnimationPlayer {
    /// Pause the animation
    pub paused: bool,
    /// Enable looping for the animation
    pub looping: bool,
    /// Speed of the animation
    pub speed: f32,
    /// Elapsed time in the animation
    pub elapsed: f32,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            looping: false,
            speed: 1.0,
            elapsed: 0.0,
        }
    }
}

/// Bundle to add an [`AnimationClip`] to an entity, and start playing it
#[derive(Bundle, Default)]
pub struct AnimationBundle {
    /// Controls for the animation
    pub player: AnimationPlayer,
    /// Handle to the [`AnimationClip`]
    pub handle: Handle<AnimationClip>,
}

/// System that will play all animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    mut animated: Query<(Entity, &Handle<AnimationClip>, &mut AnimationPlayer)>,
    named: Query<&Name>,
    mut transformed: Query<&mut Transform>,
    children: Query<&Children>,
) {
    for (entity, animation_handle, mut player) in animated.iter_mut() {
        if let Some(animation_clip) = animations.get(animation_handle) {
            // Continue if paused unless the `AnimationPlayer` was changed
            // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
            if player.paused && !player.is_changed() {
                continue;
            }
            if !player.paused {
                player.elapsed += time.delta_seconds() * player.speed;
            }
            let mut elapsed = player.elapsed;
            if player.looping {
                elapsed %= animation_clip.duration;
            }
            'entity: for (path, curves) in &animation_clip.curves {
                // PERF: finding the target entity can be optimised
                let mut current_entity = entity;
                // Ignore the first name, it is the root node which we already have
                for part in path.parts.iter().skip(1) {
                    let mut found = false;
                    if let Ok(children) = children.get(current_entity) {
                        for child in children.deref() {
                            if let Ok(name) = named.get(*child) {
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
                if let Ok(mut transform) = transformed.get_mut(current_entity) {
                    for curve in curves {
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
                                transform.rotation = Quat::from_array(rot_start.normalize().into())
                                    .slerp(Quat::from_array(rot_end.normalize().into()), lerp);
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
            .add_system(animation_player);
    }
}
