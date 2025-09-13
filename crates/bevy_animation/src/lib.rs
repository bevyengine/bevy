#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]

//! Animation for the game engine Bevy

extern crate alloc;

pub mod animatable;
pub mod animation_curves;
pub mod gltf_curves;
pub mod graph;
pub mod transition;

mod animation_event;
mod util;

pub use animation_event::*;

use core::{
    any::TypeId,
    cell::RefCell,
    fmt::Debug,
    hash::{Hash, Hasher},
    iter, slice,
};
use graph::AnimationNodeType;
use prelude::AnimationCurveEvaluator;

use crate::{
    graph::{AnimationGraphHandle, ThreadedAnimationGraphs},
    prelude::EvaluatorId,
};

use bevy_app::{AnimationSystems, App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, AssetEventSystems, Assets};
use bevy_ecs::{prelude::*, world::EntityMutExcept};
use bevy_math::FloatOrd;
use bevy_platform::{collections::HashMap, hash::NoOpHash};
use bevy_reflect::{prelude::ReflectDefault, Reflect, TypePath};
use bevy_time::Time;
use bevy_transform::TransformSystems;
use bevy_utils::{PreHashMap, PreHashMapExt, TypeIdMap};
use serde::{Deserialize, Serialize};
use thread_local::ThreadLocal;
use tracing::{trace, warn};
use uuid::Uuid;

/// The animation prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        animatable::*, animation_curves::*, graph::*, transition::*, AnimationClip,
        AnimationPlayer, AnimationPlugin, VariableCurve,
    };
}

use crate::{
    animation_curves::AnimationCurve,
    graph::{AnimationGraph, AnimationGraphAssetLoader, AnimationNodeIndex},
    transition::{advance_transitions, expire_completed_transitions},
};
use alloc::sync::Arc;

/// The [UUID namespace] of animation targets (e.g. bones).
///
/// [UUID namespace]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Versions_3_and_5_(namespace_name-based)
pub static ANIMATION_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x3179f519d9274ff2b5966fd077023911);

/// Contains an [animation curve] which is used to animate a property of an entity.
///
/// [animation curve]: AnimationCurve
#[derive(Debug, TypePath)]
pub struct VariableCurve(pub Box<dyn AnimationCurve>);

impl Clone for VariableCurve {
    fn clone(&self) -> Self {
        Self(AnimationCurve::clone_value(&*self.0))
    }
}

impl VariableCurve {
    /// Create a new [`VariableCurve`] from an [animation curve].
    ///
    /// [animation curve]: AnimationCurve
    pub fn new(animation_curve: impl AnimationCurve) -> Self {
        Self(Box::new(animation_curve))
    }
}

/// A list of [`VariableCurve`]s and the [`AnimationTargetId`]s to which they
/// apply.
///
/// Because animation clips refer to targets by UUID, they can target any
/// [`AnimationTarget`] with that ID.
#[derive(Asset, Reflect, Clone, Debug, Default)]
#[reflect(Clone, Default)]
pub struct AnimationClip {
    // This field is ignored by reflection because AnimationCurves can contain things that are not reflect-able
    #[reflect(ignore, clone)]
    curves: AnimationCurves,
    events: AnimationEvents,
    duration: f32,
}

#[derive(Reflect, Debug, Clone)]
#[reflect(Clone)]
struct TimedAnimationEvent {
    time: f32,
    event: AnimationEventData,
}

#[derive(Reflect, Debug, Clone)]
#[reflect(Clone)]
struct AnimationEventData {
    #[reflect(ignore, clone)]
    trigger: AnimationEventFn,
}

impl AnimationEventData {
    fn trigger(&self, commands: &mut Commands, entity: Entity, time: f32, weight: f32) {
        (self.trigger.0)(commands, entity, time, weight);
    }
}

#[derive(Reflect, Clone)]
#[reflect(opaque)]
#[reflect(Clone, Default, Debug)]
struct AnimationEventFn(Arc<dyn Fn(&mut Commands, Entity, f32, f32) + Send + Sync>);

impl Default for AnimationEventFn {
    fn default() -> Self {
        Self(Arc::new(|_commands, _entity, _time, _weight| {}))
    }
}

impl Debug for AnimationEventFn {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("AnimationEventFn").finish()
    }
}

#[derive(Reflect, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
#[reflect(Clone)]
enum AnimationEventTarget {
    Root,
    Node(AnimationTargetId),
}

type AnimationEvents = HashMap<AnimationEventTarget, Vec<TimedAnimationEvent>>;

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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect, Debug, Serialize, Deserialize)]
#[reflect(Clone)]
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
#[reflect(Component, Clone)]
pub struct AnimationTarget {
    /// The ID of this animation target.
    ///
    /// Typically, this is derived from the path.
    pub id: AnimationTargetId,

    /// The entity containing the [`AnimationPlayer`].
    #[entities]
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

