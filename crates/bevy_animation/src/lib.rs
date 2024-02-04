//! Animation for the game engine Bevy

mod animatable;
mod util;

use std::ops::{Add, Deref, Mul};
use std::time::Duration;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_math::{FloatExt, Quat, Vec3};
use bevy_reflect::Reflect;
use bevy_render::mesh::morph::MorphWeights;
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::{tracing::warn, HashMap};

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        animatable::*, AnimationClip, AnimationPlayer, AnimationPlugin, EntityPath, Interpolation,
        Keyframes, VariableCurve,
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

impl Keyframes {
    /// Returns the number of keyframes.
    pub fn len(&self) -> usize {
        match self {
            Keyframes::Weights(vec) => vec.len(),
            Keyframes::Translation(vec) | Keyframes::Scale(vec) => vec.len(),
            Keyframes::Rotation(vec) => vec.len(),
        }
    }

    /// Returns true if the number of keyframes is zero.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Describes how an attribute of a [`Transform`] or [`MorphWeights`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Reflect, Clone, Debug)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    ///
    /// The representation will depend on the interpolation type of this curve:
    ///
    /// - for `Interpolation::Step` and `Interpolation::Linear`, each keyframe is a single value
    /// - for `Interpolation::CubicSpline`, each keyframe is made of three values for `tangent_in`,
    /// `keyframe_value` and `tangent_out`
    pub keyframes: Keyframes,
    /// Interpolation method to use between keyframes.
    pub interpolation: Interpolation,
}

impl VariableCurve {
    /// Find the index of the keyframe at or before the current time.
    ///
    /// Returns [`None`] if the curve is finished or not yet started.
    /// To be more precise, this returns [`None`] if the frame is at or past the last keyframe:
    /// we cannot get the *next* keyframe to interpolate to in that case.
    pub fn find_current_keyframe(&self, seek_time: f32) -> Option<usize> {
        // An Ok(keyframe_index) result means an exact result was found by binary search
        // An Err result means the keyframe was not found, and the index is the keyframe
        // PERF: finding the current keyframe can be optimised
        let search_result = self
            .keyframe_timestamps
            .binary_search_by(|probe| probe.partial_cmp(&seek_time).unwrap());

        // Subtract one for zero indexing!
        let last_keyframe = self.keyframes.len() - 1;

        // We want to find the index of the keyframe before the current time
        // If the keyframe is past the second-to-last keyframe, the animation cannot be interpolated.
        let step_start = match search_result {
            // An exact match was found, and it is the last keyframe (or something has gone terribly wrong).
            // This means that the curve is finished.
            Ok(n) if n >= last_keyframe => return None,
            // An exact match was found, and it is not the last keyframe.
            Ok(i) => i,
            // No exact match was found, and the seek_time is before the start of the animation.
            // This occurs because the binary search returns the index of where we could insert a value
            // without disrupting the order of the vector.
            // If the value is less than the first element, the index will be 0.
            Err(0) => return None,
            // No exact match was found, and it was after the last keyframe.
            // The curve is finished.
            Err(n) if n > last_keyframe => return None,
            // No exact match was found, so return the previous keyframe to interpolate from.
            Err(i) => i - 1,
        };

        // Consumers need to be able to interpolate between the return keyframe and the next
        assert!(step_start < self.keyframe_timestamps.len());

        Some(step_start)
    }
}

/// Interpolation method to use between keyframes.
#[derive(Reflect, Clone, Debug)]
pub enum Interpolation {
    /// Linear interpolation between the two closest keyframes.
    Linear,
    /// Step interpolation, the value of the start keyframe is used.
    Step,
    /// Cubic spline interpolation. The value of the two closest keyframes is used, with the out
    /// tangent of the start keyframe and the in tangent of the end keyframe.
    CubicSpline,
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
#[derive(Reflect, Debug, PartialEq, Eq, Copy, Clone, Default)]
pub enum RepeatAnimation {
    /// The animation will finish after running once.
    #[default]
    Never,
    /// The animation will finish after running "n" times.
    Count(u32),
    /// The animation will never finish.
    Forever,
}

#[derive(Debug, Reflect)]
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

        let over_time = self.speed > 0.0 && self.seek_time >= clip_duration;
        let under_time = self.speed < 0.0 && self.seek_time < 0.0;

