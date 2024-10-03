#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Animation for the game engine Bevy

extern crate alloc;

pub mod animatable;
pub mod animation_curves;
pub mod gltf_curves;
pub mod graph;
pub mod transition;
mod util;

use core::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    hash::{Hash, Hasher},
    iter,
};
use graph::AnimationNodeType;
use prelude::AnimationCurveEvaluator;

use crate::graph::ThreadedAnimationGraphs;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    entity::{VisitEntities, VisitEntitiesMut},
    prelude::*,
    reflect::{ReflectMapEntities, ReflectVisitEntities, ReflectVisitEntitiesMut},
    world::EntityMutExcept,
};
use bevy_reflect::{
    prelude::ReflectDefault, utility::NonGenericTypeInfoCell, ApplyError, DynamicTupleStruct,
    FromReflect, FromType, GetTypeRegistration, PartialReflect, Reflect, ReflectFromPtr,
    ReflectKind, ReflectMut, ReflectOwned, ReflectRef, TupleStruct, TupleStructFieldIter,
    TupleStructInfo, TypeInfo, TypePath, TypeRegistration, Typed, UnnamedField,
};
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_ui::UiSystem;
use bevy_utils::{
    hashbrown::HashMap,
    tracing::{trace, warn},
    NoOpHash, TypeIdMap,
};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use thread_local::ThreadLocal;
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
    transition::{advance_transitions, expire_completed_transitions, AnimationTransitions},
};

/// The [UUID namespace] of animation targets (e.g. bones).
///
/// [UUID namespace]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Versions_3_and_5_(namespace_name-based)
pub static ANIMATION_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x3179f519d9274ff2b5966fd077023911);

/// Contains an [animation curve] which is used to animate entities.
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

// We have to implement `PartialReflect` manually because of the embedded
// `Box<dyn AnimationCurve>`, which can't be automatically derived yet.
impl PartialReflect for VariableCurve {
    #[inline]
    fn get_represented_type_info(&self) -> Option<&'static TypeInfo> {
        Some(<Self as Typed>::type_info())
    }

    #[inline]
    fn into_partial_reflect(self: Box<Self>) -> Box<dyn PartialReflect> {
        self
    }

    #[inline]
    fn as_partial_reflect(&self) -> &dyn PartialReflect {
        self
    }

    #[inline]
    fn as_partial_reflect_mut(&mut self) -> &mut dyn PartialReflect {
        self
    }

    fn try_into_reflect(self: Box<Self>) -> Result<Box<dyn Reflect>, Box<dyn PartialReflect>> {
        Ok(self)
    }

    #[inline]
    fn try_as_reflect(&self) -> Option<&dyn Reflect> {
        Some(self)
    }

    #[inline]
    fn try_as_reflect_mut(&mut self) -> Option<&mut dyn Reflect> {
        Some(self)
    }

    fn try_apply(&mut self, value: &dyn PartialReflect) -> Result<(), ApplyError> {
        if let ReflectRef::TupleStruct(tuple_value) = value.reflect_ref() {
            for (i, value) in tuple_value.iter_fields().enumerate() {
                if let Some(v) = self.field_mut(i) {
                    v.try_apply(value)?;
                }
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::TupleStruct,
            });
        }
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::TupleStruct(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::TupleStruct(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::TupleStruct(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new((*self).clone())
    }
}

// We have to implement `Reflect` manually because of the embedded `Box<dyn
// AnimationCurve>`, which can't be automatically derived yet.
impl Reflect for VariableCurve {
    #[inline]
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }

    #[inline]
    fn as_any(&self) -> &dyn Any {
        self
    }

    #[inline]
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    #[inline]
    fn into_reflect(self: Box<Self>) -> Box<dyn Reflect> {
        self
    }

    #[inline]
    fn as_reflect(&self) -> &dyn Reflect {
        self
    }

    #[inline]
    fn as_reflect_mut(&mut self) -> &mut dyn Reflect {
        self
    }

    #[inline]
    fn set(&mut self, value: Box<dyn Reflect>) -> Result<(), Box<dyn Reflect>> {
        *self = value.take()?;
        Ok(())
    }
}

