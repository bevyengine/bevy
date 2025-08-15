use core::marker::PhantomData;

use bevy_app::{Plugin, PostUpdate};
use bevy_color::Srgba;
use bevy_ecs::component::{ComponentId, StorageType};
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::query::QueryState;
use bevy_ecs::schedule::IntoScheduleConfigs as _;
use bevy_ecs::world::{DeferredWorld, EntityMutExcept, World};
use bevy_ecs::{
    component::{Component, Mutable},
    world::Mut,
};
use bevy_math::{curve::EaseFunction, Curve, StableInterpolate};
use bevy_math::{Rot2, Vec2};
use bevy_platform::collections::HashMap;
use bevy_time::Time;
use bevy_ui::{BackgroundColor, BorderColor, Node, UiSystems, UiTransform, Val};

/// Why the interpolation failed.
#[derive(Clone, Debug)]
pub enum InterpolationError {
    /// The values to be interpolated are not in the same units.
    MismatchedUnits,
}

/// A trait that indicates that a value may be interpolable via [`StableInterpolate`]. An
/// interpolation may fail if the values have different units - for example, attempting to
/// interpolate between `Val::Px` and `Val::Percent` will fail, even though they are the same
/// type.
pub trait FallibleInterpolate: Clone {
    /// Attempt to interpolate the value. This may fail if the two interpolation values have
    /// different units.
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError>;
}

impl FallibleInterpolate for f32 {
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError> {
        Ok(self.interpolate_stable(other, t))
    }
}

impl FallibleInterpolate for Vec2 {
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError> {
        Ok(self.interpolate_stable(other, t))
    }
}

impl FallibleInterpolate for Srgba {
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError> {
        Ok(self.interpolate_stable(other, t))
    }
}

impl FallibleInterpolate for Rot2 {
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError> {
        Ok(self.interpolate_stable(other, t))
    }
}

impl FallibleInterpolate for Val {
    fn try_interpolate(&self, other: &Self, t: f32) -> Result<Self, InterpolationError> {
        match (self, other) {
            (Val::Px(a), Val::Px(b)) => Ok(Val::Px(a.interpolate_stable(b, t))),
            (Val::Percent(a), Val::Percent(b)) => Ok(Val::Percent(a.interpolate_stable(b, t))),
            (Val::Vw(a), Val::Vw(b)) => Ok(Val::Vw(a.interpolate_stable(b, t))),
            (Val::Vh(a), Val::Vh(b)) => Ok(Val::Vh(a.interpolate_stable(b, t))),
            (Val::VMin(a), Val::VMin(b)) => Ok(Val::VMin(a.interpolate_stable(b, t))),
            (Val::VMax(a), Val::VMax(b)) => Ok(Val::VMax(a.interpolate_stable(b, t))),
            (Val::Auto, Val::Auto) => Ok(Val::Auto),
            _ => Err(InterpolationError::MismatchedUnits),
        }
    }
}

/// Represents an animatable property such as `BackgroundColor` or `Width`.
pub trait TransitionProperty {
    /// The data type of the animated property.
    type ValueType: Copy + Send + Sync + PartialEq + 'static + FallibleInterpolate;

    /// The type of component that contains the animated property.
    type ComponentType: Component<Mutability = Mutable>;

    /// Read the value of the animatable property.
    fn get(component: &Self::ComponentType) -> Self::ValueType;

    /// Update the value of the animatable property.
    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType);
}

/// Controls the direction of playback for a transition.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum PlaybackDirection {
    /// Playback is paused
    #[default]
    Paused,

    /// Playback is going forward
    Forward,

    /// Playback is going backwards
    Reverse,
}

/// A type alias for [`EntityMutExcept`] as used in animation.
pub type TransitionEntityMut<'w, 's> = EntityMutExcept<'w, 's, AnimatedTransitionSet>;

/// A map containing an adapter function for each animated transition component on an entity,
/// indexed by [`ComponentId`]. This allows the animations to be driven in a type-erased
/// way without trait queries. The map is built automatically by component hooks.
#[derive(Component, Default)]
pub struct AnimatedTransitionSet(
    pub HashMap<ComponentId, Box<dyn Fn(&mut TransitionEntityMut, &Time) + Send + Sync + 'static>>,
);

