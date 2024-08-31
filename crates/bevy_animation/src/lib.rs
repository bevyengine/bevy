#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Animation for the game engine Bevy

pub mod animatable;
pub mod graph;
pub mod transition;
mod util;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::iter;
use std::ops::{Add, Mul};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::entity::MapEntities;
use bevy_ecs::prelude::*;
use bevy_ecs::reflect::ReflectMapEntities;
use bevy_math::{FloatExt, Quat, Vec3};
use bevy_reflect::Reflect;
use bevy_render::mesh::morph::MorphWeights;
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_utils::hashbrown::HashMap;
use bevy_utils::{
    tracing::{error, trace},
    NoOpHash,
};
use fixedbitset::FixedBitSet;
use graph::{AnimationGraph, AnimationNodeIndex};
use petgraph::graph::NodeIndex;
use petgraph::Direction;
use prelude::{AnimationGraphAssetLoader, AnimationTransitions};
use thread_local::ThreadLocal;
use uuid::Uuid;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        animatable::*, graph::*, transition::*, AnimationClip, AnimationPlayer, AnimationPlugin,
        Interpolation, Keyframes, VariableCurve,
    };
}

use crate::transition::{advance_transitions, expire_completed_transitions};

/// The [UUID namespace] of animation targets (e.g. bones).
///
/// [UUID namespace]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Versions_3_and_5_(namespace_name-based)
pub static ANIMATION_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x3179f519d9274ff2b5966fd077023911);

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
        let last_keyframe = self.keyframe_timestamps.len() - 1;

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

    /// Find the index of the keyframe at or before the current time.
    ///
    /// Returns the first keyframe if the `seek_time` is before the first keyframe, and
    /// the second-to-last keyframe if the `seek_time` is after the last keyframe.
    /// Panics if there are less than 2 keyframes.
    pub fn find_interpolation_start_keyframe(&self, seek_time: f32) -> usize {
        // An Ok(keyframe_index) result means an exact result was found by binary search
        // An Err result means the keyframe was not found, and the index is the keyframe
        // PERF: finding the current keyframe can be optimised
        let search_result = self
            .keyframe_timestamps
            .binary_search_by(|probe| probe.partial_cmp(&seek_time).unwrap());

        // We want to find the index of the keyframe before the current time
        // If the keyframe is past the second-to-last keyframe, the animation cannot be interpolated.
        match search_result {
            // An exact match was found
            Ok(i) => i.clamp(0, self.keyframe_timestamps.len() - 2),
            // No exact match was found, so return the previous keyframe to interpolate from.
            Err(i) => (i.saturating_sub(1)).clamp(0, self.keyframe_timestamps.len() - 2),
        }
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

/// A list of [`VariableCurve`]s and the [`AnimationTargetId`]s to which they
/// apply.
///
/// Because animation clips refer to targets by UUID, they can target any
/// [`AnimationTarget`] with that ID.
#[derive(Asset, Reflect, Clone, Debug, Default)]
pub struct AnimationClip {
    curves: AnimationCurves,
    duration: f32,
}

/// A mapping from [`AnimationTargetId`] (e.g. bone in a skinned mesh) to the
/// animation curves.
pub type AnimationCurves = HashMap<AnimationTargetId, Vec<VariableCurve>, NoOpHash>;

/// A unique [UUID] for an animation target (e.g. bone in a skinned mesh).
///
/// The [`AnimationClip`] asset and the [`AnimationTarget`] component both use
/// this to refer to targets (e.g. bones in a skinned mesh) to be animated.
///
/// When importing an armature or an animation clip, asset loaders typically use
/// the full path name from the armature to the bone to generate these UUIDs.
/// The ID is unique to the full path name and based only on the names. So, for
/// example, any imported armature with a bone at the root named `Hips` will
/// assign the same [`AnimationTargetId`] to its root bone. Likewise, any
/// imported animation clip that animates a root bone named `Hips` will
/// reference the same [`AnimationTargetId`]. Any animation is playable on any
/// armature as long as the bone names match, which allows for easy animation
/// retargeting.
///
/// Note that asset loaders generally use the *full* path name to generate the
/// [`AnimationTargetId`]. Thus a bone named `Chest` directly connected to a
/// bone named `Hips` will have a different ID from a bone named `Chest` that's
/// connected to a bone named `Stomach`.
///
/// [UUID]: https://en.wikipedia.org/wiki/Universally_unique_identifier
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect, Debug)]
pub struct AnimationTargetId(pub Uuid);