    /// Adds an [`AnimationCurve`] to an [`AnimationTarget`] named by an
    /// [`AnimationTargetId`].
    ///
    /// If the curve extends beyond the current duration of this clip, this
    /// method lengthens this clip to include the entire time span that the
    /// curve covers.
    ///
    /// More specifically:
    /// - This clip will be sampled on the interval `[0, duration]`.
    /// - Each curve in the clip is sampled by first clamping the sample time to its [domain].
    /// - Curves that extend forever never contribute to the duration.
    ///
    /// For example, a curve with domain `[2, 5]` will extend the clip to cover `[0, 5]`
    /// when added and will produce the same output on the entire interval `[0, 2]` because
    /// these time values all get clamped to `2`.
    ///
    /// By contrast, a curve with domain `[-10, ∞]` will never extend the clip duration when
    /// added and will be sampled only on `[0, duration]`, ignoring all negative time values.
    ///
    /// [domain]: AnimationCurve::domain
    pub fn add_curve_to_target(
        &mut self,
        target_id: AnimationTargetId,
        curve: impl AnimationCurve,
    ) {
        // Update the duration of the animation by this curve duration if it's longer
        let end = curve.domain().end();
        if end.is_finite() {
            self.duration = self.duration.max(end);
        }
        self.curves
            .entry(target_id)
            .or_default()
            .push(VariableCurve::new(curve));
    }

    /// Like [`add_curve_to_target`], but adding a [`VariableCurve`] directly.
    ///
    /// Under normal circumstances, that method is generally more convenient.
    ///
    /// [`add_curve_to_target`]: AnimationClip::add_curve_to_target
    pub fn add_variable_curve_to_target(
        &mut self,
        target_id: AnimationTargetId,
        variable_curve: VariableCurve,
    ) {
        let end = variable_curve.0.domain().end();
        if end.is_finite() {
            self.duration = self.duration.max(end);
        }
        self.curves
            .entry(target_id)
            .or_default()
            .push(variable_curve);
    }

    /// Add an [`EntityEvent`] with no [`AnimationTarget`] to this [`AnimationClip`].
    ///
    /// The `event` will be cloned and triggered on the [`AnimationPlayer`] entity once the `time` (in seconds)
    /// is reached in the animation.
    ///
    /// See also [`add_event_to_target`](Self::add_event_to_target).
    pub fn add_event(&mut self, time: f32, event: impl AnimationEvent) {
        self.add_event_fn(
            time,
            move |commands: &mut Commands, entity: Entity, _time: f32, _weight: f32| {
                commands.trigger_with(
                    event.clone(),
                    AnimationEventTrigger {
                        animation_player: entity,
                    },
                );
            },
        );
    }

    /// Add an [`EntityEvent`] to an [`AnimationTarget`] named by an [`AnimationTargetId`].
    ///
    /// The `event` will be cloned and triggered on the entity matching the target once the `time` (in seconds)
    /// is reached in the animation.
    ///
    /// Use [`add_event`](Self::add_event) instead if you don't have a specific target.
    pub fn add_event_to_target(
        &mut self,
        target_id: AnimationTargetId,
        time: f32,
        event: impl AnimationEvent,
    ) {
        self.add_event_fn_to_target(
            target_id,
            time,
            move |commands: &mut Commands, entity: Entity, _time: f32, _weight: f32| {
                commands.trigger_with(
                    event.clone(),
                    AnimationEventTrigger {
                        animation_player: entity,
                    },
                );
            },
        );
    }

    /// Add an event function with no [`AnimationTarget`] to this [`AnimationClip`].
    ///
    /// The `func` will trigger on the [`AnimationPlayer`] entity once the `time` (in seconds)
    /// is reached in the animation.
    ///
    /// For a simpler [`EntityEvent`]-based alternative, see [`AnimationClip::add_event`].
    /// See also [`add_event_to_target`](Self::add_event_to_target).
    ///
    /// ```
    /// # use bevy_animation::AnimationClip;
    /// # let mut clip = AnimationClip::default();
    /// clip.add_event_fn(1.0, |commands, entity, time, weight| {
    ///   println!("Animation event triggered {entity:#?} at time {time} with weight {weight}");
    /// })
    /// ```
    pub fn add_event_fn(
        &mut self,
        time: f32,
        func: impl Fn(&mut Commands, Entity, f32, f32) + Send + Sync + 'static,
    ) {
        self.add_event_internal(AnimationEventTarget::Root, time, func);
    }

    /// Add an event function to an [`AnimationTarget`] named by an [`AnimationTargetId`].
    ///
    /// The `func` will trigger on the entity matching the target once the `time` (in seconds)
    /// is reached in the animation.
    ///
    /// For a simpler [`EntityEvent`]-based alternative, see [`AnimationClip::add_event_to_target`].
    /// Use [`add_event`](Self::add_event) instead if you don't have a specific target.
    ///
    /// ```
    /// # use bevy_animation::{AnimationClip, AnimationTargetId};
    /// # let mut clip = AnimationClip::default();
    /// clip.add_event_fn_to_target(AnimationTargetId::from_iter(["Arm", "Hand"]), 1.0, |commands, entity, time, weight| {
    ///   println!("Animation event triggered {entity:#?} at time {time} with weight {weight}");
    /// })
    /// ```
    pub fn add_event_fn_to_target(
        &mut self,
        target_id: AnimationTargetId,
        time: f32,
        func: impl Fn(&mut Commands, Entity, f32, f32) + Send + Sync + 'static,
    ) {
        self.add_event_internal(AnimationEventTarget::Node(target_id), time, func);
    }

