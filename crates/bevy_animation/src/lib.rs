//! Animation for the game engine Bevy

mod animatable;
mod util;

use std::ops::{Add, Mul};
use std::time::Duration;

use bevy_app::{App, Plugin, PostUpdate, Update};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;
use bevy_hierarchy::{Children, Parent};
use bevy_log::error;
use bevy_math::{FloatExt, Quat, Vec3};
use bevy_reflect::Reflect;
use bevy_render::mesh::morph::MorphWeights;
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::{EntityHashMap, EntityHashSet, HashMap};

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

#[derive(Clone, Component)]
pub struct AnimationTarget {
    root: Entity,
    path: EntityPath,
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

pub fn update_animation_targets(
    mut commands: Commands,
    candidate_targets: Query<
        (
            Entity,
            &Parent,
            Option<&Children>,
            &Name,
            Option<&AnimationTarget>,
        ),
        (
            Or<(With<Parent>, With<Children>)>,
            Or<(Changed<Parent>, Changed<Children>, Changed<Name>)>,
            Without<AnimationPlayer>,
        ),
    >,
    players: Query<(
        Entity,
        &mut AnimationPlayer,
        Option<Ref<Children>>,
        Ref<Name>,
    )>,
) {
    // Get ready to mark all dirty entities. Start with all targets that were
    // modified, as well as any children of animation players that were
    // modified.
    //
    // TODO: Make this a Local.
    let mut worklist: Vec<Entity> = candidate_targets.iter().map(|row| row.0).collect();
    for (_, _, kids, name) in players.iter() {
        if let Some(kids) = kids {
            if name.is_changed() || kids.is_changed() {
                worklist.extend(kids.iter().map(|kid| *kid));
            }
        }
    }

    // Iteratively mark all descendants of dirty entities as dirty themselves.
    // At the end of this process, `dirty_entities` will consist of a forest.
    let mut dirty_entities: EntityHashSet<Entity> = EntityHashSet::default();
    while let Some(entity) = worklist.pop() {
        if dirty_entities.contains(&entity) {
            continue;
        }
        if let Ok((_, _, Some(kids), _, _)) = candidate_targets.get(entity) {
            worklist.extend(kids.iter().map(|&kid| kid));
        }
        dirty_entities.insert(entity);
    }

    // Find the roots of the trees in the forest.
    worklist = dirty_entities
        .iter()
        .filter(|&&entity| {
            !candidate_targets
                .get(entity)
                .is_ok_and(|(_, parent, _, _, _)| dirty_entities.contains(&parent.get()))
        })
        .cloned()
        .collect();

    // Compute new `AnimationTarget` components.
    let mut new_targets: EntityHashMap<Entity, AnimationTarget> = EntityHashMap::default();
    while let Some(entity) = worklist.pop() {
        let (_, parent, kids, name, _) = candidate_targets
            .get(entity)
            .expect("Shouldn't have dirtied an entity that wasn't a target");

        // Figure out the parent path. First, if we've already computed the path
        // of the parent, just use that.
        let mut new_target = AnimationTarget::new();
        if let Some(parent_target) = new_targets.get(&parent.get()) {
            new_target = parent_target.clone();
        }

        // Otherwise, if this is a root of a tree in the forest, we know the
        // parent's `AnimationTarget` is up to date, so fetch that.
        if new_target.path.is_empty() {
            if let Ok((_, _, _, _, Some(parent_target))) = candidate_targets.get(parent.get()) {
                new_target = parent_target.clone();
            }
        }

        // If we still didn't find an `AnimationTarget` for the parent, we must be an
        // immediate child of an animation player. Handle that case.
        if new_target.path.is_empty() {
            match players.get(parent.get()) {
                Ok((_, _, _, name)) => {
                    new_target = AnimationTarget {
                        root: parent.get(),
                        path: EntityPath::from(name.clone()),
                    }
                }
                Err(_) => {
                    error!(
                        "Couldn't determine an appropriate path for {:?}. Is the tree malformed?",
                        entity
                    );
                    break;
                }
            }
        }

        // Push our own name onto the path and record it.
        new_target.path.parts.push(name.clone());
        new_targets.insert(entity, new_target);

        // Enqueue children.
        if let Some(kids) = kids {
            worklist.extend(kids);
        }
    }

    // Finally, update the `AnimationTarget` components.
    for (entity, animation_target) in new_targets.drain() {
        commands.entity(entity).insert(animation_target);
    }
}

pub fn animation_player(
    time: Res<Time>,
    animation_clips: Res<Assets<AnimationClip>>,
    mut players: Query<&mut AnimationPlayer>,
) {
    for mut player in players.iter_mut() {
        update_transitions(&mut player, &time);

        let paused = player.paused;
        if paused {
            continue;
        }

        // Update the main animation.
        update_animation(&mut player.animation, paused, &time, &animation_clips);

        // Update transition animations.
        for transition in &mut player.transitions {
            update_animation(&mut transition.animation, paused, &time, &animation_clips)
        }
    }
}

pub fn animate_targets(
    world: &mut World,
    mut system_state: Local<
        Option<
            SystemState<(
                Query<&AnimationPlayer, Without<AnimationTarget>>,
                Query<EntityMut, With<AnimationTarget>>,
            )>,
        >,
    >,
) {
    if system_state.is_none() {
        *system_state = Some(SystemState::new(world));
    }

    // This elaborate setup using `resource_scope` is a workaround for the fact
    // that `EntityMut` queries lock out resources. This is a known bug in Bevy.
    world.resource_scope(|world, clips: Mut<Assets<AnimationClip>>| {
        let (players, mut targets) = system_state.as_mut().unwrap().get_mut(world);
        targets.par_iter_mut().for_each(|mut target_entity| {
            let target = target_entity.get::<AnimationTarget>().unwrap();
            let Ok(player) = players.get(target.root) else {
                error!(
                    "Couldn't find the animation player for {:?}",
                    target_entity.id()
                );
                return;
            };

            player.animation.apply(&clips, 1.0, &mut target_entity);

            for transition in &player.transitions {
                transition
                    .animation
                    .apply(&clips, transition.current_weight, &mut target_entity);
            }
        });
    });
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
            .add_systems(Update, update_animation_targets)
            .add_systems(
                PostUpdate,
                (animation_player, animate_targets)
                    .chain()
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

impl AnimationTarget {
    // For private use in `update_animation_targets` only.
    fn new() -> AnimationTarget {
        AnimationTarget {
            root: Entity::PLACEHOLDER,
            path: EntityPath::new(),
        }
    }
}

fn update_animation(
    animation: &mut PlayingAnimation,
    paused: bool,
    time: &Time,
    animation_clips: &Assets<AnimationClip>,
) {
    let Some(animation_clip) = animation_clips.get(&animation.animation_clip) else {
        return;
    };

    let delta = if paused { 0.0 } else { time.delta_seconds() };

    animation.update(delta, animation_clip.duration);
}

impl From<Name> for EntityPath {
    fn from(value: Name) -> Self {
        EntityPath { parts: vec![value] }
    }
}

impl EntityPath {
    fn new() -> EntityPath {
        EntityPath { parts: vec![] }
    }

    fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

impl PlayingAnimation {
    fn apply(&self, clips: &Assets<AnimationClip>, weight: f32, target_entity: &mut EntityMut) {
        let target = target_entity.get::<AnimationTarget>().unwrap();

        let Some(clip) = clips.get(&self.animation_clip) else {
            error!(
                "Couldn't find the animation clip for {:?}",
                target_entity.id()
            );
            return;
        };

        // Extract the bone ID. If this bone isn't being animated, bail.
        let Some(bone_id) = clip.paths.get(&target.path) else {
            return;
        };

        let Some(curves) = clip.get_curves(*bone_id) else {
            error!("Couldn't find the curves for bone {:?}", target.path);
            return;
        };

        self.apply_curves(curves, weight, target_entity)
    }

    fn apply_curves(&self, curves: &[VariableCurve], weight: f32, target: &mut EntityMut) {
        for curve in curves {
            // Some curves have only one keyframe used to set a transform
            if curve.keyframe_timestamps.len() == 1 {
                self.apply_single_keyframe(curve, weight, target);
                return;
            }

            // Find the current keyframe
            let Some(step_start) = curve.find_current_keyframe(self.seek_time) else {
                return;
            };

            let timestamp_start = curve.keyframe_timestamps[step_start];
            let timestamp_end = curve.keyframe_timestamps[step_start + 1];
            // Compute how far we are through the keyframe, normalized to [0, 1]
            let lerp = f32::inverse_lerp(timestamp_start, timestamp_end, self.seek_time);

            self.apply_tweened_keyframe(
                curve,
                step_start,
                weight,
                lerp,
                timestamp_end - timestamp_start,
                target,
            );
        }
    }

    fn apply_single_keyframe(&self, curve: &VariableCurve, weight: f32, target: &mut EntityMut) {
        match &curve.keyframes {
            Keyframes::Rotation(keyframes) => {
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.rotation = transform.rotation.slerp(keyframes[0], weight);
                }
            }

            Keyframes::Translation(keyframes) => {
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.translation = transform.translation.lerp(keyframes[0], weight);
                }
            }

            Keyframes::Scale(keyframes) => {
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.scale = transform.scale.lerp(keyframes[0], weight);
                }
            }

            Keyframes::Weights(keyframes) => {
                let Some(mut morphs) = target.get_mut::<MorphWeights>() else {
                    error!(
                        "Tried to animate morphs on {:?}, but no `MorphWeights` was found",
                        target.id()
                    );
                    return;
                };

                let target_count = morphs.weights().len();
                lerp_morph_weights(
                    morphs.weights_mut(),
                    get_keyframe(target_count, keyframes, 0).iter().copied(),
                    weight,
                );
            }
        }
    }

    fn apply_tweened_keyframe(
        &self,
        curve: &VariableCurve,
        step_start: usize,
        weight: f32,
        lerp: f32,
        duration: f32,
        target: &mut EntityMut,
    ) {
        match (&curve.interpolation, &curve.keyframes) {
            (Interpolation::Step, Keyframes::Rotation(keyframes)) => {
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.rotation = transform.rotation.slerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Rotation(keyframes)) => {
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

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
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

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
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.translation =
                        transform.translation.lerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Translation(keyframes)) => {
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

                let translation_start = keyframes[step_start];
                let translation_end = keyframes[step_start + 1];
                let result = translation_start.lerp(translation_end, lerp);
                transform.translation = transform.translation.lerp(result, weight);
            }

            (Interpolation::CubicSpline, Keyframes::Translation(keyframes)) => {
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

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
                if let Some(mut transform) = target.get_mut::<Transform>() {
                    transform.scale = transform.scale.lerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Scale(keyframes)) => {
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

                let scale_start = keyframes[step_start];
                let scale_end = keyframes[step_start + 1];
                let result = scale_start.lerp(scale_end, lerp);
                transform.scale = transform.scale.lerp(result, weight);
            }

            (Interpolation::CubicSpline, Keyframes::Scale(keyframes)) => {
                let Some(mut transform) = target.get_mut::<Transform>() else {
                    return;
                };

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
                let Some(mut morphs) = target.get_mut::<MorphWeights>() else {
                    return;
                };

                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start);
                lerp_morph_weights(morphs.weights_mut(), morph_start.iter().copied(), weight);
            }

            (Interpolation::Linear, Keyframes::Weights(keyframes)) => {
                let Some(mut morphs) = target.get_mut::<MorphWeights>() else {
                    return;
                };

                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start);
                let morph_end = get_keyframe(target_count, keyframes, step_start + 1);
                let result = morph_start
                    .iter()
                    .zip(morph_end)
                    .map(|(a, b)| a.lerp(*b, lerp));
                lerp_morph_weights(morphs.weights_mut(), result, weight);
            }

            (Interpolation::CubicSpline, Keyframes::Weights(keyframes)) => {
                let Some(mut morphs) = target.get_mut::<MorphWeights>() else {
                    return;
                };

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
