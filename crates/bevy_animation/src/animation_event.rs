pub use bevy_animation_macros::AnimationEvent;

use bevy_ecs::{
    entity::Entity,
    event::{trigger_entity_internal, Event, Trigger},
    observer::{CachedObservers, TriggerContext},
    world::DeferredWorld,
};

/// An [`Event`] that an [`AnimationPlayer`](crate::AnimationPlayer) can trigger when playing an [`AnimationClip`](crate::AnimationClip).
/// See [`AnimationClip::add_event`](crate::AnimationClip::add_event).
///
/// This trait can be derived.
pub trait AnimationEvent: Clone + for<'a> Event<Trigger<'a> = AnimationEventTrigger> {}

/// The [`Trigger`] implementation for [`AnimationEvent`]. This passes in the [`AnimationPlayer`](crate::AnimationPlayer)
/// context, and uses that to run any observers that target that entity.
pub struct AnimationEventTrigger {
    /// The [`AnimationPlayer`](crate::AnimationPlayer) where this [`AnimationEvent`] occurred.
    pub animation_player: Entity,
}

#[expect(
    unsafe_code,
    reason = "We must implement this trait to define a custom Trigger, which is required to be unsafe due to safety considerations within bevy_ecs."
)]
// SAFETY:
// - `E`'s [`Event::Trigger`] is constrained to [`AnimationEventTrigger`]
// - The implementation abides by the other safety constraints defined in [`Trigger`]
unsafe impl<E: AnimationEvent + for<'a> Event<Trigger<'a> = AnimationEventTrigger>> Trigger<E>
    for AnimationEventTrigger
{
    unsafe fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let animation_player = self.animation_player;
        // SAFETY:
        // - `observers` come from `world` and match the event type `E`, enforced by the call to `trigger`
        // - the passed in event pointer comes from `event`, which is an `Event`
        // - `trigger` is a matching trigger type, as it comes from `self`, which is the Trigger for `E`
        // - `trigger_context`'s event_key matches `E`, enforced by the call to `trigger`
        // - this abides by the nuances defined in the `Trigger` safety docs
        unsafe {
            trigger_entity_internal(
                world,
                observers,
                event.into(),
                self.into(),
                animation_player,
                trigger_context,
            );
        }
    }
}