    fn add_event_internal(
        &mut self,
        target: AnimationEventTarget,
        time: f32,
        trigger_fn: impl Fn(&mut Commands, Entity, f32, f32) + Send + Sync + 'static,
    ) {
        self.duration = self.duration.max(time);
        let triggers = self.events.entry(target).or_default();
        match triggers.binary_search_by_key(&FloatOrd(time), |e| FloatOrd(e.time)) {
            Ok(index) | Err(index) => triggers.insert(
                index,
                TimedAnimationEvent {
                    time,
                    event: AnimationEventData {
                        trigger: AnimationEventFn(Arc::new(trigger_fn)),
                    },
                },
            ),
        }
    }
}

/// Repetition behavior of an animation.
#[derive(Reflect, Debug, PartialEq, Eq, Copy, Clone, Default)]
#[reflect(Clone, Default)]
pub enum RepeatAnimation {
    /// The animation will finish after running once.
    #[default]
    Never,
    /// The animation will finish after running "n" times.
    Count(u32),
    /// The animation will never finish.
    Forever,
}

/// Why Bevy failed to evaluate an animation.
#[derive(Clone, Debug)]
pub enum AnimationEvaluationError {
    /// The component to be animated isn't present on the animation target.
    ///
    /// To fix this error, make sure the entity to be animated contains all
    /// components that have animation curves.
    ComponentNotPresent(TypeId),

    /// The component to be animated was present, but the property on the
    /// component wasn't present.
    PropertyNotPresent(TypeId),

    /// An internal error occurred in the implementation of
    /// [`AnimationCurveEvaluator`].
    ///
    /// You shouldn't ordinarily see this error unless you implemented
    /// [`AnimationCurveEvaluator`] yourself. The contained [`TypeId`] is the ID
    /// of the curve evaluator.
    InconsistentEvaluatorImplementation(TypeId),
}

/// An animation that an [`AnimationPlayer`] is currently either playing or was
/// playing, but is presently paused.
///
/// A stopped animation is considered no longer active.
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Clone, Default)]
pub struct ActiveAnimation {
    /// The factor by which the weight from the [`AnimationGraph`] is multiplied.
    weight: f32,
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
    /// The `seek_time` of the previous tick, if any.
    last_seek_time: Option<f32>,
    /// Number of times the animation has completed.
    /// If the animation is playing in reverse, this increments when the animation passes the start.
    completions: u32,
    /// `true` if the animation was completed at least once this tick.
    just_completed: bool,
    paused: bool,
}