// We have to implement `TupleStruct` manually because of the embedded `Box<dyn
// AnimationCurve>`, which can't be automatically derived yet.
impl TupleStruct for VariableCurve {
    fn field(&self, index: usize) -> Option<&dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect()),
            _ => None,
        }
    }

    fn field_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        match index {
            0 => Some(self.0.as_partial_reflect_mut()),
            _ => None,
        }
    }

    fn field_len(&self) -> usize {
        1
    }

    fn iter_fields(&self) -> TupleStructFieldIter {
        TupleStructFieldIter::new(self)
    }

    fn clone_dynamic(&self) -> DynamicTupleStruct {
        DynamicTupleStruct::from_iter([PartialReflect::clone_value(&*self.0)])
    }
}

// We have to implement `FromReflect` manually because of the embedded `Box<dyn
// AnimationCurve>`, which can't be automatically derived yet.
impl FromReflect for VariableCurve {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<VariableCurve>()?.clone())
    }
}

// We have to implement `GetTypeRegistration` manually because of the embedded
// `Box<dyn AnimationCurve>`, which can't be automatically derived yet.
impl GetTypeRegistration for VariableCurve {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

// We have to implement `Typed` manually because of the embedded `Box<dyn
// AnimationCurve>`, which can't be automatically derived yet.
impl Typed for VariableCurve {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| {
            TypeInfo::TupleStruct(TupleStructInfo::new::<Self>(&[UnnamedField::new::<()>(0)]))
        })
    }
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Reflect, Debug, Serialize, Deserialize)]
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
#[derive(Clone, Copy, Component, Reflect, VisitEntities, VisitEntitiesMut)]
#[reflect(Component, MapEntities, VisitEntities, VisitEntitiesMut)]
pub struct AnimationTarget {
    /// The ID of this animation target.
    ///
    /// Typically, this is derived from the path.
    #[visit_entities(ignore)]
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

    /// Adds an [`AnimationCurve`] to an [`AnimationTarget`] named by an
    /// [`AnimationTargetId`].
    ///
    /// If the curve extends beyond the current duration of this clip, this
    /// method lengthens this clip to include the entire time span that the
    /// curve covers.
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
/// An stopped animation is considered no longer active.
#[derive(Debug, Clone, Copy, Reflect)]
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
    /// Number of times the animation has completed.
    /// If the animation is playing in reverse, this increments when the animation passes the start.
    completions: u32,
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

/// Animation controls.
///
/// Automatically added to any root animations of a scene when it is
/// spawned.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
pub struct AnimationPlayer {
    active_animations: HashMap<AnimationNodeIndex, ActiveAnimation>,
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

/// Temporary data that the [`animate_targets`] system maintains.
#[derive(Default)]
pub struct AnimationEvaluationState {
    /// Stores all [`AnimationCurveEvaluator`]s corresponding to properties that
    /// we've seen so far.
    ///
    /// This is a mapping from the type ID of an animation curve evaluator to
    /// the animation curve evaluator itself.
    ///
    /// For efficiency's sake, the [`AnimationCurveEvaluator`]s are cached from
    /// frame to frame and animation target to animation target. Therefore,
    /// there may be entries in this list corresponding to properties that the
    /// current [`AnimationPlayer`] doesn't animate. To iterate only over the
    /// properties that are currently being animated, consult the
    /// [`Self::current_curve_evaluator_types`] set.
    curve_evaluators: TypeIdMap<Box<dyn AnimationCurveEvaluator>>,

    /// The set of [`AnimationCurveEvaluator`] types that the current
    /// [`AnimationPlayer`] is animating.
    ///
    /// This is built up as new curve evaluators are encountered during graph
    /// traversal.
    current_curve_evaluator_types: TypeIdMap<()>,
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

    #[deprecated = "Use `animation_is_playing` instead"]
    /// Check if the given animation node is being played.
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
) {
    let delta_seconds = time.delta_seconds();
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
                    if !active_animation.paused {
                        if let AnimationNodeType::Clip(ref clip_handle) = node.node_type {
                            if let Some(clip) = animation_clips.get(clip_handle) {
                                active_animation.update(delta_seconds, clip.duration);
                            }
                        }
                    }
                }
            }
        });
}

/// A type alias for [`EntityMutExcept`] as used in animation.
pub type AnimationEntityMut<'w> = EntityMutExcept<
    'w,
    (
        AnimationTarget,
        Transform,
        AnimationPlayer,
        Handle<AnimationGraph>,
    ),
>;

