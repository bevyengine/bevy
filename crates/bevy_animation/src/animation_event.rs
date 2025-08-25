pub use bevy_animation_macros::AnimationEvent;

use bevy_ecs::{
    entity::Entity,
    event::{trigger_entity_internal, Event, Trigger},
    observer::{CachedObservers, TriggerContext},
    world::DeferredWorld,
};

pub trait AnimationEvent: Clone + for<'a> Event<Trigger<'a> = AnimationEventTrigger> {}

pub struct AnimationEventTrigger {
    pub animation_player: Entity,
}

impl<E: Event> Trigger<E> for AnimationEventTrigger {
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
