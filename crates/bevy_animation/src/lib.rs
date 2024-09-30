#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Animation for the game engine Bevy

extern crate alloc;

pub mod animatable;
pub mod graph;
pub mod keyframes;
pub mod transition;
mod util;

use alloc::collections::BTreeMap;
use core::{
    any::{Any, TypeId},
    cell::RefCell,
    fmt::Debug,
    hash::{Hash, Hasher},
    iter,
};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Asset, AssetApp, Assets, Handle};
use bevy_core::Name;
use bevy_ecs::{
    entity::MapEntities, prelude::*, reflect::ReflectMapEntities, world::EntityMutExcept,
};
use bevy_math::FloatExt;
use bevy_reflect::{
    prelude::ReflectDefault, utility::NonGenericTypeInfoCell, ApplyError, DynamicStruct, FieldIter,
    FromReflect, FromType, GetTypeRegistration, NamedField, PartialReflect, Reflect,
    ReflectFromPtr, ReflectKind, ReflectMut, ReflectOwned, ReflectRef, Struct, StructInfo,
    TypeInfo, TypePath, TypeRegistration, Typed,
};
use bevy_time::Time;
use bevy_transform::{prelude::Transform, TransformSystem};
use bevy_ui::UiSystem;
use bevy_utils::{
    hashbrown::HashMap,
    tracing::{trace, warn},
    NoOpHash,
};
use fixedbitset::FixedBitSet;
use graph::AnimationMask;
use petgraph::{graph::NodeIndex, Direction};
use serde::{Deserialize, Serialize};
use thread_local::ThreadLocal;
use uuid::Uuid;

/// The animation prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        animatable::*, graph::*, keyframes::*, transition::*, AnimationClip, AnimationPlayer,
        AnimationPlugin, Interpolation, VariableCurve,
    };
}

use crate::{
    graph::{AnimationGraph, AnimationGraphAssetLoader, AnimationNodeIndex},
    keyframes::Keyframes,
    transition::{advance_transitions, expire_completed_transitions, AnimationTransitions},
};

/// The [UUID namespace] of animation targets (e.g. bones).
///
/// [UUID namespace]: https://en.wikipedia.org/wiki/Universally_unique_identifier#Versions_3_and_5_(namespace_name-based)
pub static ANIMATION_TARGET_NAMESPACE: Uuid = Uuid::from_u128(0x3179f519d9274ff2b5966fd077023911);

/// Describes how an attribute of a [`Transform`] or
/// [`bevy_render::mesh::morph::MorphWeights`] should be animated.
///
/// `keyframe_timestamps` and `keyframes` should have the same length.
#[derive(Debug, TypePath)]
pub struct VariableCurve {
    /// Timestamp for each of the keyframes.
    pub keyframe_timestamps: Vec<f32>,
    /// List of the keyframes.
    ///
    /// The representation will depend on the interpolation type of this curve:
    ///
    /// - for `Interpolation::Step` and `Interpolation::Linear`, each keyframe is a single value
    /// - for `Interpolation::CubicSpline`, each keyframe is made of three values for `tangent_in`,
    ///     `keyframe_value` and `tangent_out`
    pub keyframes: Box<dyn Keyframes>,
    /// Interpolation method to use between keyframes.
    pub interpolation: Interpolation,
}

impl Clone for VariableCurve {
    fn clone(&self) -> Self {
        VariableCurve {
            keyframe_timestamps: self.keyframe_timestamps.clone(),
            keyframes: Keyframes::clone_value(&*self.keyframes),
            interpolation: self.interpolation,
        }
    }
}

impl VariableCurve {
    /// Creates a new curve from timestamps, keyframes, and interpolation type.
    ///
    /// The two arrays must have the same length.
    pub fn new<K>(
        keyframe_timestamps: Vec<f32>,
        keyframes: impl Into<K>,
        interpolation: Interpolation,
    ) -> VariableCurve
    where
        K: Keyframes,
    {
        VariableCurve {
            keyframe_timestamps,
            keyframes: Box::new(keyframes.into()),
            interpolation,
        }
    }

    /// Creates a new curve from timestamps and keyframes with no interpolation.
    ///
    /// The two arrays must have the same length.
    pub fn step<K>(
        keyframe_timestamps: impl Into<Vec<f32>>,
        keyframes: impl Into<K>,
    ) -> VariableCurve
    where
        K: Keyframes,
    {
        VariableCurve::new(keyframe_timestamps.into(), keyframes, Interpolation::Step)
    }

