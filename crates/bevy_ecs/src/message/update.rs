use crate::{
    change_detection::Mut,
    component::Tick,
    message::{MessageRegistry, ShouldUpdateMessages},
    system::{Local, Res, ResMut},
    world::World,
};
use bevy_ecs_macros::SystemSet;
#[cfg(feature = "bevy_reflect")]
use core::hash::Hash;

#[doc(hidden)]
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct MessageUpdateSystems;

/// Deprecated alias for [`MessageUpdateSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `MessageUpdateSystems`.")]
pub type EventUpdates = MessageUpdateSystems;

/// Signals the [`message_update_system`] to run after `FixedUpdate` systems.
///
/// This will change the behavior of the [`MessageRegistry`] to only run after a fixed update cycle has passed.
/// Normally, this will simply run every frame.
pub fn signal_message_update_system(signal: Option<ResMut<MessageRegistry>>) {
    if let Some(mut registry) = signal {
        registry.should_update = ShouldUpdateMessages::Ready;
    }
}

/// A system that calls [`Messages::update`](super::Messages::update) on all registered [`Messages`][super::Messages] in the world.
pub fn message_update_system(world: &mut World, mut last_change_tick: Local<Tick>) {
    world.try_resource_scope(|world, mut registry: Mut<MessageRegistry>| {
        registry.run_updates(world, *last_change_tick);

        registry.should_update = match registry.should_update {
            // If we're always updating, keep doing so.
            ShouldUpdateMessages::Always => ShouldUpdateMessages::Always,
            // Disable the system until signal_message_update_system runs again.
            ShouldUpdateMessages::Waiting | ShouldUpdateMessages::Ready => {
                ShouldUpdateMessages::Waiting
            }
        };
    });
    *last_change_tick = world.change_tick();
}

/// A run condition for [`message_update_system`].
///
/// If [`signal_message_update_system`] has been run at least once,
/// we will wait for it to be run again before updating the messages.
///
/// Otherwise, we will always update the messages.
pub fn message_update_condition(maybe_signal: Option<Res<MessageRegistry>>) -> bool {
    match maybe_signal {
        Some(signal) => match signal.should_update {
            ShouldUpdateMessages::Always | ShouldUpdateMessages::Ready => true,
            ShouldUpdateMessages::Waiting => false,
        },
        None => true,
    }
}
