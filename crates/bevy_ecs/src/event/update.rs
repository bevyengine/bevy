use crate as bevy_ecs;
use bevy_ecs::{
    change_detection::Mut,
    component::Tick,
    event::EventRegistry,
    system::{Local, Res, ResMut},
    world::World,
};
use bevy_ecs_macros::SystemSet;
#[cfg(feature = "bevy_reflect")]
use std::hash::Hash;

use super::registry::ShouldUpdateEvents;

#[doc(hidden)]
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventUpdates;

/// Signals the [`event_update_system`] to run after `FixedUpdate` systems.
///
/// This will change the behavior of the [`EventRegistry`] to only run after a fixed update cycle has passed.
/// Normally, this will simply run every frame.
pub fn signal_event_update_system(signal: Option<ResMut<EventRegistry>>) {
    if let Some(mut registry) = signal {
        registry.should_update = ShouldUpdateEvents::Ready;
    }
}

/// A system that calls [`Events::update`] on all registered [`Events`] in the world.
pub fn event_update_system(world: &mut World, mut last_change_tick: Local<Tick>) {
    if world.contains_resource::<EventRegistry>() {
        world.resource_scope(|world, mut registry: Mut<EventRegistry>| {
            registry.run_updates(world, *last_change_tick);

            registry.should_update = match registry.should_update {
                // If we're always updating, keep doing so.
                ShouldUpdateEvents::Always => ShouldUpdateEvents::Always,
                // Disable the system until signal_event_update_system runs again.
                ShouldUpdateEvents::Waiting | ShouldUpdateEvents::Ready => {
                    ShouldUpdateEvents::Waiting
                }
            };
        });
    }
    *last_change_tick = world.change_tick();
}

/// A run condition for [`event_update_system`].
///
/// If [`signal_event_update_system`] has been run at least once,
/// we will wait for it to be run again before updating the events.
///
/// Otherwise, we will always update the events.
pub fn event_update_condition(maybe_signal: Option<Res<EventRegistry>>) -> bool {
    match maybe_signal {
        Some(signal) => match signal.should_update {
            ShouldUpdateEvents::Always | ShouldUpdateEvents::Ready => true,
            ShouldUpdateEvents::Waiting => false,
        },
        None => true,
    }
}