    /// Creates a new curve from timestamps and keyframes with linear
    /// interpolation.
    ///
    /// The two arrays must have the same length.
    pub fn linear<K>(
        keyframe_timestamps: impl Into<Vec<f32>>,
        keyframes: impl Into<K>,
    ) -> VariableCurve
    where
        K: Keyframes,
    {
        VariableCurve::new(keyframe_timestamps.into(), keyframes, Interpolation::Linear)
    }

    /// Creates a new curve from timestamps and keyframes with no interpolation.
    ///
    /// The two arrays must have the same length.
    pub fn cubic_spline<K>(
        keyframe_timestamps: impl Into<Vec<f32>>,
        keyframes: impl Into<K>,
    ) -> VariableCurve
    where
        K: Keyframes,
    {
        VariableCurve::new(
            keyframe_timestamps.into(),
            keyframes,
            Interpolation::CubicSpline,
        )
    }

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

// We have to implement `PartialReflect` manually because of the embedded
// `Box<dyn Keyframes>`, which can't be automatically derived yet.
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
        if let ReflectRef::Struct(struct_value) = value.reflect_ref() {
            for (i, value) in struct_value.iter_fields().enumerate() {
                let name = struct_value.name_at(i).unwrap();
                if let Some(v) = self.field_mut(name) {
                    v.try_apply(value)?;
                }
            }
        } else {
            return Err(ApplyError::MismatchedKinds {
                from_kind: value.reflect_kind(),
                to_kind: ReflectKind::Struct,
            });
        }
        Ok(())
    }

    fn reflect_ref(&self) -> ReflectRef {
        ReflectRef::Struct(self)
    }

    fn reflect_mut(&mut self) -> ReflectMut {
        ReflectMut::Struct(self)
    }

    fn reflect_owned(self: Box<Self>) -> ReflectOwned {
        ReflectOwned::Struct(self)
    }

    fn clone_value(&self) -> Box<dyn PartialReflect> {
        Box::new((*self).clone())
    }
}

// We have to implement `Reflect` manually because of the embedded `Box<dyn
// Keyframes>`, which can't be automatically derived yet.
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

// We have to implement `Struct` manually because of the embedded `Box<dyn
// Keyframes>`, which can't be automatically derived yet.
impl Struct for VariableCurve {
    fn field(&self, name: &str) -> Option<&dyn PartialReflect> {
        match name {
            "keyframe_timestamps" => Some(&self.keyframe_timestamps),
            "keyframes" => Some(self.keyframes.as_partial_reflect()),
            "interpolation" => Some(&self.interpolation),
            _ => None,
        }
    }

    fn field_mut(&mut self, name: &str) -> Option<&mut dyn PartialReflect> {
        match name {
            "keyframe_timestamps" => Some(&mut self.keyframe_timestamps),
            "keyframes" => Some(self.keyframes.as_partial_reflect_mut()),
            "interpolation" => Some(&mut self.interpolation),
            _ => None,
        }
    }

    fn field_at(&self, index: usize) -> Option<&dyn PartialReflect> {
        match index {
            0 => Some(&self.keyframe_timestamps),
            1 => Some(self.keyframes.as_partial_reflect()),
            2 => Some(&self.interpolation),
            _ => None,
        }
    }

    fn field_at_mut(&mut self, index: usize) -> Option<&mut dyn PartialReflect> {
        match index {
            0 => Some(&mut self.keyframe_timestamps),
            1 => Some(self.keyframes.as_partial_reflect_mut()),
            2 => Some(&mut self.interpolation),
            _ => None,
        }
    }

    fn name_at(&self, index: usize) -> Option<&str> {
        match index {
            0 => Some("keyframe_timestamps"),
            1 => Some("keyframes"),
            2 => Some("interpolation"),
            _ => None,
        }
    }

    fn field_len(&self) -> usize {
        3
    }

    fn iter_fields(&self) -> FieldIter {
        FieldIter::new(self)
    }

    fn clone_dynamic(&self) -> DynamicStruct {
        DynamicStruct::from_iter([
            (
                "keyframe_timestamps",
                Box::new(self.keyframe_timestamps.clone()) as Box<dyn PartialReflect>,
            ),
            ("keyframes", PartialReflect::clone_value(&*self.keyframes)),
            (
                "interpolation",
                Box::new(self.interpolation) as Box<dyn PartialReflect>,
            ),
        ])
    }
}

// We have to implement `FromReflect` manually because of the embedded `Box<dyn
// Keyframes>`, which can't be automatically derived yet.
impl FromReflect for VariableCurve {
    fn from_reflect(reflect: &dyn PartialReflect) -> Option<Self> {
        Some(reflect.try_downcast_ref::<VariableCurve>()?.clone())
    }
}

