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

#[doc(hidden)]
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct EventUpdates;

/// Signals the [`event_update_system`] to run after `FixedUpdate` systems.
pub fn signal_event_update_system(signal: Option<ResMut<EventRegistry>>) {
    if let Some(mut registry) = signal {
        registry.needs_update = true;
    }
}

/// A system that calls [`Events::update`] on all registered [`Events`] in the world.
pub fn event_update_system(world: &mut World, mut last_change_tick: Local<Tick>) {
    println!("Running event_update_system");
    if world.contains_resource::<EventRegistry>() {
        world.resource_scope(|world, mut registry: Mut<EventRegistry>| {
            registry.run_updates(world, *last_change_tick);
            // Disable the system until signal_event_update_system runs again.
            registry.needs_update = false;
        });
    }
    *last_change_tick = world.change_tick();
}

/// A run condition for [`event_update_system`].
pub fn event_update_condition(signal: Option<Res<EventRegistry>>) -> bool {
    println!("Checking if we should run event_update_system");

    println!("signal: {:?}", signal);

    // If we haven't got a signal to update the events, but we *could* get such a signal
    // return early and update the events later.
    signal.map_or(false, |signal| signal.needs_update)
}
