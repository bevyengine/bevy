use core::marker::PhantomData;

use bevy_color::Srgba;
use bevy_ecs::{
    component::{Component, Mutable},
    system::{Query, Res},
    world::Mut,
};
use bevy_math::{curve::EaseFunction, Curve, StableInterpolate};
use bevy_time::Time;
use bevy_ui::{BackgroundColor, Node, Val};

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

#[derive(Component, Clone, Debug)]
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

pub(crate) fn animate_transition<P: TransitionProperty + Send + Sync + 'static>(
    mut q_transitions: Query<(&mut AnimatedTransition<P>, &mut P::ComponentType)>,
    time: Res<Time>,
) {
    for (mut transition, mut component) in q_transitions.iter_mut() {
        transition.update(&mut component, *time);
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