// We have to implement `GetTypeRegistration` manually because of the embedded
// `Box<dyn Keyframes>`, which can't be automatically derived yet.
impl GetTypeRegistration for VariableCurve {
    fn get_type_registration() -> TypeRegistration {
        let mut registration = TypeRegistration::of::<Self>();
        registration.insert::<ReflectFromPtr>(FromType::<Self>::from_type());
        registration
    }
}

// We have to implement `Typed` manually because of the embedded `Box<dyn
// Keyframes>`, which can't be automatically derived yet.
impl Typed for VariableCurve {
    fn type_info() -> &'static TypeInfo {
        static CELL: NonGenericTypeInfoCell = NonGenericTypeInfoCell::new();
        CELL.get_or_set(|| {
            TypeInfo::Struct(StructInfo::new::<Self>(&[
                NamedField::new::<Vec<f32>>("keyframe_timestamps"),
                NamedField::new::<()>("keyframes"),
                NamedField::new::<Interpolation>("interpolation"),
            ]))
        })
    }
}

/// Interpolation method to use between keyframes.
#[derive(Reflect, Clone, Copy, Debug)]
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

/// Why Bevy failed to evaluate an animation.
#[derive(Clone, Debug)]
pub enum AnimationEvaluationError {
    /// The `keyframes` array is too small.
    ///
    /// For curves with `Interpolation::Step` or `Interpolation::Linear`, the
    /// `keyframes` array must have at least as many elements as keyframe
    /// timestamps. For curves with `Interpolation::CubicBezier`, the
    /// `keyframes` array must have at least 3× the number of elements as
    /// keyframe timestamps, in order to account for the tangents.
    KeyframeNotPresent(usize),

    /// The component to be animated isn't present on the animation target.
    ///
    /// To fix this error, make sure the entity to be animated contains all
    /// components that have animation curves.
    ComponentNotPresent(TypeId),

    /// The component to be animated was present, but the property on the
    /// component wasn't present.
    PropertyNotPresent(TypeId),
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
    /// The mask groups that are masked out (i.e. won't be animated) this frame,
    /// taking the `AnimationGraph` into account.
    computed_mask: AnimationMask,
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
            computed_mask: 0,
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
/// Automatically added to any root animations of a `SceneBundle` when it is
/// spawned.
#[derive(Component, Default, Reflect)]
#[reflect(Component, Default)]
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

/// Information needed during the traversal of the animation graph in
/// [`advance_animations`].
#[derive(Default)]
pub struct AnimationGraphEvaluator {
    /// The stack used for the depth-first search of the graph.
    dfs_stack: Vec<NodeIndex>,
    /// The list of visited nodes during the depth-first traversal.
    dfs_visited: FixedBitSet,
    /// Accumulated weights and masks for each node.
    nodes: Vec<EvaluatedAnimationGraphNode>,
}

/// The accumulated weight and computed mask for a single node.
#[derive(Clone, Copy, Default, Debug)]
struct EvaluatedAnimationGraphNode {
    /// The weight that has been accumulated for this node, taking its
    /// ancestors' weights into account.
    weight: f32,
    /// The mask that has been computed for this node, taking its ancestors'
    /// masks into account.
    mask: AnimationMask,
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

                // Calculate weight and mask from the graph.
                let (mut weight, mut mask) = (node.weight, node.mask);
                for parent_index in animation_graph
                    .graph
                    .neighbors_directed(node_index, Direction::Incoming)
                {
                    let evaluated_parent = &evaluator.nodes[parent_index.index()];
                    weight *= evaluated_parent.weight;
                    mask |= evaluated_parent.mask;
                }
                evaluator.nodes[node_index.index()] = EvaluatedAnimationGraphNode { weight, mask };

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

