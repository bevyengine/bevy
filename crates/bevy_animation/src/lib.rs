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
    query::With,
    reflect::ReflectComponent,
    schedule::IntoSystemDescriptor,
    system::{Query, Res},
};
use bevy_hierarchy::{Children, Parent};
use bevy_math::{Quat, Vec3};
use bevy_reflect::{FromReflect, Reflect, TypeUuid};
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
#[derive(Reflect, FromReflect, Clone, Debug)]
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
#[derive(Reflect, FromReflect, Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes,
}

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Reflect, FromReflect, Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Reflect, FromReflect, Clone, TypeUuid, Debug, Default)]
#[uuid = "d81b7179-0448-4eb0-89fe-c067222725bf"]
pub struct AnimationClip {
    curves: Vec<Vec<VariableCurve>>,
    paths: HashMap<EntityPath, usize>,
    duration: f32,
}

impl AnimationClip {
    #[inline]
    /// [`VariableCurve`]s for each bone. Indexed by the bone ID.
    pub fn curves(&self) -> &Vec<Vec<VariableCurve>> {
        &self.curves
    }

    /// Gets the curves for a bone.
    ///
    /// Returns `None` if the bone is invalid.
    #[inline]
    pub fn get_curves(&self, bone_id: usize) -> Option<&'_ Vec<VariableCurve>> {
        self.curves.get(bone_id)
    }

    /// Gets the curves by it's [`EntityPath`].
    ///
    /// Returns `None` if the bone is invalid.
    #[inline]
    pub fn get_curves_by_path(&self, path: &EntityPath) -> Option<&'_ Vec<VariableCurve>> {
        self.paths.get(path).and_then(|id| self.curves.get(*id))
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
        if let Some(bone_id) = self.paths.get(&path) {
            self.curves[*bone_id].push(curve);
        } else {
            let idx = self.curves.len();
            self.curves.push(vec![curve]);
            self.paths.insert(path, idx);
        }
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
    path_cache: Vec<Vec<Option<Entity>>>,
}

impl Default for AnimationPlayer {
    fn default() -> Self {
        Self {
            paused: false,
            repeat: false,
            speed: 1.0,
            elapsed: 0.0,
            animation_clip: Default::default(),
            path_cache: Vec::new(),
        }
    }
}

impl AnimationPlayer {
    /// Start playing an animation, resetting state of the player
    pub fn start(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        *self = Self {
            animation_clip: handle,
            ..Default::default()
        };
        self
    }

    /// Start playing an animation, resetting state of the player, unless the requested animation is already playing.
    pub fn play(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        if self.animation_clip != handle || self.is_paused() {
            self.start(handle);
        }
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

fn find_bone(
    root: Entity,
    path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
    path_cache: &mut Vec<Option<Entity>>,
) -> Option<Entity> {
    // PERF: finding the target entity can be optimised
    let mut current_entity = root;
    path_cache.resize(path.parts.len(), None);
    // Ignore the first name, it is the root node which we already have
    for (idx, part) in path.parts.iter().enumerate().skip(1) {
        let mut found = false;
        let children = children.get(current_entity).ok()?;
        if let Some(cached) = path_cache[idx] {
            if children.contains(&cached) {
                if let Ok(name) = names.get(cached) {
                    if name == part {
                        current_entity = cached;
                        found = true;
                    }
                }
            }
        }
        if !found {
            for child in children.deref() {
                if let Ok(name) = names.get(*child) {
                    if name == part {
                        // Found a children with the right name, continue to the next part
                        current_entity = *child;
                        path_cache[idx] = Some(*child);
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            warn!("Entity not found for path {:?} on part {:?}", path, part);
            return None;
        }
    }
    Some(current_entity)
}

/// Verify that there are no ancestors of a given entity that have an `AnimationPlayer`.
fn verify_no_ancestor_player(
    player_parent: Option<&Parent>,
    parents: &Query<(Option<With<AnimationPlayer>>, Option<&Parent>)>,
) -> bool {
    let Some(mut current) = player_parent.map(Parent::get) else { return true };
    loop {
        let Ok((maybe_player, parent)) = parents.get(current) else { return true };
        if maybe_player.is_some() {
            return false;
        }
        if let Some(parent) = parent {
            current = parent.get();
        } else {
            return true;
        }
    }
}

/// System that will play all animations, using any entity with a [`AnimationPlayer`]
/// and a [`Handle<AnimationClip>`] as an animation root
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&mut Transform>,
    parents: Query<(Option<With<AnimationPlayer>>, Option<&Parent>)>,
    mut animation_players: Query<(Entity, Option<&Parent>, &mut AnimationPlayer)>,
) {
    animation_players.par_for_each_mut(10, |(root, maybe_parent, mut player)| {
        let Some(animation_clip) = animations.get(&player.animation_clip) else { return };
        // Continue if paused unless the `AnimationPlayer` was changed
        // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
        if player.paused && !player.is_changed() {
            return;
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
        if player.path_cache.len() != animation_clip.paths.len() {
            player.path_cache = vec![Vec::new(); animation_clip.paths.len()];
        }
        if !verify_no_ancestor_player(maybe_parent, &parents) {
            warn!("Animation player on {:?} has a conflicting animation player on an ancestor. Cannot safely animate.", root);
            return;
        }
        for (path, bone_id) in &animation_clip.paths {
            let cached_path = &mut player.path_cache[*bone_id];
            let curves = animation_clip.get_curves(*bone_id).unwrap();
            let Some(target) = find_bone(root, path, &children, &names, cached_path) else { continue };
            // SAFETY: The verify_no_ancestor_player check above ensures that two animation players cannot alias
            // any of their descendant Transforms.
            // 
            // The system scheduler prevents any other system from mutating Transforms at the same time, 
            // so the only way this fetch can alias is if two AnimationPlayers are targetting the same bone.
            // This can only happen if there are two or more AnimationPlayers are ancestors to the same
            // entities. By verifying that there is no other AnimationPlayer in the ancestors of a
            // running AnimationPlayer before animating any entity, this fetch cannot alias.
            //
            // This means only the AnimationPlayers closest to the root of the hierarchy will be able
            // to run their animation. Any players in the children or descendants will log a warning
            // and do nothing.
            let Ok(mut transform) = (unsafe { transforms.get_unchecked(target) }) else { continue };
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
                    Ok(n) if n >= curve.keyframe_timestamps.len() - 1 => continue, // this curve is finished
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
    });
}

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin {}

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.add_asset::<AnimationClip>()
            .register_asset_reflect::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .add_system_to_stage(
                CoreStage::PostUpdate,
                animation_player.before(TransformSystem::TransformPropagate),
            );
    }
}
