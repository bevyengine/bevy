//! Animation for the game engine Bevy

#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

use std::ops::Deref;
use std::time::Duration;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_math::{Quat, Vec3};
use bevy_reflect::Reflect;
use bevy_render::mesh::morph::MorphWeights;
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
#[derive(Reflect, Clone, Debug)]
pub enum Keyframes {
    /// Keyframes for rotation.
    Rotation(Vec<Quat>),
    /// Keyframes for translation.
    Translation(Vec<Vec3>),
    /// Keyframes for scale.
    Scale(Vec<Vec3>),
    /// Keyframes for morph target weights.
    ///
    /// Note that in `.0`, each contiguous `target_count` values is a single
    /// keyframe representing the weight values at given keyframe.
    ///
    /// This follows the [glTF design].
    ///
    /// [glTF design]: https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#animations
    Weights(Vec<f32>),
}

/// Describes how an attribute of a [`Transform`] or [`MorphWeights`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Reflect, Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    pub keyframes: Keyframes,
}

/// Path to an entity, with [`Name`]s. Each entity in a path must have a name.
#[derive(Reflect, Clone, Debug, Hash, PartialEq, Eq, Default)]
pub struct EntityPath {
    /// Parts of the path
    pub parts: Vec<Name>,
}

/// A list of [`VariableCurve`], and the [`EntityPath`] to which they apply.
#[derive(Asset, Reflect, Clone, Debug, Default)]
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

    /// Whether this animation clip can run on entity with given [`Name`].
    pub fn compatible_with(&self, name: &Name) -> bool {
        self.paths.keys().any(|path| &path.parts[0] == name)
    }
}

/// Repetition behavior of an animation.
#[derive(Reflect, Copy, Clone, Default)]
pub enum RepeatAnimation {
    /// The animation will finish after running once.
    #[default]
    Never,
    /// The animation will finish after running "n" times.
    Count(u32),
    /// The animation will never finish.
    Forever,
}

#[derive(Reflect)]
struct PlayingAnimation {
    repeat: RepeatAnimation,
    speed: f32,
    /// Total time the animation has been played.
    ///
    /// Note: Time does not increase when the animation is paused or after it has completed.
    elapsed: f32,
    /// The timestamp inside of the animation clip.
    ///
    /// Note: This will always be in the range [0.0, animation clip duration]
    seek_time: f32,
    animation_clip: Handle<AnimationClip>,
    path_cache: Vec<Vec<Option<Entity>>>,
    /// Number of times the animation has completed.
    /// If the animation is playing in reverse, this increments when the animation passes the start.
    completions: u32,
}

impl Default for PlayingAnimation {
    fn default() -> Self {
        Self {
            repeat: RepeatAnimation::default(),
            speed: 1.0,
            elapsed: 0.0,
            seek_time: 0.0,
            animation_clip: Default::default(),
            path_cache: Vec::new(),
            completions: 0,
        }
    }
}

impl PlayingAnimation {
    /// Check if the animation has finished, based on its repetition behavior and the number of times it has repeated.
    ///
    /// Note: An animation with `RepeatAnimation::Forever` will never finish.
    #[inline]
    pub fn is_finished(&self) -> bool {
        match self.repeat {
            RepeatAnimation::Forever => false,
            RepeatAnimation::Never => self.completions >= 1,
            RepeatAnimation::Count(n) => self.completions >= n,
        }
    }

    /// Update the animation given the delta time and the duration of the clip being played.
    #[inline]
    fn update(&mut self, delta: f32, clip_duration: f32) {
        if self.is_finished() {
            return;
        }

        self.elapsed += delta;
        self.seek_time += delta * self.speed;

        if (self.seek_time > clip_duration && self.speed > 0.0)
            || (self.seek_time < 0.0 && self.speed < 0.0)
        {
            self.completions += 1;
        }

        if self.seek_time >= clip_duration {
            self.seek_time %= clip_duration;
        }
        if self.seek_time < 0.0 {
            self.seek_time += clip_duration;
        }
    }

    /// Reset back to the initial state as if no time has elapsed.
    fn replay(&mut self) {
        self.completions = 0;
        self.elapsed = 0.0;
        self.seek_time = 0.0;
    }
}

/// An animation that is being faded out as part of a transition
struct AnimationTransition {
    /// The current weight. Starts at 1.0 and goes to 0.0 during the fade-out.
    current_weight: f32,
    /// How much to decrease `current_weight` per second
    weight_decline_per_sec: f32,
    /// The animation that is being faded out
    animation: PlayingAnimation,
}

/// Animation controls
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayer {
    paused: bool,

    animation: PlayingAnimation,

    // List of previous animations we're currently transitioning away from.
    // Usually this is empty, when transitioning between animations, there is
    // one entry. When another animation transition happens while a transition
    // is still ongoing, then there can be more than one entry.
    // Once a transition is finished, it will be automatically removed from the list
    #[reflect(ignore)]
    transitions: Vec<AnimationTransition>,
}