impl AnimatedTransitionSet {
    /// Animate all registered transitions.
    pub fn animate(&self, entity: &mut TransitionEntityMut, time: &Time) {
        for (_, transition) in self.0.iter() {
            (transition)(entity, time);
        }
    }
}

/// A component that describes an animated transition. This includes the clock, easing function,
/// and adapter for updating the component containing the property to be animated.
///
/// Transitions can be played forward, backward, or paused; and the direction of playback can
/// be changed at any time.
///
/// It is also possible to modify the animation target value in mid-animation, but care must
/// be taken to avoid jumps. The recommended approach is to reset the animation's clock to zero,
/// and change the start value to the current value of the animated property.
#[derive(Clone, Debug)]
pub struct AnimatedTransition<P: TransitionProperty> {
    /// The property we are targeting.
    prop: PhantomData<P>,

    /// The current playback direction
    direction: PlaybackDirection,

    /// The value of the property at the start of the transition
    start: P::ValueType,

    /// The value of the property at the end of the transition.
    end: P::ValueType,

    /// Timer that goes from 0 to 1 during the animation.
    clock: f32,

    /// Rate at which the timer advances.
    speed: f32,

    /// The easing function for this transition.
    ease: EaseFunction,
}

impl<P: TransitionProperty + Sync + Send + 'static> Component for AnimatedTransition<P> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    type Mutability = Mutable;

    /// Add this component to the [`AnimatedTransitionSet`].
    fn on_insert() -> Option<bevy_ecs::lifecycle::ComponentHook> {
        Some(|mut world: DeferredWorld, context: HookContext| {
            if let Some(mut transitions) = world.get_mut::<AnimatedTransitionSet>(context.entity) {
                transitions.0.insert(
                    context.component_id,
                    Box::new(|entity, time| {
                        let Some(mut transition) = entity.get_mut::<AnimatedTransition<P>>() else {
                            return;
                        };
                        let value = transition.advance(time);
                        if let Some(mut target) = entity.get_mut::<P::ComponentType>() {
                            P::set(&mut target, value);
                        }
                    }),
                );
            }
        })
    }

    /// Remove this component from the [`AnimatedTransitionSet`].
    fn on_remove() -> Option<bevy_ecs::lifecycle::ComponentHook> {
        Some(|mut world: DeferredWorld, context: HookContext| {
            if let Some(mut transitions) = world.get_mut::<AnimatedTransitionSet>(context.entity) {
                transitions.0.remove(&context.component_id);
            }
        })
    }

    fn register_required_components(
        _component_id: ComponentId,
        required_components: &mut bevy_ecs::component::RequiredComponentsRegistrator,
    ) {
        required_components.register_required(AnimatedTransitionSet::default);
    }
}

impl<P: TransitionProperty> AnimatedTransition<P> {
    /// Construct a new transition that goes from `start` to `end`.
    pub fn new(start: P::ValueType, end: P::ValueType) -> Self {
        Self {
            prop: PhantomData::<P>,
            direction: PlaybackDirection::Paused,
            start,
            end,
            clock: 0.,
            speed: 0.,
            ease: EaseFunction::Linear,
        }
    }

    /// Set the ease function of the transition.
    pub fn with_ease(self, ease: EaseFunction) -> Self {
        Self { ease, ..self }
    }

    /// Set the duration of the transition in seconds.
    pub fn with_duration(self, duration_seconds: f32) -> Self {
        let speed = if duration_seconds > 0.0 {
            duration_seconds.recip()
        } else {
            f32::INFINITY // Instant transition
        };

        Self { speed, ..self }
    }

    /// Start the transition in the forward direction.
    pub fn start(&mut self) -> &mut Self {
        self.direction = PlaybackDirection::Forward;
        self
    }

    /// Start the transition in the reverse direction.
    pub fn reverse(&mut self) -> &mut Self {
        self.direction = PlaybackDirection::Reverse;
        self
    }

