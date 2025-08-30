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

impl<E: AnimationEvent> Trigger<E> for AnimationEventTrigger {
    fn trigger(
        &mut self,
        world: DeferredWorld,
        observers: &CachedObservers,
        trigger_context: &TriggerContext,
        event: &mut E,
    ) {
        let animation_player = self.animation_player;
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
