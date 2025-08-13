use core::marker::PhantomData;

use bevy_app::{Plugin, PostUpdate};
use bevy_color::Srgba;
use bevy_ecs::component::ComponentId;
use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::query::QueryState;
use bevy_ecs::schedule::IntoScheduleConfigs as _;
use bevy_ecs::world::{DeferredWorld, EntityMut, World};
use bevy_ecs::{
    component::{Component, Mutable},
    world::Mut,
};
use bevy_math::{curve::EaseFunction, Curve, StableInterpolate};
use bevy_time::Time;
use bevy_ui::{BackgroundColor, Node, UiSystems, Val};
use std::collections::HashSet;

/// Represents an animatable property such as `BackgroundColor` or `Width`.
pub trait TransitionProperty {
    /// The data type of the animated property.
    type ValueType: Copy + Send + Sync + PartialEq + 'static + StableInterpolate;

    type ComponentType: Component<Mutability = Mutable>;

    /// Update the value of the animatable property.
    fn update(component: &mut Mut<Self::ComponentType>, value: Self::ValueType);
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

/// Trait that allows type-erased access to the animated transition component.
pub trait AnyAnimatedTransition {
    /// Animate the property controlled by this transition.
    fn animate(&mut self, entity: &mut EntityMut, time: &Time);
}

/// A collection of component IDs for tracking transitions
#[derive(Component, Default, Clone, Debug)]
pub struct AnimatedTransitionSet(pub HashSet<ComponentId>);

/// A component that describes an animated transition. This includes the clock, easing function,
/// and adapter for updating the component containing the property to be animated.
///
/// Transitions can be played forward, backward, or paused; and the direction of playback can
/// be changed at any time.
///
/// It is also possible to modify the animation target value in mid-animation, but care must
/// be taken to avoid jumps. The recommended approach is to reset the animation's clock to zero,
/// and change the start value to the current value of the animated property.
#[derive(Component, Clone, Debug)]
#[require(AnimatedTransitionSet)]
#[component(on_insert = on_add_animation, on_remove = on_remove_animation)]
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

impl<P: TransitionProperty> AnyAnimatedTransition for AnimatedTransition<P> {
    fn animate(&mut self, entity: &mut EntityMut, time: &Time) {
        if let Some(mut component) = entity.get_mut::<P::ComponentType>() {
            self.update(&mut component, *time);
        }
    }
}

/// Hook function to update the set of component ids.
fn on_add_animation(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut transitions) = world.get_mut::<AnimatedTransitionSet>(context.entity) {
        transitions.0.insert(context.component_id);
    }
}

fn on_remove_animation(mut world: DeferredWorld, context: HookContext) {
    if let Some(mut transitions) = world.get_mut::<AnimatedTransitionSet>(context.entity) {
        transitions.0.remove(&context.component_id);
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
    fn update(&mut self, component: &mut Mut<P::ComponentType>, time: Time) {
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
        P::update(component, self.start.interpolate_stable(&self.end, t));
    }
}

/// ECS system which drives all animated transitions.
pub(crate) fn animate_transitions(
    world: &mut World,
    q_transitions: &mut QueryState<(Entity, &AnimatedTransitionSet)>,
) {
    let Some(time) = world.get_resource::<Time>() else {
        return;
    };

    for (entity_id, transition_set) in q_transitions.iter(world) {
        for component_id in transition_set.0.iter() {
            let mut entity = world.entity_mut(entity_id);
            if let Ok(target_component) = entity.get_mut_by_id(*component_id) {
                // Yes, I know there's no such nethod.
                let Some(mut any_transition) =
                    target_component.downcast_unsafe_mut::<dyn AnyAnimatedTransition>()
                else {
                    continue;
                };

                any_transition.animate(&mut entity, &time);
            }
        }
    }
}

/// Animated transition for background color
pub struct BackgroundColorTransition;

impl TransitionProperty for BackgroundColorTransition {
    type ValueType = Srgba;
    type ComponentType = BackgroundColor;

    fn update(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.0 = value.into();
    }
}

/// Animated transition for [`Node::left`]
pub struct LeftPercentTransition;

impl TransitionProperty for LeftPercentTransition {
    type ValueType = f32;
    type ComponentType = Node;

    fn update(component: &mut Mut<Self::ComponentType>, value: Self::ValueType) {
        component.left = Val::Percent(value);
    }
}

pub struct AnimatedTransitionPlugin;

impl Plugin for AnimatedTransitionPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.add_systems(PostUpdate, animate_transitions.in_set(UiSystems::Prepare));
    }
}