impl Hash for AnimationTargetId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (hi, lo) = self.0.as_u64_pair();
        state.write_u64(hi ^ lo);
    }
}

/// An entity that can be animated by an [`AnimationPlayer`].
///
/// These are frequently referred to as *bones* or *joints*, because they often
/// refer to individually-animatable parts of an armature.
///
/// Asset loaders for armatures are responsible for adding these as necessary.
/// Typically, they're generated from hashed versions of the entire name path
/// from the root of the armature to the bone. See the [`AnimationTargetId`]
/// documentation for more details.
///
/// By convention, asset loaders add [`AnimationTarget`] components to the
/// descendants of an [`AnimationPlayer`], as well as to the [`AnimationPlayer`]
/// entity itself, but Bevy doesn't require this in any way. So, for example,
/// it's entirely possible for an [`AnimationPlayer`] to animate a target that
/// it isn't an ancestor of. If you add a new bone to or delete a bone from an
/// armature at runtime, you may want to update the [`AnimationTarget`]
/// component as appropriate, as Bevy won't do this automatically.
///
/// Note that each entity can only be animated by one animation player at a
/// time. However, you can change [`AnimationTarget`]'s `player` property at
/// runtime to change which player is responsible for animating the entity.
#[derive(Clone, Copy, Component, Reflect)]
#[reflect(Component, MapEntities)]
pub struct AnimationTarget {
    /// The ID of this animation target.
    ///
    /// Typically, this is derived from the path.
    pub id: AnimationTargetId,

    /// The entity containing the [`AnimationPlayer`].
    pub player: Entity,
}

impl AnimationClip {
    #[inline]
    /// [`VariableCurve`]s for each animation target. Indexed by the [`AnimationTargetId`].
    pub fn curves(&self) -> &AnimationCurves {
        &self.curves
    }

    #[inline]
    /// Get mutable references of [`VariableCurve`]s for each animation target. Indexed by the [`AnimationTargetId`].
    pub fn curves_mut(&mut self) -> &mut AnimationCurves {
        &mut self.curves
    }

