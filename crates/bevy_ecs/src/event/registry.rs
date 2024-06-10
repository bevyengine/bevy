use crate as bevy_ecs;
use bevy_ecs::{
    change_detection::{DetectChangesMut, MutUntyped},
    component::{ComponentId, Tick},
    event::{Event, Events},
    system::Resource,
    world::World,
};

#[doc(hidden)]
struct RegisteredEvent {
    component_id: ComponentId,
    // Required to flush the secondary buffer and drop events even if left unchanged.
    previously_updated: bool,
    // SAFETY: The component ID and the function must be used to fetch the Events<T> resource
    // of the same type initialized in `register_event`, or improper type casts will occur.
    update: unsafe fn(MutUntyped),
}

/// A registry of all of the [`Events`] in the [`World`], used by [`event_update_system`]
/// to update all of the events.
#[derive(Resource, Default)]
pub struct EventRegistry {
    pub(super) needs_update: bool,
    event_updates: Vec<RegisteredEvent>,
}

impl EventRegistry {
    /// Registers an event type to be updated.
    pub fn register_event<T: Event>(world: &mut World) {
        // By initializing the resource here, we can be sure that it is present,
        // and receive the correct, up-to-date `ComponentId` even if it was previously removed.
        let component_id = world.init_resource::<Events<T>>();
        let mut registry = world.get_resource_or_insert_with(Self::default);
        registry.event_updates.push(RegisteredEvent {
            component_id,
            previously_updated: false,
            update: |ptr| {
                // SAFETY: The resource was initialized with the type Events<T>.
                unsafe { ptr.with_type::<Events<T>>() }
                    .bypass_change_detection()
                    .update();
            },
        });
    }

    /// Updates all of the registered events in the World.
    pub fn run_updates(&mut self, world: &mut World, last_change_tick: Tick) {
        for registered_event in &mut self.event_updates {
            // Bypass the type ID -> Component ID lookup with the cached component ID.
            if let Some(events) = world.get_resource_mut_by_id(registered_event.component_id) {
                let has_changed = events.has_changed_since(last_change_tick);
                if registered_event.previously_updated || has_changed {
                    // SAFETY: The update function pointer is called with the resource
                    // fetched from the same component ID.
                    unsafe { (registered_event.update)(events) };
                    // Always set to true if the events have changed, otherwise disable running on the second invocation
                    // to wait for more changes.
                    registered_event.previously_updated =
                        has_changed || !registered_event.previously_updated;
                }
            }
        }
    }
}