    /// Pause the transition at its current position.
    pub fn pause(&mut self) -> &mut Self {
        self.direction = PlaybackDirection::Paused;
        self
    }

    /// Set the animation progress to a specific point (0.0 to 1.0).
    pub fn seek(&mut self, position: f32) -> &mut Self {
        self.clock = position.clamp(0.0, 1.0);
        self
    }

    /// Get the current progress of the transition (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        self.clock
    }

    /// Check if the transition is currently playing (forward or reverse).
    pub fn is_playing(&self) -> bool {
        self.direction != PlaybackDirection::Paused
    }

    /// Update the interpolation target values
    pub fn set_values(&mut self, start: P::ValueType, end: P::ValueType) -> &mut Self {
        self.start = start;
        self.end = end;
        self
    }
}

impl<P: TransitionProperty> AnimatedTransition<P> {
    /// Increment the clock based on the current playback state. Returns the current animatee
    /// value.
    fn advance(&mut self, time: &Time) -> P::ValueType {
        let speed = match self.direction {
            PlaybackDirection::Paused => 0.,
            PlaybackDirection::Forward => self.speed,
            PlaybackDirection::Reverse => -self.speed,
        };
        self.clock = (self.clock + speed * time.delta_secs()).clamp(0.0, 1.0);
        if (self.clock >= 1.0 && self.direction == PlaybackDirection::Forward)
            || (self.clock <= 0.0 && self.direction == PlaybackDirection::Reverse)
        {
            self.direction = PlaybackDirection::Paused;
        }
        let t = self.ease.sample_clamped(self.clock);
        match self.start.try_interpolate(&self.end, t) {
            Ok(value) => value,
            Err(_) => self.end,
        }
    }
}

/// ECS system which drives all animated transitions.
pub(crate) fn animate_transitions(
    world: &mut World,
    q_transitions: &mut QueryState<(TransitionEntityMut, &AnimatedTransitionSet)>,
) {
    let Some(time) = world.get_resource::<Time>() else {
        return;
    };
    let time = *time;

    // For all entities which have an `AnimatedTransitionSet`
    for (mut entity, transition_set) in q_transitions.iter_mut(world) {
        transition_set.animate(&mut entity, &time);
    }
}

/// Animated transition for background color
pub struct BackgroundColorTransition;

impl TransitionProperty for BackgroundColorTransition {
    type ValueType = Srgba;
    type ComponentType = BackgroundColor;

    fn get(component: &Self::ComponentType) -> Self::ValueType {
        component.0.into()
    }

    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.0 = value.into();
    }
}

/// Animated transition for border color. This sets all border sides to the same color.
pub struct BorderColorTransition;

impl TransitionProperty for BorderColorTransition {
    type ValueType = Srgba;
    type ComponentType = BorderColor;

    fn get(component: &Self::ComponentType) -> Self::ValueType {
        // For now, assume all sides the same.
        component.top.into()
    }

    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.set_all(value);
    }
}

/// Animated transition for [`Node::left`]
pub struct LeftTransition;

impl TransitionProperty for LeftTransition {
    type ValueType = Val;
    type ComponentType = Node;

    fn get(component: &Self::ComponentType) -> Self::ValueType {
        component.left
    }

    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.left = value;
    }
}

/// Animated transition for [`UiTransform::rotation`]
pub struct UiRotateTransition;

impl TransitionProperty for UiRotateTransition {
    type ValueType = Rot2;
    type ComponentType = UiTransform;

    fn get(component: &Self::ComponentType) -> Self::ValueType {
        component.rotation
    }

    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.rotation = value;
    }
}

/// Animated transition for [`UiTransform::scale`]
pub struct UiScaleTransition;

impl TransitionProperty for UiScaleTransition {
    type ValueType = Vec2;
    type ComponentType = UiTransform;

    fn get(component: &Self::ComponentType) -> Self::ValueType {
        component.scale
    }

    fn set(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.scale = value;
    }
}

/// Plugin which registers the animation driver system.
pub struct AnimatedTransitionPlugin;

impl Plugin for AnimatedTransitionPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, animate_transitions.in_set(UiSystems::Prepare));
    }
}