    /// Gets the curves for a single animation target.
    ///
    /// Returns `None` if this clip doesn't animate the target.
    #[inline]
    pub fn curves_for_target(
        &self,
        target_id: AnimationTargetId,
    ) -> Option<&'_ Vec<VariableCurve>> {
        self.curves.get(&target_id)
    }

    /// Gets mutable references of the curves for a single animation target.
    ///
    /// Returns `None` if this clip doesn't animate the target.
    #[inline]
    pub fn curves_for_target_mut(
        &mut self,
        target_id: AnimationTargetId,
    ) -> Option<&'_ mut Vec<VariableCurve>> {
        self.curves.get_mut(&target_id)
    }

    /// Duration of the clip, represented in seconds.
    #[inline]
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Set the duration of the clip in seconds.
    #[inline]
    pub fn set_duration(&mut self, duration_sec: f32) {
        self.duration = duration_sec;
    }

    /// Adds a [`VariableCurve`] to an [`AnimationTarget`] named by an
    /// [`AnimationTargetId`].
    ///
    /// If the curve extends beyond the current duration of this clip, this
    /// method lengthens this clip to include the entire time span that the
    /// curve covers.
    pub fn add_curve_to_target(&mut self, target_id: AnimationTargetId, curve: VariableCurve) {
        // Update the duration of the animation by this curve duration if it's longer
        self.duration = self
            .duration
            .max(*curve.keyframe_timestamps.last().unwrap_or(&0.0));
        self.curves.entry(target_id).or_default().push(curve);
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

/// An animation that an [`AnimationPlayer`] is currently either playing or was
/// playing, but is presently paused.
///
/// An stopped animation is considered no longer active.
#[derive(Debug, Clone, Copy, Reflect)]
pub struct ActiveAnimation {
    /// The factor by which the weight from the [`AnimationGraph`] is multiplied.
    weight: f32,
    /// The actual weight of this animation this frame, taking the
    /// [`AnimationGraph`] into account.
    computed_weight: f32,
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
    /// Number of times the animation has completed.
    /// If the animation is playing in reverse, this increments when the animation passes the start.
    completions: u32,
    paused: bool,
}

impl Default for ActiveAnimation {
    fn default() -> Self {
        Self {
            weight: 1.0,
            computed_weight: 1.0,
            repeat: RepeatAnimation::default(),
            speed: 1.0,
            elapsed: 0.0,
            seek_time: 0.0,
            completions: 0,
            paused: false,
        }
    }
}

impl ActiveAnimation {
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
    pub fn replay(&mut self) {
        self.completions = 0;
        self.elapsed = 0.0;
        self.seek_time = 0.0;
    }

    /// Returns the current weight of this animation.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets the weight of this animation.
    pub fn set_weight(&mut self, weight: f32) {
        self.weight = weight;
    }

    /// Pause the animation.
    pub fn pause(&mut self) -> &mut Self {
        self.paused = true;
        self
    }

    /// Unpause the animation.
    pub fn resume(&mut self) -> &mut Self {
        self.paused = false;
        self
    }

    /// Returns true if this animation is currently paused.
    ///
    /// Note that paused animations are still [`ActiveAnimation`]s.
    #[inline]
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Sets the repeat mode for this playing animation.
    pub fn set_repeat(&mut self, repeat: RepeatAnimation) -> &mut Self {
        self.repeat = repeat;
        self
    }

    /// Marks this animation as repeating forever.
    pub fn repeat(&mut self) -> &mut Self {
        self.set_repeat(RepeatAnimation::Forever)
    }

    /// Returns the repeat mode assigned to this active animation.
    pub fn repeat_mode(&self) -> RepeatAnimation {
        self.repeat
    }

    /// Returns the number of times this animation has completed.
    pub fn completions(&self) -> u32 {
        self.completions
    }

    /// Returns true if the animation is playing in reverse.
    pub fn is_playback_reversed(&self) -> bool {
        self.speed < 0.0
    }

    /// Returns the speed of the animation playback.
    pub fn speed(&self) -> f32 {
        self.speed
    }

    /// Sets the speed of the animation playback.
    pub fn set_speed(&mut self, speed: f32) -> &mut Self {
        self.speed = speed;
        self
    }

    /// Returns the amount of time the animation has been playing.
    pub fn elapsed(&self) -> f32 {
        self.elapsed
    }

    /// Returns the seek time of the animation.
    ///
    /// This is nonnegative and no more than the clip duration.
    pub fn seek_time(&self) -> f32 {
        self.seek_time
    }

    /// Seeks to a specific time in the animation.
    pub fn seek_to(&mut self, seek_time: f32) -> &mut Self {
        self.seek_time = seek_time;
        self
    }

    /// Seeks to the beginning of the animation.
    pub fn rewind(&mut self) -> &mut Self {
        self.seek_time = 0.0;
        self
    }
}

/// Animation controls
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct AnimationPlayer {
    /// We use a `BTreeMap` instead of a `HashMap` here to ensure a consistent
    /// ordering when applying the animations.
    active_animations: BTreeMap<AnimationNodeIndex, ActiveAnimation>,
    blend_weights: HashMap<AnimationNodeIndex, f32>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for AnimationPlayer {
    fn clone(&self) -> Self {
        Self {
            active_animations: self.active_animations.clone(),
            blend_weights: self.blend_weights.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.active_animations.clone_from(&source.active_animations);
        self.blend_weights.clone_from(&source.blend_weights);
    }
}

/// The components that we might need to read or write during animation of each
/// animation target.
struct AnimationTargetContext<'a> {
    entity: Entity,
    target: &'a AnimationTarget,
    name: Option<&'a Name>,
    transform: Option<Mut<'a, Transform>>,
    morph_weights: Option<Mut<'a, MorphWeights>>,
}

/// Information needed during the traversal of the animation graph in
/// [`advance_animations`].
#[derive(Default)]
pub struct AnimationGraphEvaluator {
    /// The stack used for the depth-first search of the graph.
    dfs_stack: Vec<NodeIndex>,
    /// The list of visited nodes during the depth-first traversal.
    dfs_visited: FixedBitSet,
    /// Accumulated weights for each node.
    weights: Vec<f32>,
}

thread_local! {
    /// A cached per-thread copy of the graph evaluator.
    ///
    /// Caching the evaluator lets us save allocation traffic from frame to
    /// frame.
    static ANIMATION_GRAPH_EVALUATOR: RefCell<AnimationGraphEvaluator> =
        RefCell::new(AnimationGraphEvaluator::default());
}

impl AnimationPlayer {
    /// Start playing an animation, restarting it if necessary.
    pub fn start(&mut self, animation: AnimationNodeIndex) -> &mut ActiveAnimation {
        let playing_animation = self.active_animations.entry(animation).or_default();
        playing_animation.replay();
        playing_animation
    }

    /// Start playing an animation, unless the requested animation is already playing.
    pub fn play(&mut self, animation: AnimationNodeIndex) -> &mut ActiveAnimation {
        self.active_animations.entry(animation).or_default()
    }

    /// Stops playing the given animation, removing it from the list of playing
    /// animations.
    pub fn stop(&mut self, animation: AnimationNodeIndex) -> &mut Self {
        self.active_animations.remove(&animation);
        self
    }

    /// Stops all currently-playing animations.
    pub fn stop_all(&mut self) -> &mut Self {
        self.active_animations.clear();
        self
    }

    /// Iterates through all animations that this [`AnimationPlayer`] is
    /// currently playing.
    pub fn playing_animations(
        &self,
    ) -> impl Iterator<Item = (&AnimationNodeIndex, &ActiveAnimation)> {
        self.active_animations.iter()
    }

    /// Iterates through all animations that this [`AnimationPlayer`] is
    /// currently playing, mutably.
    pub fn playing_animations_mut(
        &mut self,
    ) -> impl Iterator<Item = (&AnimationNodeIndex, &mut ActiveAnimation)> {
        self.active_animations.iter_mut()
    }

    /// Check if the given animation node is being played.
    pub fn is_playing_animation(&self, animation: AnimationNodeIndex) -> bool {
        self.active_animations.contains_key(&animation)
    }

    /// Check if all playing animations have finished, according to the repetition behavior.
    pub fn all_finished(&self) -> bool {
        self.active_animations
            .values()
            .all(|playing_animation| playing_animation.is_finished())
    }

    /// Check if all playing animations are paused.
    #[doc(alias = "is_paused")]
    pub fn all_paused(&self) -> bool {
        self.active_animations
            .values()
            .all(|playing_animation| playing_animation.is_paused())
    }

    /// Resume all playing animations.
    #[doc(alias = "pause")]
    pub fn pause_all(&mut self) -> &mut Self {
        for (_, playing_animation) in self.playing_animations_mut() {
            playing_animation.pause();
        }
        self
    }

    /// Resume all active animations.
    #[doc(alias = "resume")]
    pub fn resume_all(&mut self) -> &mut Self {
        for (_, playing_animation) in self.playing_animations_mut() {
            playing_animation.resume();
        }
        self
    }

    /// Rewinds all active animations.
    #[doc(alias = "rewind")]
    pub fn rewind_all(&mut self) -> &mut Self {
        for (_, playing_animation) in self.playing_animations_mut() {
            playing_animation.rewind();
        }
        self
    }

    /// Multiplies the speed of all active animations by the given factor.
    #[doc(alias = "set_speed")]
    pub fn adjust_speeds(&mut self, factor: f32) -> &mut Self {
        for (_, playing_animation) in self.playing_animations_mut() {
            let new_speed = playing_animation.speed() * factor;
            playing_animation.set_speed(new_speed);
        }
        self
    }

    /// Seeks all active animations forward or backward by the same amount.
    ///
    /// To seek forward, pass a positive value; to seek negative, pass a
    /// negative value. Values below 0.0 or beyond the end of the animation clip
    /// are clamped appropriately.
    #[doc(alias = "seek_to")]
    pub fn seek_all_by(&mut self, amount: f32) -> &mut Self {
        for (_, playing_animation) in self.playing_animations_mut() {
            let new_time = playing_animation.seek_time();
            playing_animation.seek_to(new_time + amount);
        }
        self
    }

    /// Returns the [`ActiveAnimation`] associated with the given animation
    /// node if it's currently playing.
    ///
    /// If the animation isn't currently active, returns `None`.
    pub fn animation(&self, animation: AnimationNodeIndex) -> Option<&ActiveAnimation> {
        self.active_animations.get(&animation)
    }

    /// Returns a mutable reference to the [`ActiveAnimation`] associated with
    /// the given animation node if it's currently active.
    ///
    /// If the animation isn't currently active, returns `None`.
    pub fn animation_mut(&mut self, animation: AnimationNodeIndex) -> Option<&mut ActiveAnimation> {
        self.active_animations.get_mut(&animation)
    }

    /// Returns true if the animation is currently playing or paused, or false
    /// if the animation is stopped.
    pub fn animation_is_playing(&self, animation: AnimationNodeIndex) -> bool {
        self.active_animations.contains_key(&animation)
    }
}

/// A system that advances the time for all playing animations.
pub fn advance_animations(
    time: Res<Time>,
    animation_clips: Res<Assets<AnimationClip>>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut players: Query<(&mut AnimationPlayer, &Handle<AnimationGraph>)>,
    animation_graph_evaluator: Local<ThreadLocal<RefCell<AnimationGraphEvaluator>>>,
) {
    let delta_seconds = time.delta_seconds();
    players
        .par_iter_mut()
        .for_each(|(mut player, graph_handle)| {
            let Some(animation_graph) = animation_graphs.get(graph_handle) else {
                return;
            };

            // Tick animations, and schedule them.
            //
            // We use a thread-local here so we can reuse allocations across
            // frames.
            let mut evaluator = animation_graph_evaluator.get_or_default().borrow_mut();

            let AnimationPlayer {
                ref mut active_animations,
                ref blend_weights,
                ..
            } = *player;

            // Reset our state.
            evaluator.reset(animation_graph.root, animation_graph.graph.node_count());

            while let Some(node_index) = evaluator.dfs_stack.pop() {
                // Skip if we've already visited this node.
                if evaluator.dfs_visited.put(node_index.index()) {
                    continue;
                }

                let node = &animation_graph[node_index];

                // Calculate weight from the graph.
                let mut weight = node.weight;
                for parent_index in animation_graph
                    .graph
                    .neighbors_directed(node_index, Direction::Incoming)
                {
                    weight *= animation_graph[parent_index].weight;
                }
                evaluator.weights[node_index.index()] = weight;

                if let Some(active_animation) = active_animations.get_mut(&node_index) {
                    // Tick the animation if necessary.
                    if !active_animation.paused {
                        if let Some(ref clip_handle) = node.clip {
                            if let Some(clip) = animation_clips.get(clip_handle) {
                                active_animation.update(delta_seconds, clip.duration);
                            }
                        }
                    }

                    weight *= active_animation.weight;
                } else if let Some(&blend_weight) = blend_weights.get(&node_index) {
                    weight *= blend_weight;
                }

                // Write in the computed weight.
                if let Some(active_animation) = active_animations.get_mut(&node_index) {
                    active_animation.computed_weight = weight;
                }

                // Push children.
                evaluator.dfs_stack.extend(
                    animation_graph
                        .graph
                        .neighbors_directed(node_index, Direction::Outgoing),
                );
            }
        });
}

/// A system that modifies animation targets (e.g. bones in a skinned mesh)
/// according to the currently-playing animation.
pub fn animate_targets(
    clips: Res<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    players: Query<(&AnimationPlayer, &Handle<AnimationGraph>)>,
    mut targets: Query<(
        Entity,
        &AnimationTarget,
        Option<&Name>,
        AnyOf<(&mut Transform, &mut MorphWeights)>,
    )>,
) {
    // We use two queries here: one read-only query for animation players and
    // one read-write query for animation targets (e.g. bones). The
    // `AnimationPlayer` query is read-only shared memory accessible from all
    // animation targets, which are evaluated in parallel.

    // Iterate over all animation targets in parallel.
    targets
        .par_iter_mut()
        .for_each(|(id, target, name, (transform, morph_weights))| {
            let Ok((animation_player, animation_graph_handle)) = players.get(target.player) else {
                trace!(
                    "Either an animation player {:?} or a graph was missing for the target \
                     entity {:?} ({:?}); no animations will play this frame",
                    target.player,
                    id,
                    name,
                );
                return;
            };

            // The graph might not have loaded yet. Safely bail.
            let Some(animation_graph) = graphs.get(animation_graph_handle) else {
                return;
            };

            let mut target_context = AnimationTargetContext {
                entity: id,
                target,
                name,
                transform,
                morph_weights,
            };

            // Apply the animations one after another. The way we accumulate
            // weights ensures that the order we apply them in doesn't matter.
            //
            // Proof: Consider three animations A₀, A₁, A₂, … with weights w₀,
            // w₁, w₂, … respectively. We seek the value:
            //
            //     A₀w₀ + A₁w₁ + A₂w₂ + ⋯
            //
            // Defining lerp(a, b, t) = a + t(b - a), we have:
            //
            //                                    ⎛    ⎛          w₁   ⎞           w₂     ⎞
            //     A₀w₀ + A₁w₁ + A₂w₂ + ⋯ = ⋯ lerp⎜lerp⎜A₀, A₁, ⎯⎯⎯⎯⎯⎯⎯⎯⎟, A₂, ⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎯⎟ ⋯
            //                                    ⎝    ⎝        w₀ + w₁⎠      w₀ + w₁ + w₂⎠
            //
            // Each step of the following loop corresponds to one of the lerp
            // operations above.
            let mut total_weight = 0.0;
            for (&animation_graph_node_index, active_animation) in
                animation_player.active_animations.iter()
            {
                if active_animation.weight == 0.0 {
                    continue;
                }

                let Some(clip) = animation_graph
                    .get(animation_graph_node_index)
                    .and_then(|animation_graph_node| animation_graph_node.clip.as_ref())
                    .and_then(|animation_clip_handle| clips.get(animation_clip_handle))
                else {
                    continue;
                };

                let Some(curves) = clip.curves_for_target(target_context.target.id) else {
                    continue;
                };

                let weight = active_animation.computed_weight;
                total_weight += weight;

                target_context.apply(curves, weight / total_weight, active_animation.seek_time);
            }
        });
}

impl AnimationTargetContext<'_> {
    /// Applies a clip to a single animation target according to the
    /// [`AnimationTargetContext`].
    fn apply(&mut self, curves: &[VariableCurve], weight: f32, seek_time: f32) {
        for curve in curves {
            // Some curves have only one keyframe used to set a transform
            if curve.keyframe_timestamps.len() == 1 {
                self.apply_single_keyframe(curve, weight);
                continue;
            }

            // Find the best keyframe to interpolate from
            let step_start = curve.find_interpolation_start_keyframe(seek_time);

            let timestamp_start = curve.keyframe_timestamps[step_start];
            let timestamp_end = curve.keyframe_timestamps[step_start + 1];
            // Compute how far we are through the keyframe, normalized to [0, 1]
            let lerp = f32::inverse_lerp(timestamp_start, timestamp_end, seek_time).clamp(0.0, 1.0);

            self.apply_tweened_keyframe(
                curve,
                step_start,
                lerp,
                weight,
                timestamp_end - timestamp_start,
            );
        }
    }

    fn apply_single_keyframe(&mut self, curve: &VariableCurve, weight: f32) {
        match &curve.keyframes {
            Keyframes::Rotation(keyframes) => {
                if let Some(ref mut transform) = self.transform {
                    transform.rotation = transform.rotation.slerp(keyframes[0], weight);
                }
            }

            Keyframes::Translation(keyframes) => {
                if let Some(ref mut transform) = self.transform {
                    transform.translation = transform.translation.lerp(keyframes[0], weight);
                }
            }

            Keyframes::Scale(keyframes) => {
                if let Some(ref mut transform) = self.transform {
                    transform.scale = transform.scale.lerp(keyframes[0], weight);
                }
            }

            Keyframes::Weights(keyframes) => {
                let Some(ref mut morphs) = self.morph_weights else {
                    error!(
                        "Tried to animate morphs on {:?} ({:?}), but no `MorphWeights` was found",
                        self.entity, self.name,
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
        &mut self,
        curve: &VariableCurve,
        step_start: usize,
        lerp: f32,
        weight: f32,
        duration: f32,
    ) {
        match (&curve.interpolation, &curve.keyframes) {
            (Interpolation::Step, Keyframes::Rotation(keyframes)) => {
                if let Some(ref mut transform) = self.transform {
                    transform.rotation = transform.rotation.slerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Rotation(keyframes)) => {
                let Some(ref mut transform) = self.transform else {
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
                let Some(ref mut transform) = self.transform else {
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
                if let Some(ref mut transform) = self.transform {
                    transform.translation =
                        transform.translation.lerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Translation(keyframes)) => {
                let Some(ref mut transform) = self.transform else {
                    return;
                };

                let translation_start = keyframes[step_start];
                let translation_end = keyframes[step_start + 1];
                let result = translation_start.lerp(translation_end, lerp);
                transform.translation = transform.translation.lerp(result, weight);
            }

            (Interpolation::CubicSpline, Keyframes::Translation(keyframes)) => {
                let Some(ref mut transform) = self.transform else {
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
                if let Some(ref mut transform) = self.transform {
                    transform.scale = transform.scale.lerp(keyframes[step_start], weight);
                }
            }

            (Interpolation::Linear, Keyframes::Scale(keyframes)) => {
                let Some(ref mut transform) = self.transform else {
                    return;
                };

                let scale_start = keyframes[step_start];
                let scale_end = keyframes[step_start + 1];
                let result = scale_start.lerp(scale_end, lerp);
                transform.scale = transform.scale.lerp(result, weight);
            }

            (Interpolation::CubicSpline, Keyframes::Scale(keyframes)) => {
                let Some(ref mut transform) = self.transform else {
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
                let Some(ref mut morphs) = self.morph_weights else {
                    return;
                };

                let target_count = morphs.weights().len();
                let morph_start = get_keyframe(target_count, keyframes, step_start);
                lerp_morph_weights(morphs.weights_mut(), morph_start.iter().copied(), weight);
            }

            (Interpolation::Linear, Keyframes::Weights(keyframes)) => {
                let Some(ref mut morphs) = self.morph_weights else {
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
                let Some(ref mut morphs) = self.morph_weights else {
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

/// Adds animation support to an app
#[derive(Default)]
pub struct AnimationPlugin;

impl Plugin for AnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<AnimationClip>()
            .init_asset::<AnimationGraph>()
            .init_asset_loader::<AnimationGraphAssetLoader>()
            .register_asset_reflect::<AnimationClip>()
            .register_asset_reflect::<AnimationGraph>()
            .register_type::<AnimationPlayer>()
            .register_type::<AnimationTarget>()
            .register_type::<AnimationTransitions>()
            .register_type::<NodeIndex>()
            .add_systems(
                PostUpdate,
                (
                    advance_transitions,
                    advance_animations,
                    animate_targets,
                    expire_completed_transitions,
                )
                    .chain()
                    .before(TransformSystem::TransformPropagate),
            );
    }
}

impl AnimationTargetId {
    /// Creates a new [`AnimationTargetId`] by hashing a list of names.
    ///
    /// Typically, this will be the path from the animation root to the
    /// animation target (e.g. bone) that is to be animated.
    pub fn from_names<'a>(names: impl Iterator<Item = &'a Name>) -> Self {
        let mut blake3 = blake3::Hasher::new();
        blake3.update(ANIMATION_TARGET_NAMESPACE.as_bytes());
        for name in names {
            blake3.update(name.as_bytes());
        }
        let hash = blake3.finalize().as_bytes()[0..16].try_into().unwrap();
        Self(*uuid::Builder::from_sha1_bytes(hash).as_uuid())
    }

    /// Creates a new [`AnimationTargetId`] by hashing a single name.
    pub fn from_name(name: &Name) -> Self {
        Self::from_names(iter::once(name))
    }
}

impl From<&Name> for AnimationTargetId {
    fn from(name: &Name) -> Self {
        AnimationTargetId::from_name(name)
    }
}

impl MapEntities for AnimationTarget {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.player = entity_mapper.map_entity(self.player);
    }
}

impl AnimationGraphEvaluator {
    // Starts a new depth-first search.
    fn reset(&mut self, root: AnimationNodeIndex, node_count: usize) {
        self.dfs_stack.clear();
        self.dfs_stack.push(root);

        self.dfs_visited.grow(node_count);
        self.dfs_visited.clear();

        self.weights.clear();
        self.weights.extend(iter::repeat(0.0).take(node_count));
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