                // Write in the computed weight and mask for this node.
                if let Some(active_animation) = active_animations.get_mut(&node_index) {
                    active_animation.computed_weight = weight;
                    active_animation.computed_mask = mask;
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
    players: Query<(&AnimationPlayer, &Handle<AnimationGraph>)>,
    mut targets: Query<(&AnimationTarget, Option<&mut Transform>, AnimationEntityMut)>,
) {
    // Evaluate all animation targets in parallel.
    targets
        .par_iter_mut()
        .for_each(|(target, mut transform, mut entity_mut)| {
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

            // Determine which mask groups this animation target belongs to.
            let target_mask = animation_graph
                .mask_groups
                .get(&target_id)
                .cloned()
                .unwrap_or_default();

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
                // If the weight is zero or the current animation target is
                // masked out, stop here.
                if active_animation.weight == 0.0
                    || (target_mask & active_animation.computed_mask) != 0
                {
                    continue;
                }

                let Some(clip) = animation_graph
                    .get(animation_graph_node_index)
                    .and_then(|animation_graph_node| animation_graph_node.clip.as_ref())
                    .and_then(|animation_clip_handle| clips.get(animation_clip_handle))
                else {
                    continue;
                };

                let Some(curves) = clip.curves_for_target(target_id) else {
                    continue;
                };

                let weight = active_animation.computed_weight;
                total_weight += weight;

                let weight = weight / total_weight;
                let seek_time = active_animation.seek_time;

                for curve in curves {
                    // Some curves have only one keyframe used to set a transform
                    if curve.keyframe_timestamps.len() == 1 {
                        if let Err(err) = curve.keyframes.apply_single_keyframe(
                            transform.as_mut().map(|transform| transform.reborrow()),
                            entity_mut.reborrow(),
                            weight,
                        ) {
                            warn!("Animation application failed: {:?}", err);
                        }

                        continue;
                    }

                    // Find the best keyframe to interpolate from
                    let step_start = curve.find_interpolation_start_keyframe(seek_time);

                    let timestamp_start = curve.keyframe_timestamps[step_start];
                    let timestamp_end = curve.keyframe_timestamps[step_start + 1];
                    // Compute how far we are through the keyframe, normalized to [0, 1]
                    let lerp = f32::inverse_lerp(timestamp_start, timestamp_end, seek_time)
                        .clamp(0.0, 1.0);

                    if let Err(err) = curve.keyframes.apply_tweened_keyframes(
                        transform.as_mut().map(|transform| transform.reborrow()),
                        entity_mut.reborrow(),
                        curve.interpolation,
                        step_start,
                        lerp,
                        weight,
                        timestamp_end - timestamp_start,
                    ) {
                        warn!("Animation application failed: {:?}", err);
                    }
                }
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
            .add_systems(
                PostUpdate,
                (
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

        self.nodes.clear();
        self.nodes
            .extend(iter::repeat(EvaluatedAnimationGraphNode::default()).take(node_count));
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::TranslationKeyframes, VariableCurve};
    use bevy_math::Vec3;

    // Returns the curve and the keyframe count.
    fn test_variable_curve() -> (VariableCurve, usize) {
        let keyframe_timestamps = vec![1.0, 2.0, 3.0, 4.0];
        let keyframes = vec![
            Vec3::ONE * 0.0,
            Vec3::ONE * 3.0,
            Vec3::ONE * 6.0,
            Vec3::ONE * 9.0,
        ];
        let interpolation = crate::Interpolation::Linear;

        assert_eq!(keyframe_timestamps.len(), keyframes.len());
        let keyframe_count = keyframes.len();

        let variable_curve = VariableCurve::new::<TranslationKeyframes>(
            keyframe_timestamps,
            keyframes,
            interpolation,
        );

        // f32 doesn't impl Ord so we can't easily sort it
        let mut maybe_last_timestamp = None;
        for current_timestamp in &variable_curve.keyframe_timestamps {
            assert!(current_timestamp.is_finite());

            if let Some(last_timestamp) = maybe_last_timestamp {
                assert!(current_timestamp > last_timestamp);
            }
            maybe_last_timestamp = Some(current_timestamp);
        }

        (variable_curve, keyframe_count)
    }

    #[test]
    fn find_current_keyframe_is_in_bounds() {
        let curve = test_variable_curve().0;
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
        let curve = test_variable_curve().0;
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
        let curve = test_variable_curve().0;
        let max_time = *curve.keyframe_timestamps.last().unwrap();

        assert!(max_time < f32::INFINITY);
        let maybe_keyframe = curve.find_current_keyframe(f32::INFINITY);
        assert!(maybe_keyframe.is_none());

        let maybe_keyframe = curve.find_current_keyframe(max_time);
        assert!(maybe_keyframe.is_none());
    }

    #[test]
    fn second_last_keyframe_is_found_correctly() {
        let curve = test_variable_curve().0;

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
        let (curve, keyframe_count) = test_variable_curve();
        let second_last_keyframe = keyframe_count - 2;

        for i in 0..=second_last_keyframe {
            let seek_time = curve.keyframe_timestamps[i];

            let keyframe = curve.find_current_keyframe(seek_time).unwrap();
            assert!(keyframe == i);
        }
    }

    #[test]
    fn exact_and_inexact_keyframes_correspond() {
        let (curve, keyframe_count) = test_variable_curve();
        let second_last_keyframe = keyframe_count - 2;

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