impl Default for ActiveAnimation {
    fn default() -> Self {
        Self {
            weight: 1.0,
            repeat: RepeatAnimation::default(),
            speed: 1.0,
            elapsed: 0.0,
            seek_time: 0.0,
            last_seek_time: None,
            completions: 0,
            just_completed: false,
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
        self.just_completed = false;
        self.last_seek_time = Some(self.seek_time);

        if self.is_finished() {
            return;
        }

        self.elapsed += delta;
        self.seek_time += delta * self.speed;

        let over_time = self.speed > 0.0 && self.seek_time >= clip_duration;
        let under_time = self.speed < 0.0 && self.seek_time < 0.0;

        if over_time || under_time {
            self.just_completed = true;
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
        self.just_completed = false;
        self.completions = 0;
        self.elapsed = 0.0;
        self.last_seek_time = None;
        self.seek_time = 0.0;
    }

    /// Returns the current weight of this animation.
    pub fn weight(&self) -> f32 {
        self.weight
    }

    /// Sets the weight of this animation.
    pub fn set_weight(&mut self, weight: f32) -> &mut Self {
        self.weight = weight;
        self
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
    ///
    /// This will not trigger events between the current time and `seek_time`.
    /// Use [`seek_to`](Self::seek_to) if this is desired.
    pub fn set_seek_time(&mut self, seek_time: f32) -> &mut Self {
        self.last_seek_time = Some(seek_time);
        self.seek_time = seek_time;
        self
    }

    /// Seeks to a specific time in the animation.
    ///
    /// Note that any events between the current time and `seek_time`
    /// will be triggered on the next update.
    /// Use [`set_seek_time`](Self::set_seek_time) if this is undesired.
    pub fn seek_to(&mut self, seek_time: f32) -> &mut Self {
        self.last_seek_time = Some(self.seek_time);
        self.seek_time = seek_time;
        self
    }

    /// Seeks to the beginning of the animation.
    ///
    /// Note that any events between the current time and `0.0`
    /// will be triggered on the next update.
    /// Use [`set_seek_time`](Self::set_seek_time) if this is undesired.
    pub fn rewind(&mut self) -> &mut Self {
        self.last_seek_time = Some(self.seek_time);
        self.seek_time = 0.0;
        self
    }
}

/// Animation controls.
///
/// Automatically added to any root animations of a scene when it is
/// spawned.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default, Clone)]
pub struct AnimationPlayer {
    active_animations: HashMap<AnimationNodeIndex, ActiveAnimation>,
}

// This is needed since `#[derive(Clone)]` does not generate optimized `clone_from`.
impl Clone for AnimationPlayer {
    fn clone(&self) -> Self {
        Self {
            active_animations: self.active_animations.clone(),
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.active_animations.clone_from(&source.active_animations);
    }
}

/// Temporary data that the [`animate_targets`] system maintains.
#[derive(Default)]
pub struct AnimationEvaluationState {
    /// Stores all [`AnimationCurveEvaluator`]s corresponding to properties that
    /// we've seen so far.
    ///
    /// This is a mapping from the id of an animation curve evaluator to
    /// the animation curve evaluator itself.
    ///
    /// For efficiency's sake, the [`AnimationCurveEvaluator`]s are cached from
    /// frame to frame and animation target to animation target. Therefore,
    /// there may be entries in this list corresponding to properties that the
    /// current [`AnimationPlayer`] doesn't animate. To iterate only over the
    /// properties that are currently being animated, consult the
    /// [`Self::current_evaluators`] set.
    evaluators: AnimationCurveEvaluators,

    /// The set of [`AnimationCurveEvaluator`] types that the current
    /// [`AnimationPlayer`] is animating.
    ///
    /// This is built up as new curve evaluators are encountered during graph
    /// traversal.
    current_evaluators: CurrentEvaluators,
}

#[derive(Default)]
struct AnimationCurveEvaluators {
    component_property_curve_evaluators:
        PreHashMap<(TypeId, usize), Box<dyn AnimationCurveEvaluator>>,
    type_id_curve_evaluators: TypeIdMap<Box<dyn AnimationCurveEvaluator>>,
}

impl AnimationCurveEvaluators {
    #[inline]
    pub(crate) fn get_mut(&mut self, id: EvaluatorId) -> Option<&mut dyn AnimationCurveEvaluator> {
        match id {
            EvaluatorId::ComponentField(component_property) => self
                .component_property_curve_evaluators
                .get_mut(component_property),
            EvaluatorId::Type(type_id) => self.type_id_curve_evaluators.get_mut(&type_id),
        }
        .map(|e| &mut **e)
    }

    #[inline]
    pub(crate) fn get_or_insert_with(
        &mut self,
        id: EvaluatorId,
        func: impl FnOnce() -> Box<dyn AnimationCurveEvaluator>,
    ) -> &mut dyn AnimationCurveEvaluator {
        match id {
            EvaluatorId::ComponentField(component_property) => &mut **self
                .component_property_curve_evaluators
                .get_or_insert_with(component_property, func),
            EvaluatorId::Type(type_id) => match self.type_id_curve_evaluators.entry(type_id) {
                bevy_platform::collections::hash_map::Entry::Occupied(occupied_entry) => {
                    &mut **occupied_entry.into_mut()
                }
                bevy_platform::collections::hash_map::Entry::Vacant(vacant_entry) => {
                    &mut **vacant_entry.insert(func())
                }
            },
        }
    }
}

#[derive(Default)]
struct CurrentEvaluators {
    component_properties: PreHashMap<(TypeId, usize), ()>,
    type_ids: TypeIdMap<()>,
}

impl CurrentEvaluators {
    pub(crate) fn keys(&self) -> impl Iterator<Item = EvaluatorId<'_>> {
        self.component_properties
            .keys()
            .map(EvaluatorId::ComponentField)
            .chain(self.type_ids.keys().copied().map(EvaluatorId::Type))
    }

    pub(crate) fn clear(
        &mut self,
        mut visit: impl FnMut(EvaluatorId) -> Result<(), AnimationEvaluationError>,
    ) -> Result<(), AnimationEvaluationError> {
        for (key, _) in self.component_properties.drain() {
            (visit)(EvaluatorId::ComponentField(&key))?;
        }

        for (key, _) in self.type_ids.drain() {
            (visit)(EvaluatorId::Type(key))?;
        }

        Ok(())
    }

    #[inline]
    pub(crate) fn insert(&mut self, id: EvaluatorId) {
        match id {
            EvaluatorId::ComponentField(component_property) => {
                self.component_properties.insert(*component_property, ());
            }
            EvaluatorId::Type(type_id) => {
                self.type_ids.insert(type_id, ());
            }
        }
    }
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

    /// Returns true if the animation is currently playing or paused, or false
    /// if the animation is stopped.
    pub fn is_playing_animation(&self, animation: AnimationNodeIndex) -> bool {
        self.active_animations.contains_key(&animation)
    }

    /// Check if all playing animations have finished, according to the repetition behavior.
    pub fn all_finished(&self) -> bool {
        self.active_animations
            .values()
            .all(ActiveAnimation::is_finished)
    }

    /// Check if all playing animations are paused.
    #[doc(alias = "is_paused")]
    pub fn all_paused(&self) -> bool {
        self.active_animations
            .values()
            .all(ActiveAnimation::is_paused)
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
}

/// A system that triggers untargeted animation events for the currently-playing animations.
fn trigger_untargeted_animation_events(
    mut commands: Commands,
    clips: Res<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    players: Query<(Entity, &AnimationPlayer, &AnimationGraphHandle)>,
) {
    for (entity, player, graph_id) in &players {
        // The graph might not have loaded yet. Safely bail.
        let Some(graph) = graphs.get(graph_id) else {
            return;
        };

        for (index, active_animation) in player.active_animations.iter() {
            if active_animation.paused {
                continue;
            }

            let Some(clip) = graph
                .get(*index)
                .and_then(|node| match &node.node_type {
                    AnimationNodeType::Clip(handle) => Some(handle),
                    AnimationNodeType::Blend | AnimationNodeType::Add => None,
                })
                .and_then(|id| clips.get(id))
            else {
                continue;
            };

            let Some(triggered_events) =
                TriggeredEvents::from_animation(AnimationEventTarget::Root, clip, active_animation)
            else {
                continue;
            };

            for TimedAnimationEvent { time, event } in triggered_events.iter() {
                event.trigger(&mut commands, entity, *time, active_animation.weight);
            }
        }
    }
}

/// A system that advances the time for all playing animations.
pub fn advance_animations(
    time: Res<Time>,
    animation_clips: Res<Assets<AnimationClip>>,
    animation_graphs: Res<Assets<AnimationGraph>>,
    mut players: Query<(&mut AnimationPlayer, &AnimationGraphHandle)>,
) {
    let delta_seconds = time.delta_secs();
    players
        .par_iter_mut()
        .for_each(|(mut player, graph_handle)| {
            let Some(animation_graph) = animation_graphs.get(graph_handle) else {
                return;
            };

            // Tick animations, and schedule them.

            let AnimationPlayer {
                ref mut active_animations,
                ..
            } = *player;

            for node_index in animation_graph.graph.node_indices() {
                let node = &animation_graph[node_index];

                if let Some(active_animation) = active_animations.get_mut(&node_index) {
                    // Tick the animation if necessary.
                    if !active_animation.paused
                        && let AnimationNodeType::Clip(ref clip_handle) = node.node_type
                        && let Some(clip) = animation_clips.get(clip_handle)
                    {
                        active_animation.update(delta_seconds, clip.duration);
                    }
                }
            }
        });
}

/// A type alias for [`EntityMutExcept`] as used in animation.
pub type AnimationEntityMut<'w, 's> =
    EntityMutExcept<'w, 's, (AnimationTarget, AnimationPlayer, AnimationGraphHandle)>;

/// A system that modifies animation targets (e.g. bones in a skinned mesh)
/// according to the currently-playing animations.
pub fn animate_targets(
    par_commands: ParallelCommands,
    clips: Res<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    threaded_animation_graphs: Res<ThreadedAnimationGraphs>,
    players: Query<(&AnimationPlayer, &AnimationGraphHandle)>,
    mut targets: Query<(Entity, &AnimationTarget, AnimationEntityMut)>,
    animation_evaluation_state: Local<ThreadLocal<RefCell<AnimationEvaluationState>>>,
) {
    // Evaluate all animation targets in parallel.
    targets
        .par_iter_mut()
        .for_each(|(entity, target, entity_mut)| {
            let &AnimationTarget {
                id: target_id,
                player: player_id,
            } = target;

            let (animation_player, animation_graph_id) =
                if let Ok((player, graph_handle)) = players.get(player_id) {
                    (player, graph_handle.id())
                } else {
                    trace!(
                        "Either an animation player {} or a graph was missing for the target \
                         entity {} ({:?}); no animations will play this frame",
                        player_id,
                        entity_mut.id(),
                        entity_mut.get::<Name>(),
                    );
                    return;
                };

            // The graph might not have loaded yet. Safely bail.
            let Some(animation_graph) = graphs.get(animation_graph_id) else {
                return;
            };

            let Some(threaded_animation_graph) =
                threaded_animation_graphs.0.get(&animation_graph_id)
            else {
                return;
            };

            // Determine which mask groups this animation target belongs to.
            let target_mask = animation_graph
                .mask_groups
                .get(&target_id)
                .cloned()
                .unwrap_or_default();

            let mut evaluation_state = animation_evaluation_state.get_or_default().borrow_mut();
            let evaluation_state = &mut *evaluation_state;

            // Evaluate the graph.
            for &animation_graph_node_index in threaded_animation_graph.threaded_graph.iter() {
                let Some(animation_graph_node) = animation_graph.get(animation_graph_node_index)
                else {
                    continue;
                };

                match animation_graph_node.node_type {
                    AnimationNodeType::Blend => {
                        // This is a blend node.
                        for edge_index in threaded_animation_graph.sorted_edge_ranges
                            [animation_graph_node_index.index()]
                        .clone()
                        {
                            if let Err(err) = evaluation_state.blend_all(
                                threaded_animation_graph.sorted_edges[edge_index as usize],
                            ) {
                                warn!("Failed to blend animation: {:?}", err);
                            }
                        }

                        if let Err(err) = evaluation_state.push_blend_register_all(
                            animation_graph_node.weight,
                            animation_graph_node_index,
                        ) {
                            warn!("Animation blending failed: {:?}", err);
                        }
                    }

                    AnimationNodeType::Add => {
                        // This is an additive blend node.
                        for edge_index in threaded_animation_graph.sorted_edge_ranges
                            [animation_graph_node_index.index()]
                        .clone()
                        {
                            if let Err(err) = evaluation_state
                                .add_all(threaded_animation_graph.sorted_edges[edge_index as usize])
                            {
                                warn!("Failed to blend animation: {:?}", err);
                            }
                        }

                        if let Err(err) = evaluation_state.push_blend_register_all(
                            animation_graph_node.weight,
                            animation_graph_node_index,
                        ) {
                            warn!("Animation blending failed: {:?}", err);
                        }
                    }

                    AnimationNodeType::Clip(ref animation_clip_handle) => {
                        // This is a clip node.
                        let Some(active_animation) = animation_player
                            .active_animations
                            .get(&animation_graph_node_index)
                        else {
                            continue;
                        };

                        // If the weight is zero or the current animation target is
                        // masked out, stop here.
                        if active_animation.weight == 0.0
                            || (target_mask
                                & threaded_animation_graph.computed_masks
                                    [animation_graph_node_index.index()])
                                != 0
                        {
                            continue;
                        }

                        let Some(clip) = clips.get(animation_clip_handle) else {
                            continue;
                        };

                        if !active_animation.paused {
                            // Trigger all animation events that occurred this tick, if any.
                            if let Some(triggered_events) = TriggeredEvents::from_animation(
                                AnimationEventTarget::Node(target_id),
                                clip,
                                active_animation,
                            ) && !triggered_events.is_empty()
                            {
                                par_commands.command_scope(move |mut commands| {
                                    for TimedAnimationEvent { time, event } in
                                        triggered_events.iter()
                                    {
                                        event.trigger(
                                            &mut commands,
                                            entity,
                                            *time,
                                            active_animation.weight,
                                        );
                                    }
                                });
                            }
                        }

                        let Some(curves) = clip.curves_for_target(target_id) else {
                            continue;
                        };

                        let weight = active_animation.weight * animation_graph_node.weight;
                        let seek_time = active_animation.seek_time;

                        for curve in curves {
                            // Fetch the curve evaluator. Curve evaluator types
                            // are unique to each property, but shared among all
                            // curve types. For example, given two curve types A
                            // and B, `RotationCurve<A>` and `RotationCurve<B>`
                            // will both yield a `RotationCurveEvaluator` and
                            // therefore will share the same evaluator in this
                            // table.
                            let curve_evaluator_id = (*curve.0).evaluator_id();
                            let curve_evaluator = evaluation_state
                                .evaluators
                                .get_or_insert_with(curve_evaluator_id.clone(), || {
                                    curve.0.create_evaluator()
                                });

                            evaluation_state
                                .current_evaluators
                                .insert(curve_evaluator_id);

                            if let Err(err) = AnimationCurve::apply(
                                &*curve.0,
                                curve_evaluator,
                                seek_time,
                                weight,
                                animation_graph_node_index,
                            ) {
                                warn!("Animation application failed: {:?}", err);
                            }
                        }
                    }
                }
            }

            if let Err(err) = evaluation_state.commit_all(entity_mut) {
                warn!("Animation application failed: {:?}", err);
            }
        });
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
            .init_resource::<ThreadedAnimationGraphs>()
            .add_systems(
                PostUpdate,
                (
                    graph::thread_animation_graphs.before(AssetEventSystems),
                    advance_transitions,
                    advance_animations,
                    // TODO: `animate_targets` can animate anything, so
                    // ambiguity testing currently considers it ambiguous with
                    // every other system in `PostUpdate`. We may want to move
                    // it to its own system set after `Update` but before
                    // `PostUpdate`. For now, we just disable ambiguity testing
                    // for this system.
                    animate_targets
                        .before(bevy_mesh::InheritWeightSystems)
                        .ambiguous_with_all(),
                    trigger_untargeted_animation_events,
                    expire_completed_transitions,
                )
                    .chain()
                    .in_set(AnimationSystems)
                    .before(TransformSystems::Propagate),
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

impl<T: AsRef<str>> FromIterator<T> for AnimationTargetId {
    /// Creates a new [`AnimationTargetId`] by hashing a list of strings.
    ///
    /// Typically, this will be the path from the animation root to the
    /// animation target (e.g. bone) that is to be animated.
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut blake3 = blake3::Hasher::new();
        blake3.update(ANIMATION_TARGET_NAMESPACE.as_bytes());
        for str in iter {
            blake3.update(str.as_ref().as_bytes());
        }
        let hash = blake3.finalize().as_bytes()[0..16].try_into().unwrap();
        Self(*uuid::Builder::from_sha1_bytes(hash).as_uuid())
    }
}

impl From<&Name> for AnimationTargetId {
    fn from(name: &Name) -> Self {
        AnimationTargetId::from_name(name)
    }
}

impl AnimationEvaluationState {
    /// Calls [`AnimationCurveEvaluator::blend`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// The given `node_index` is the node that we're evaluating.
    fn blend_all(
        &mut self,
        node_index: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .blend(node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::add`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// The given `node_index` is the node that we're evaluating.
    fn add_all(&mut self, node_index: AnimationNodeIndex) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .add(node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::push_blend_register`] on all curve
    /// evaluator types that we've been building up for a single target.
    ///
    /// The `weight` parameter is the weight that should be pushed onto the
    /// stack, while the `node_index` parameter is the node that we're
    /// evaluating.
    fn push_blend_register_all(
        &mut self,
        weight: f32,
        node_index: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_evaluators.keys() {
            self.evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .push_blend_register(weight, node_index)?;
        }
        Ok(())
    }

    /// Calls [`AnimationCurveEvaluator::commit`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// This is the call that actually writes the computed values into the
    /// components being animated.
    fn commit_all(
        &mut self,
        mut entity_mut: AnimationEntityMut,
    ) -> Result<(), AnimationEvaluationError> {
        self.current_evaluators.clear(|id| {
            self.evaluators
                .get_mut(id)
                .unwrap()
                .commit(entity_mut.reborrow())
        })
    }
}

/// All the events from an [`AnimationClip`] that occurred this tick.
#[derive(Debug, Clone)]
struct TriggeredEvents<'a> {
    direction: TriggeredEventsDir,
    lower: &'a [TimedAnimationEvent],
    upper: &'a [TimedAnimationEvent],
}

impl<'a> TriggeredEvents<'a> {
    fn from_animation(
        target: AnimationEventTarget,
        clip: &'a AnimationClip,
        active_animation: &ActiveAnimation,
    ) -> Option<Self> {
        let events = clip.events.get(&target)?;
        let reverse = active_animation.is_playback_reversed();
        let is_finished = active_animation.is_finished();

        // Return early if the animation have finished on a previous tick.
        if is_finished && !active_animation.just_completed {
            return None;
        }

        // The animation completed this tick, while still playing.
        let looping = active_animation.just_completed && !is_finished;
        let direction = match (reverse, looping) {
            (false, false) => TriggeredEventsDir::Forward,
            (false, true) => TriggeredEventsDir::ForwardLooping,
            (true, false) => TriggeredEventsDir::Reverse,
            (true, true) => TriggeredEventsDir::ReverseLooping,
        };

        let last_time = active_animation.last_seek_time?;
        let this_time = active_animation.seek_time;

        let (lower, upper) = match direction {
            // Return all events where last_time <= event.time < this_time.
            TriggeredEventsDir::Forward => {
                let start = events.partition_point(|event| event.time < last_time);
                // The animation finished this tick, return any remaining events.
                if is_finished {
                    (&events[start..], &events[0..0])
                } else {
                    let end = events.partition_point(|event| event.time < this_time);
                    (&events[start..end], &events[0..0])
                }
            }
            // Return all events where this_time < event.time <= last_time.
            TriggeredEventsDir::Reverse => {
                let end = events.partition_point(|event| event.time <= last_time);
                // The animation finished, return any remaining events.
                if is_finished {
                    (&events[..end], &events[0..0])
                } else {
                    let start = events.partition_point(|event| event.time <= this_time);
                    (&events[start..end], &events[0..0])
                }
            }
            // The animation is looping this tick and we have to return events where
            // either last_tick <= event.time or event.time < this_tick.
            TriggeredEventsDir::ForwardLooping => {
                let upper_start = events.partition_point(|event| event.time < last_time);
                let lower_end = events.partition_point(|event| event.time < this_time);

                let upper = &events[upper_start..];
                let lower = &events[..lower_end];
                (lower, upper)
            }
            // The animation is looping this tick and we have to return events where
            // either last_tick >= event.time or event.time > this_tick.
            TriggeredEventsDir::ReverseLooping => {
                let lower_end = events.partition_point(|event| event.time <= last_time);
                let upper_start = events.partition_point(|event| event.time <= this_time);

                let upper = &events[upper_start..];
                let lower = &events[..lower_end];
                (lower, upper)
            }
        };
        Some(Self {
            direction,
            lower,
            upper,
        })
    }

    fn is_empty(&self) -> bool {
        self.lower.is_empty() && self.upper.is_empty()
    }

    fn iter(&self) -> TriggeredEventsIter<'_> {
        match self.direction {
            TriggeredEventsDir::Forward => TriggeredEventsIter::Forward(self.lower.iter()),
            TriggeredEventsDir::Reverse => TriggeredEventsIter::Reverse(self.lower.iter().rev()),
            TriggeredEventsDir::ForwardLooping => TriggeredEventsIter::ForwardLooping {
                upper: self.upper.iter(),
                lower: self.lower.iter(),
            },
            TriggeredEventsDir::ReverseLooping => TriggeredEventsIter::ReverseLooping {
                lower: self.lower.iter().rev(),
                upper: self.upper.iter().rev(),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TriggeredEventsDir {
    /// The animation is playing normally
    Forward,
    /// The animation is playing in reverse
    Reverse,
    /// The animation is looping this tick
    ForwardLooping,
    /// The animation playing in reverse and looping this tick
    ReverseLooping,
}

#[derive(Debug, Clone)]
enum TriggeredEventsIter<'a> {
    Forward(slice::Iter<'a, TimedAnimationEvent>),
    Reverse(iter::Rev<slice::Iter<'a, TimedAnimationEvent>>),
    ForwardLooping {
        upper: slice::Iter<'a, TimedAnimationEvent>,
        lower: slice::Iter<'a, TimedAnimationEvent>,
    },
    ReverseLooping {
        lower: iter::Rev<slice::Iter<'a, TimedAnimationEvent>>,
        upper: iter::Rev<slice::Iter<'a, TimedAnimationEvent>>,
    },
}

impl<'a> Iterator for TriggeredEventsIter<'a> {
    type Item = &'a TimedAnimationEvent;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            TriggeredEventsIter::Forward(iter) => iter.next(),
            TriggeredEventsIter::Reverse(rev) => rev.next(),
            TriggeredEventsIter::ForwardLooping { upper, lower } => {
                upper.next().or_else(|| lower.next())
            }
            TriggeredEventsIter::ReverseLooping { lower, upper } => {
                lower.next().or_else(|| upper.next())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_animation;
    use bevy_reflect::{DynamicMap, Map};

    use super::*;

    #[derive(AnimationEvent, Reflect, Clone)]
    struct A;

    #[track_caller]
    fn assert_triggered_events_with(
        active_animation: &ActiveAnimation,
        clip: &AnimationClip,
        expected: impl Into<Vec<f32>>,
    ) {
        let Some(events) =
            TriggeredEvents::from_animation(AnimationEventTarget::Root, clip, active_animation)
        else {
            assert_eq!(expected.into(), Vec::<f32>::new());
            return;
        };
        let got: Vec<_> = events.iter().map(|t| t.time).collect();
        assert_eq!(
            expected.into(),
            got,
            "\n{events:#?}\nlast_time: {:?}\nthis_time:{}",
            active_animation.last_seek_time,
            active_animation.seek_time
        );
    }

    #[test]
    fn test_multiple_events_triggers() {
        let mut active_animation = ActiveAnimation {
            repeat: RepeatAnimation::Forever,
            ..Default::default()
        };
        let mut clip = AnimationClip {
            duration: 1.0,
            ..Default::default()
        };
        clip.add_event(0.5, A);
        clip.add_event(0.5, A);
        clip.add_event(0.5, A);

        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.8, clip.duration); // 0.0 : 0.8
        assert_triggered_events_with(&active_animation, &clip, [0.5, 0.5, 0.5]);

        clip.add_event(1.0, A);
        clip.add_event(0.0, A);
        clip.add_event(1.0, A);
        clip.add_event(0.0, A);

        active_animation.update(0.4, clip.duration); // 0.8 : 0.2
        assert_triggered_events_with(&active_animation, &clip, [1.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_events_triggers() {
        let mut active_animation = ActiveAnimation::default();
        let mut clip = AnimationClip::default();
        clip.add_event(0.2, A);
        clip.add_event(0.0, A);
        assert_eq!(0.2, clip.duration);

        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.0 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.0]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.2
        assert_triggered_events_with(&active_animation, &clip, [0.2]);
        active_animation.update(0.1, clip.duration); // 0.2 : 0.2
        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.2 : 0.2
        assert_triggered_events_with(&active_animation, &clip, []);

        active_animation.speed = -1.0;
        active_animation.completions = 0;
        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.2 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.2]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.0
        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.0 : 0.0
        assert_triggered_events_with(&active_animation, &clip, [0.0]);
        active_animation.update(0.1, clip.duration); // 0.0 : 0.0
        assert_triggered_events_with(&active_animation, &clip, []);
    }

    #[test]
    fn test_events_triggers_looping() {
        let mut active_animation = ActiveAnimation {
            repeat: RepeatAnimation::Forever,
            ..Default::default()
        };
        let mut clip = AnimationClip::default();
        clip.add_event(0.3, A);
        clip.add_event(0.0, A);
        clip.add_event(0.2, A);
        assert_eq!(0.3, clip.duration);

        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.0 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.0]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.2
        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.2 : 0.3
        assert_triggered_events_with(&active_animation, &clip, [0.2, 0.3]);
        active_animation.update(0.1, clip.duration); // 0.3 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.0]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.2
        assert_triggered_events_with(&active_animation, &clip, []);

        active_animation.speed = -1.0;
        active_animation.update(0.1, clip.duration); // 0.2 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.2]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.0
        assert_triggered_events_with(&active_animation, &clip, []);
        active_animation.update(0.1, clip.duration); // 0.0 : 0.2
        assert_triggered_events_with(&active_animation, &clip, [0.0, 0.3]);
        active_animation.update(0.1, clip.duration); // 0.2 : 0.1
        assert_triggered_events_with(&active_animation, &clip, [0.2]);
        active_animation.update(0.1, clip.duration); // 0.1 : 0.0
        assert_triggered_events_with(&active_animation, &clip, []);

        active_animation.replay();
        active_animation.update(clip.duration, clip.duration); // 0.0 : 0.0
        assert_triggered_events_with(&active_animation, &clip, [0.0, 0.3, 0.2]);

        active_animation.replay();
        active_animation.seek_time = clip.duration;
        active_animation.last_seek_time = Some(clip.duration);
        active_animation.update(clip.duration, clip.duration); // 0.3 : 0.0
        assert_triggered_events_with(&active_animation, &clip, [0.3, 0.2]);
    }

    #[test]
    fn test_animation_node_index_as_key_of_dynamic_map() {
        let mut map = DynamicMap::default();
        map.insert_boxed(
            Box::new(AnimationNodeIndex::new(0)),
            Box::new(ActiveAnimation::default()),
        );
    }
}