impl AnimationPlayer {
    /// Start playing an animation, resetting state of the player.
    /// This will use a linear blending between the previous and the new animation to make a smooth transition.
    pub fn start(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        self.animation = PlayingAnimation {
            animation_clip: handle,
            ..Default::default()
        };

        // We want a hard transition.
        // In case any previous transitions are still playing, stop them
        self.transitions.clear();

        self
    }

    /// Start playing an animation, resetting state of the player.
    /// This will use a linear blending between the previous and the new animation to make a smooth transition.
    pub fn start_with_transition(
        &mut self,
        handle: Handle<AnimationClip>,
        transition_duration: Duration,
    ) -> &mut Self {
        let mut animation = PlayingAnimation {
            animation_clip: handle,
            ..Default::default()
        };
        std::mem::swap(&mut animation, &mut self.animation);

        // Add the current transition. If other transitions are still ongoing,
        // this will keep those transitions running and cause a transition between
        // the output of that previous transition to the new animation.
        self.transitions.push(AnimationTransition {
            current_weight: 1.0,
            weight_decline_per_sec: 1.0 / transition_duration.as_secs_f32(),
            animation,
        });

        self
    }

    /// Start playing an animation, resetting state of the player, unless the requested animation is already playing.
    /// If `transition_duration` is set, this will use a linear blending
    /// between the previous and the new animation to make a smooth transition
    pub fn play(&mut self, handle: Handle<AnimationClip>) -> &mut Self {
        if !self.is_playing_clip(&handle) || self.is_paused() {
            self.start(handle);
        }
        self
    }

    /// Start playing an animation, resetting state of the player, unless the requested animation is already playing.
    /// This will use a linear blending between the previous and the new animation to make a smooth transition
    pub fn play_with_transition(
        &mut self,
        handle: Handle<AnimationClip>,
        transition_duration: Duration,
    ) -> &mut Self {
        if !self.is_playing_clip(&handle) || self.is_paused() {
            self.start_with_transition(handle, transition_duration);
        }
        self
    }

    /// Handle to the animation clip being played.
    pub fn animation_clip(&self) -> &Handle<AnimationClip> {
        &self.animation.animation_clip
    }

    /// Check if the given animation clip is being played.
    pub fn is_playing_clip(&self, handle: &Handle<AnimationClip>) -> bool {
        self.animation_clip() == handle
    }

    /// Check if the playing animation has finished, according to the repetition behavior.
    pub fn is_finished(&self) -> bool {
        self.animation.is_finished()
    }

    /// Sets repeat to [`RepeatAnimation::Forever`].
    ///
    /// See also [`Self::set_repeat`].
    pub fn repeat(&mut self) -> &mut Self {
        self.animation.repeat = RepeatAnimation::Forever;
        self
    }

    /// Set the repetition behaviour of the animation.
    pub fn set_repeat(&mut self, repeat: RepeatAnimation) -> &mut Self {
        self.animation.repeat = repeat;
        self
    }

    /// Repetition behavior of the animation.
    pub fn repeat_mode(&self) -> RepeatAnimation {
        self.animation.repeat
    }

    /// Number of times the animation has completed.
    pub fn completions(&self) -> u32 {
        self.animation.completions
    }

    /// Check if the animation is playing in reverse.
    pub fn is_playback_reversed(&self) -> bool {
        self.animation.speed < 0.0
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
        self.animation.speed
    }

    /// Set the speed of the animation playback
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.animation.speed = speed;
        self
    }

    /// Time elapsed playing the animation
    pub fn elapsed(&self) -> f32 {
        self.animation.elapsed
    }

    /// Seek time inside of the animation. Always within the range [0.0, clip duration].
    pub fn seek_time(&self) -> f32 {
        self.animation.seek_time
    }

    /// Seek to a specific time in the animation.
    pub fn seek_to(&mut self, seek_time: f32) -> &mut Self {
        self.animation.seek_time = seek_time;
        self
    }

    /// Reset the animation to its initial state, as if no time has elapsed.
    pub fn replay(&mut self) {
        self.animation.replay();
    }
}