/// A system that modifies animation targets (e.g. bones in a skinned mesh)
/// according to the currently-playing animations.
pub fn animate_targets(
    clips: Res<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    threaded_animation_graphs: Res<ThreadedAnimationGraphs>,
    players: Query<(&AnimationPlayer, &Handle<AnimationGraph>)>,
    mut targets: Query<(&AnimationTarget, Option<&mut Transform>, AnimationEntityMut)>,
    animation_evaluation_state: Local<ThreadLocal<RefCell<AnimationEvaluationState>>>,
) {
    // Evaluate all animation targets in parallel.
    targets
        .par_iter_mut()
        .for_each(|(target, transform, entity_mut)| {
            let &AnimationTarget {
                id: target_id,
                player: player_id,
            } = target;

            let (animation_player, animation_graph_id) =
                if let Ok((player, graph_handle)) = players.get(player_id) {
                    (player, graph_handle.id())
                } else {
                    trace!(
                        "Either an animation player {:?} or a graph was missing for the target \
                         entity {:?} ({:?}); no animations will play this frame",
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
                    AnimationNodeType::Blend | AnimationNodeType::Add => {
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

                        let Some(curves) = clip.curves_for_target(target_id) else {
                            continue;
                        };

                        let weight = active_animation.weight;
                        let seek_time = active_animation.seek_time;

                        for curve in curves {
                            // Fetch the curve evaluator. Curve evaluator types
                            // are unique to each property, but shared among all
                            // curve types. For example, given two curve types A
                            // and B, `RotationCurve<A>` and `RotationCurve<B>`
                            // will both yield a `RotationCurveEvaluator` and
                            // therefore will share the same evaluator in this
                            // table.
                            let curve_evaluator_type_id = (*curve.0).evaluator_type();
                            let curve_evaluator = evaluation_state
                                .curve_evaluators
                                .entry(curve_evaluator_type_id)
                                .or_insert_with(|| curve.0.create_evaluator());

                            evaluation_state
                                .current_curve_evaluator_types
                                .insert(curve_evaluator_type_id, ());

                            if let Err(err) = AnimationCurve::apply(
                                &*curve.0,
                                &mut **curve_evaluator,
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

            if let Err(err) = evaluation_state.commit_all(transform, entity_mut) {
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
            .register_type::<AnimationPlayer>()
            .register_type::<AnimationTarget>()
            .register_type::<AnimationTransitions>()
            .register_type::<NodeIndex>()
            .register_type::<ThreadedAnimationGraphs>()
            .init_resource::<ThreadedAnimationGraphs>()
            .add_systems(
                PostUpdate,
                (
                    graph::thread_animation_graphs,
                    advance_transitions,
                    advance_animations,
                    // TODO: `animate_targets` can animate anything, so
                    // ambiguity testing currently considers it ambiguous with
                    // every other system in `PostUpdate`. We may want to move
                    // it to its own system set after `Update` but before
                    // `PostUpdate`. For now, we just disable ambiguity testing
                    // for this system.
                    animate_targets
                        .after(bevy_render::mesh::morph::inherit_weights)
                        .ambiguous_with_all(),
                    expire_completed_transitions,
                )
                    .chain()
                    .before(TransformSystem::TransformPropagate)
                    .before(UiSystem::Prepare),
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

impl AnimationEvaluationState {
    /// Calls [`AnimationCurveEvaluator::blend`] on all curve evaluator types
    /// that we've been building up for a single target.
    ///
    /// The given `node_index` is the node that we're evaluating.
    fn blend_all(
        &mut self,
        node_index: AnimationNodeIndex,
    ) -> Result<(), AnimationEvaluationError> {
        for curve_evaluator_type in self.current_curve_evaluator_types.keys() {
            self.curve_evaluators
                .get_mut(curve_evaluator_type)
                .unwrap()
                .blend(node_index)?;
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
        for curve_evaluator_type in self.current_curve_evaluator_types.keys() {
            self.curve_evaluators
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
        mut transform: Option<Mut<Transform>>,
        mut entity_mut: AnimationEntityMut,
    ) -> Result<(), AnimationEvaluationError> {
        for (curve_evaluator_type, _) in self.current_curve_evaluator_types.drain() {
            self.curve_evaluators
                .get_mut(&curve_evaluator_type)
                .unwrap()
                .commit(
                    transform.as_mut().map(|transform| transform.reborrow()),
                    entity_mut.reborrow(),
                )?;
        }
        Ok(())
    }
}
