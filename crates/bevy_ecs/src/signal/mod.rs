//! Types for creating and storing [`Signal`]s

use crate::{
    component::{ComponentHooks, StorageType},
    prelude::{Component, Entity, Observer, Query, Trigger, World},
    world::OnMutate,
};
use std::marker::PhantomData;

impl World {
    /// Create a new [`Signal`] with an initial value.
    pub fn spawn_signal<S: Component>(&mut self, initial_value: S) -> Signal<S> {
        let entity = self.spawn(initial_value).id();
        Signal {
            entity,
            phantom_data: PhantomData,
        }
    }
}

/// A wrapper around a value that entities can subscribe to.
///
/// TODO: Example
#[derive(Clone, Copy)]
pub struct Signal<S: Component> {
    entity: Entity,
    phantom_data: PhantomData<S>,
}

impl<S: Component> Signal<S> {
    // TODO: Have `f` return `impl Into<Option<C>>` to allow conditional subscribed components, removing when None
    /// Create a subscription that takes this signal's value and produces a component value of type `C`.
    ///
    /// The returned setup component should be inserted on the entity you intend to have the component of type
    /// `C` that is subscribed to this signal.
    pub fn subscribe<C: Component>(
        &self,
        f: impl (Fn(&S) -> C) + 'static + Send + Sync,
    ) -> SubscribedComponentSetup<S, C> {
        SubscribedComponentSetup {
            f: Box::new(f),
            signal_entity: self.entity,
        }
    }

    /// Set this signal's value, updating all entities with subscribed components.
    pub fn set(&self, value: S, query: &mut Query<&mut S>) {
        *query
            .get_mut(self.entity)
            .expect("Signal component was removed") = value;
    }
}

/// An intermediate component for setting up a subscribed component for an entity.
///
/// Adding this component to an entity will initiate the setup for that entity, and then remove this component from it.
pub struct SubscribedComponentSetup<S: Component, C: Component> {
    f: Box<dyn (Fn(&S) -> C) + 'static + Send + Sync>,
    signal_entity: Entity,
}

impl<S: Component, C: Component> Component for SubscribedComponentSetup<S, C> {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, subscriber_entity, _| world.commands().queue(move |world: &mut World| {
            // Get and remove the SubscribedComponentSetup component we just added to the subscriber entity
            let setup_info = world.entity_mut(subscriber_entity).take::<Self>().unwrap();

            // Get the signal entity
            let signal_entity = world
                .get_entity(setup_info.signal_entity)
                .expect("Tried to subscribe an entity's component to a signal that was despawned.");

            // Get the value the signal entity is holding
            let signal_value = signal_entity.get::<S>().expect("Tried to subscribe an entity's component to a signal that had its component removed.");

            // Call the subscription function to produce the initial subscribed component value
            let signaled_component_initial_value = (setup_info.f)(signal_value);

            // Add the initial subscription value to the subscriber entity
            world.entity_mut(subscriber_entity).insert(signaled_component_initial_value);

            // Setup an ongoing subscription to update the subscriber entity's component value whenever the signal value changes
            let f = setup_info.f;
            world.spawn(Observer::new(move |signal_trigger: Trigger<OnMutate, S>, signal_query: Query<&S>, mut subscriber_query: Query<&mut C>| {
                // Get the current value of the signal
                let signal_value = signal_query.get(signal_trigger.entity()).expect("Signal component was removed");

                // Call the subscription function to produce the subscribed component value
                let signaled_component_value = (f)(signal_value);

                // Update the subscribed entity's component value
                *subscriber_query.get_mut(subscriber_entity).expect("Subscribed component was removed") = signaled_component_value;
            })
            .with_entity(setup_info.signal_entity));
        }));
    }
}

#[cfg(test)]
mod tests {}