fn entity_from_path(
    root: Entity,
    path: &EntityPath,
    children: &Query<&Children>,
    names: &Query<&Name>,
    path_cache: &mut Vec<Option<Entity>>,
) -> Option<Entity> {
    // PERF: finding the target entity can be optimised
    let mut current_entity = root;
    path_cache.resize(path.parts.len(), None);

    let mut parts = path.parts.iter().enumerate();

    // check the first name is the root node which we already have
    let Some((_, root_name)) = parts.next() else {
        return None;
    };
    if names.get(current_entity) != Ok(root_name) {
        return None;
    }

    for (idx, part) in parts {
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

/// Verify that there are no ancestors of a given entity that have an [`AnimationPlayer`].
fn verify_no_ancestor_player(
    player_parent: Option<&Parent>,
    parents: &Query<(Has<AnimationPlayer>, Option<&Parent>)>,
) -> bool {
    let Some(mut current) = player_parent.map(Parent::get) else {
        return true;
    };
    loop {
        let Ok((has_player, parent)) = parents.get(current) else {
            return true;
        };
        if has_player {
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
#[allow(clippy::too_many_arguments)]
pub fn animation_player(
    time: Res<Time>,
    animations: Res<Assets<AnimationClip>>,
    children: Query<&Children>,
    names: Query<&Name>,
    transforms: Query<&mut Transform>,
    morphs: Query<&mut MorphWeights>,
    parents: Query<(Has<AnimationPlayer>, Option<&Parent>)>,
    mut animation_players: Query<(Entity, Option<&Parent>, &mut AnimationPlayer)>,
) {
    animation_players
        .par_iter_mut()
        .for_each(|(root, maybe_parent, mut player)| {
            update_transitions(&mut player, &time);
            run_animation_player(
                root,
                player,
                &time,
                &animations,
                &names,
                &transforms,
                &morphs,
                maybe_parent,
                &parents,
                &children,
            );
        });
}

#[allow(clippy::too_many_arguments)]
fn run_animation_player(
    root: Entity,
    mut player: Mut<AnimationPlayer>,
    time: &Time,
    animations: &Assets<AnimationClip>,
    names: &Query<&Name>,
    transforms: &Query<&mut Transform>,
    morphs: &Query<&mut MorphWeights>,
    maybe_parent: Option<&Parent>,
    parents: &Query<(Has<AnimationPlayer>, Option<&Parent>)>,
    children: &Query<&Children>,
) {
    let paused = player.paused;
    // Continue if paused unless the `AnimationPlayer` was changed
    // This allow the animation to still be updated if the player.elapsed field was manually updated in pause
    if paused && !player.is_changed() {
        return;
    }

    // Apply the main animation
    apply_animation(
        1.0,
        &mut player.animation,
        paused,
        root,
        time,
        animations,
        names,
        transforms,
        morphs,
        maybe_parent,
        parents,
        children,
    );

    // Apply any potential fade-out transitions from previous animations
    for AnimationTransition {
        current_weight,
        animation,
        ..
    } in &mut player.transitions
    {
        apply_animation(
            *current_weight,
            animation,
            paused,
            root,
            time,
            animations,
            names,
            transforms,
            morphs,
            maybe_parent,
            parents,
            children,
        );
    }
}

/// Update `weights` based on weights in `keyframe` with a linear interpolation
/// on `key_lerp`.
fn lerp_morph_weights(weights: &mut [f32], keyframe: impl Iterator<Item = f32>, key_lerp: f32) {
    let zipped = weights.iter_mut().zip(keyframe);
    for (morph_weight, keyframe) in zipped {
        *morph_weight += (keyframe - *morph_weight) * key_lerp;
    }
}

/// Extract a keyframe from a list of keyframes by index.
///
/// # Panics
///
/// When `key_index * target_count` is larger than `keyframes`
///
/// This happens when `keyframes` is not formatted as described in
/// [`Keyframes::Weights`]. A possible cause is [`AnimationClip`] not being
/// meant to be used for the [`MorphWeights`] of the entity it's being applied to.
fn get_keyframe(target_count: usize, keyframes: &[f32], key_index: usize) -> &[f32] {
    let start = target_count * key_index;
    let end = target_count * (key_index + 1);
    &keyframes[start..end]
}

#[allow(clippy::too_many_arguments)]
fn apply_animation(
    weight: f32,
    animation: &mut PlayingAnimation,
    paused: bool,
    root: Entity,
    time: &Time,
    animations: &Assets<AnimationClip>,
    names: &Query<&Name>,
    transforms: &Query<&mut Transform>,
    morphs: &Query<&mut MorphWeights>,
    maybe_parent: Option<&Parent>,
    parents: &Query<(Has<AnimationPlayer>, Option<&Parent>)>,
    children: &Query<&Children>,
) {
    if let Some(animation_clip) = animations.get(&animation.animation_clip) {
        // We don't return early because seek_to() may have been called on the animation player.
        animation.update(
            if paused { 0.0 } else { time.delta_seconds() },
            animation_clip.duration,
        );

        if animation.path_cache.len() != animation_clip.paths.len() {
            animation.path_cache = vec![Vec::new(); animation_clip.paths.len()];
        }
        if !verify_no_ancestor_player(maybe_parent, parents) {
            warn!("Animation player on {:?} has a conflicting animation player on an ancestor. Cannot safely animate.", root);
            return;
        }

        let mut any_path_found = false;
        for (path, bone_id) in &animation_clip.paths {
            let cached_path = &mut animation.path_cache[*bone_id];
            let curves = animation_clip.get_curves(*bone_id).unwrap();
            let Some(target) = entity_from_path(root, path, children, names, cached_path) else {
                continue;
            };
            any_path_found = true;
            // SAFETY: The verify_no_ancestor_player check above ensures that two animation players cannot alias
            // any of their descendant Transforms.
            //
            // The system scheduler prevents any other system from mutating Transforms at the same time,
            // so the only way this fetch can alias is if two AnimationPlayers are targeting the same bone.
            // This can only happen if there are two or more AnimationPlayers are ancestors to the same
            // entities. By verifying that there is no other AnimationPlayer in the ancestors of a
            // running AnimationPlayer before animating any entity, this fetch cannot alias.
            //
            // This means only the AnimationPlayers closest to the root of the hierarchy will be able
            // to run their animation. Any players in the children or descendants will log a warning
            // and do nothing.
            let Ok(mut transform) = (unsafe { transforms.get_unchecked(target) }) else {
                continue;
            };
            let mut morphs = unsafe { morphs.get_unchecked(target) };
            for curve in curves {
                // Some curves have only one keyframe used to set a transform
                if curve.keyframe_timestamps.len() == 1 {
                    match &curve.keyframes {
                        Keyframes::Rotation(keyframes) => {
                            transform.rotation = transform.rotation.slerp(keyframes[0], weight);
                        }
                        Keyframes::Translation(keyframes) => {
                            transform.translation =
                                transform.translation.lerp(keyframes[0], weight);
                        }
                        Keyframes::Scale(keyframes) => {
                            transform.scale = transform.scale.lerp(keyframes[0], weight);
                        }
                        Keyframes::Weights(keyframes) => {
                            if let Ok(morphs) = &mut morphs {
                                let target_count = morphs.weights().len();
                                lerp_morph_weights(
                                    morphs.weights_mut(),
                                    get_keyframe(target_count, keyframes, 0).iter().copied(),
                                    weight,
                                );
                            }
                        }
                    }
                    continue;
                }

                // Find the current keyframe
                // PERF: finding the current keyframe can be optimised
                let step_start = match curve
                    .keyframe_timestamps
                    .binary_search_by(|probe| probe.partial_cmp(&animation.seek_time).unwrap())
                {
                    Ok(n) if n >= curve.keyframe_timestamps.len() - 1 => continue, // this curve is finished
                    Ok(i) => i,
                    Err(0) => continue, // this curve isn't started yet
                    Err(n) if n > curve.keyframe_timestamps.len() - 1 => continue, // this curve is finished
                    Err(i) => i - 1,
                };
                let ts_start = curve.keyframe_timestamps[step_start];
                let ts_end = curve.keyframe_timestamps[step_start + 1];
                let lerp = (animation.seek_time - ts_start) / (ts_end - ts_start);

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
                        let rot = rot_start.normalize().slerp(rot_end.normalize(), lerp);
                        transform.rotation = transform.rotation.slerp(rot, weight);
                    }
                    Keyframes::Translation(keyframes) => {
                        let translation_start = keyframes[step_start];
                        let translation_end = keyframes[step_start + 1];
                        let result = translation_start.lerp(translation_end, lerp);
                        transform.translation = transform.translation.lerp(result, weight);
                    }
                    Keyframes::Scale(keyframes) => {
                        let scale_start = keyframes[step_start];
                        let scale_end = keyframes[step_start + 1];
                        let result = scale_start.lerp(scale_end, lerp);
                        transform.scale = transform.scale.lerp(result, weight);
                    }
                    Keyframes::Weights(keyframes) => {
                        if let Ok(morphs) = &mut morphs {
                            let target_count = morphs.weights().len();
                            let morph_start = get_keyframe(target_count, keyframes, step_start);
                            let morph_end = get_keyframe(target_count, keyframes, step_start + 1);
                            let result = morph_start
                                .iter()
                                .zip(morph_end)
                                .map(|(a, b)| *a + lerp * (*b - *a));
                            lerp_morph_weights(morphs.weights_mut(), result, weight);
                        }
                    }
                }
            }
        }

        if !any_path_found {
            warn!("Animation player on {root:?} did not match any entity paths.");
        }
    }
}

fn update_transitions(player: &mut AnimationPlayer, time: &Time) {
    player.transitions.retain_mut(|animation| {
        animation.current_weight -= animation.weight_decline_per_sec * time.delta_seconds();
        animation.current_weight > 0.0
    });
}

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AnimationClip>()
            .register_asset_reflect::<AnimationClip>()
            .register_type::<AnimationPlayer>()
            .add_systems(
                PostUpdate,
                animation_player.before(TransformSystem::TransformPropagate),
            );
    }
}