        if over_time || under_time {
            self.completions += 1;

            if self.is_finished() {
                return;
            }
        }
        if self.seek_time >= clip_duration {
            self.seek_time %= clip_duration;
        }
        // Note: assumes delta is never lower than -clip_duration
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
        *morph_weight = morph_weight.lerp(keyframe, key_lerp);
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

/// Helper function for cubic spline interpolation.
fn cubic_spline_interpolation<T>(
    value_start: T,
    tangent_out_start: T,
    tangent_in_end: T,
    value_end: T,
    lerp: f32,
    step_duration: f32,
) -> T
where
    T: Mul<f32, Output = T> + Add<Output = T>,
{
    value_start * (2.0 * lerp.powi(3) - 3.0 * lerp.powi(2) + 1.0)
        + tangent_out_start * (step_duration) * (lerp.powi(3) - 2.0 * lerp.powi(2) + lerp)
        + value_end * (-2.0 * lerp.powi(3) + 3.0 * lerp.powi(2))
        + tangent_in_end * step_duration * (lerp.powi(3) - lerp.powi(2))
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
    let Some(animation_clip) = animations.get(&animation.animation_clip) else {
        return;
    };

    // We don't return early because seek_to() may have been called on the animation player.
    animation.update(
        if paused { 0.0 } else { time.delta_seconds() },
        animation_clip.duration,
    );

    if animation.path_cache.len() != animation_clip.paths.len() {
        let new_len = animation_clip.paths.len();
        animation.path_cache.iter_mut().for_each(|v| v.clear());
        animation.path_cache.resize_with(new_len, Vec::new);
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
        // SAFETY: As above, there can't be other AnimationPlayers with this target so this fetch can't alias
        let mut morphs = unsafe { morphs.get_unchecked(target) }.ok();
        for curve in curves {
            // Some curves have only one keyframe used to set a transform
            if curve.keyframe_timestamps.len() == 1 {
                match &curve.keyframes {
                    Keyframes::Rotation(keyframes) => {
                        transform.rotation = transform.rotation.slerp(keyframes[0], weight);
                    }
                    Keyframes::Translation(keyframes) => {
                        transform.translation = transform.translation.lerp(keyframes[0], weight);
                    }
                    Keyframes::Scale(keyframes) => {
                        transform.scale = transform.scale.lerp(keyframes[0], weight);
                    }
                    Keyframes::Weights(keyframes) => {
                        if let Some(morphs) = &mut morphs {
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
            let Some(step_start) = curve.find_current_keyframe(animation.seek_time) else {
                continue;
            };

            let timestamp_start = curve.keyframe_timestamps[step_start];
            let timestamp_end = curve.keyframe_timestamps[step_start + 1];
            // Compute how far we are through the keyframe, normalized to [0, 1]
            let lerp = f32::inverse_lerp(timestamp_start, timestamp_end, animation.seek_time);

            apply_keyframe(
                curve,
                step_start,
                weight,
                lerp,
                timestamp_end - timestamp_start,
                &mut transform,
                &mut morphs,
            );
        }
    }

    if !any_path_found {
        warn!("Animation player on {root:?} did not match any entity paths.");
    }
}

#[inline(always)]
fn apply_keyframe(
    curve: &VariableCurve,
    step_start: usize,
    weight: f32,
    lerp: f32,
    duration: f32,
    transform: &mut Mut<Transform>,
    morphs: &mut Option<Mut<MorphWeights>>,
) {
    match (&curve.interpolation, &curve.keyframes) {
        (Interpolation::Step, Keyframes::Rotation(keyframes)) => {
            transform.rotation = transform.rotation.slerp(keyframes[step_start], weight);
        }
        (Interpolation::Linear, Keyframes::Rotation(keyframes)) => {
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
        (Interpolation::CubicSpline, Keyframes::Rotation(keyframes)) => {
            let value_start = keyframes[step_start * 3 + 1];
            let tangent_out_start = keyframes[step_start * 3 + 2];
            let tangent_in_end = keyframes[(step_start + 1) * 3];
            let value_end = keyframes[(step_start + 1) * 3 + 1];
            let result = cubic_spline_interpolation(
                value_start,
                tangent_out_start,
                tangent_in_end,
                value_end,
                lerp,
                duration,
            );
            transform.rotation = transform.rotation.slerp(result.normalize(), weight);
        }
        (Interpolation::Step, Keyframes::Translation(keyframes)) => {
            transform.translation = transform.translation.lerp(keyframes[step_start], weight);
        }
        (Interpolation::Linear, Keyframes::Translation(keyframes)) => {
            let translation_start = keyframes[step_start];
            let translation_end = keyframes[step_start + 1];
            let result = translation_start.lerp(translation_end, lerp);
            transform.translation = transform.translation.lerp(result, weight);
        }
        (Interpolation::CubicSpline, Keyframes::Translation(keyframes)) => {
            let value_start = keyframes[step_start * 3 + 1];
            let tangent_out_start = keyframes[step_start * 3 + 2];
            let tangent_in_end = keyframes[(step_start + 1) * 3];
            let value_end = keyframes[(step_start + 1) * 3 + 1];
            let result = cubic_spline_interpolation(
                value_start,
                tangent_out_start,
                tangent_in_end,
                value_end,
                lerp,
                duration,
            );
            transform.translation = transform.translation.lerp(result, weight);
        }
        (Interpolation::Step, Keyframes::Scale(keyframes)) => {
            transform.scale = transform.scale.lerp(keyframes[step_start], weight);
        }
        (Interpolation::Linear, Keyframes::Scale(keyframes)) => {
            let scale_start = keyframes[step_start];
            let scale_end = keyframes[step_start + 1];
            let result = scale_start.lerp(scale_end, lerp);
            transform.scale = transform.scale.lerp(result, weight);
        }
        (Interpolation::CubicSpline, Keyframes::Scale(keyframes)) => {
            let value_start = keyframes[step_start * 3 + 1];
            let tangent_out_start = keyframes[step_start * 3 + 2];
            let tangent_in_end = keyframes[(step_start + 1) * 3];
            let value_end = keyframes[(step_start + 1) * 3 + 1];
            let result = cubic_spline_interpolation(
                value_start,
                tangent_out_start,
                tangent_in_end,
                value_end,
                lerp,
                duration,
            );
            transform.scale = transform.scale.lerp(result, weight);
        }
        (Interpolation::Step, Keyframes::Weights(keyframes)) => {
            if let Some(morphs) = morphs {
                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start);
                lerp_morph_weights(morphs.weights_mut(), morph_start.iter().copied(), weight);
            }
        }
        (Interpolation::Linear, Keyframes::Weights(keyframes)) => {
            if let Some(morphs) = morphs {
                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start);
                let morph_end = get_keyframe(target_count, keyframes, step_start + 1);
                let result = morph_start
                    .iter()
                    .zip(morph_end)
                    .map(|(a, b)| a.lerp(*b, lerp));
                lerp_morph_weights(morphs.weights_mut(), result, weight);
            }
        }
        (Interpolation::CubicSpline, Keyframes::Weights(keyframes)) => {
            if let Some(morphs) = morphs {
                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start * 3 + 1);
                let tangents_out_start = get_keyframe(target_count, keyframes, step_start * 3 + 2);
                let tangents_in_end = get_keyframe(target_count, keyframes, (step_start + 1) * 3);
                let morph_end = get_keyframe(target_count, keyframes, (step_start + 1) * 3 + 1);
                let result = morph_start
                    .iter()
                    .zip(tangents_out_start)
                    .zip(tangents_in_end)
                    .zip(morph_end)
                    .map(
                        |(((&value_start, &tangent_out_start), &tangent_in_end), &value_end)| {
                            cubic_spline_interpolation(
                                value_start,
                                tangent_out_start,
                                tangent_in_end,
                                value_end,
                                lerp,
                                duration,
                            )
                        },
                    );
                lerp_morph_weights(morphs.weights_mut(), result, weight);
            }
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

#[cfg(test)]
mod tests {
    use crate::VariableCurve;
    use bevy_math::Vec3;

    fn test_variable_curve() -> VariableCurve {
        let keyframe_timestamps = vec![1.0, 2.0, 3.0, 4.0];
        let keyframes = vec![
            Vec3::ONE * 0.0,
            Vec3::ONE * 3.0,
            Vec3::ONE * 6.0,
            Vec3::ONE * 9.0,
        ];
        let interpolation = crate::Interpolation::Linear;

        let variable_curve = VariableCurve {
            keyframe_timestamps,
            keyframes: crate::Keyframes::Translation(keyframes),
            interpolation,
        };

        assert!(variable_curve.keyframe_timestamps.len() == variable_curve.keyframes.len());

        // f32 doesn't impl Ord so we can't easily sort it
        let mut maybe_last_timestamp = None;
        for current_timestamp in &variable_curve.keyframe_timestamps {
            assert!(current_timestamp.is_finite());

            if let Some(last_timestamp) = maybe_last_timestamp {
                assert!(current_timestamp > last_timestamp);
            }
            maybe_last_timestamp = Some(current_timestamp);
        }

        variable_curve
    }

    #[test]
    fn find_current_keyframe_is_in_bounds() {
        let curve = test_variable_curve();
        let min_time = *curve.keyframe_timestamps.first().unwrap();
        // We will always get none at times at or past the second last keyframe
        let second_last_keyframe = curve.keyframe_timestamps.len() - 2;
        let max_time = curve.keyframe_timestamps[second_last_keyframe];
        let elapsed_time = max_time - min_time;

        let n_keyframes = curve.keyframe_timestamps.len();
        let n_test_points = 5;

        for i in 0..=n_test_points {
            // Get a value between 0 and 1
            let normalized_time = i as f32 / n_test_points as f32;
            let seek_time = min_time + normalized_time * elapsed_time;
            assert!(seek_time >= min_time);
            assert!(seek_time <= max_time);

            let maybe_current_keyframe = curve.find_current_keyframe(seek_time);
            assert!(
                maybe_current_keyframe.is_some(),
                "Seek time: {seek_time}, Min time: {min_time}, Max time: {max_time}"
            );

            // We cannot return the last keyframe,
            // because we want to interpolate between the current and next keyframe
            assert!(maybe_current_keyframe.unwrap() < n_keyframes);
        }
    }

    #[test]
    fn find_current_keyframe_returns_none_on_unstarted_animations() {
        let curve = test_variable_curve();
        let min_time = *curve.keyframe_timestamps.first().unwrap();
        let seek_time = 0.0;
        assert!(seek_time < min_time);

        let maybe_keyframe = curve.find_current_keyframe(seek_time);
        assert!(
            maybe_keyframe.is_none(),
            "Seek time: {seek_time}, Minimum time: {min_time}"
        );
    }

    #[test]
    fn find_current_keyframe_returns_none_on_finished_animation() {
        let curve = test_variable_curve();
        let max_time = *curve.keyframe_timestamps.last().unwrap();

        assert!(max_time < f32::INFINITY);
        let maybe_keyframe = curve.find_current_keyframe(f32::INFINITY);
        assert!(maybe_keyframe.is_none());

        let maybe_keyframe = curve.find_current_keyframe(max_time);
        assert!(maybe_keyframe.is_none());
    }

    #[test]
    fn second_last_keyframe_is_found_correctly() {
        let curve = test_variable_curve();

        // Exact time match
        let second_last_keyframe = curve.keyframe_timestamps.len() - 2;
        let second_last_time = curve.keyframe_timestamps[second_last_keyframe];
        let maybe_keyframe = curve.find_current_keyframe(second_last_time);
        assert!(maybe_keyframe.unwrap() == second_last_keyframe);

        // Inexact match, between the last and second last frames
        let seek_time = second_last_time + 0.001;
        let last_time = curve.keyframe_timestamps[second_last_keyframe + 1];
        assert!(seek_time < last_time);

        let maybe_keyframe = curve.find_current_keyframe(seek_time);
        assert!(maybe_keyframe.unwrap() == second_last_keyframe);
    }

    #[test]
    fn exact_keyframe_matches_are_found_correctly() {
        let curve = test_variable_curve();
        let second_last_keyframe = curve.keyframes.len() - 2;

        for i in 0..=second_last_keyframe {
            let seek_time = curve.keyframe_timestamps[i];

            let keyframe = curve.find_current_keyframe(seek_time).unwrap();
            assert!(keyframe == i);
        }
    }

    #[test]
    fn exact_and_inexact_keyframes_correspond() {
        let curve = test_variable_curve();

        let second_last_keyframe = curve.keyframes.len() - 2;

        for i in 0..=second_last_keyframe {
            let seek_time = curve.keyframe_timestamps[i];

            let exact_keyframe = curve.find_current_keyframe(seek_time).unwrap();

            let inexact_seek_time = seek_time + 0.0001;
            let final_time = *curve.keyframe_timestamps.last().unwrap();
            assert!(inexact_seek_time < final_time);

            let inexact_keyframe = curve.find_current_keyframe(inexact_seek_time).unwrap();

            assert!(exact_keyframe == inexact_keyframe);
        }
    }
}
