//! Types for creating and storing [`Signal`]s

use crate::{
    component::{ComponentHooks, StorageType},
    prelude::{Component, Entity, Observer, Query, Trigger, World},
    system::Commands,
    world::OnMutate,
};
use bevy_reflect::Reflect;
use std::{marker::PhantomData, sync::Arc};

impl World {
    /// Create a new [`Signal`] with an initial value.
    pub fn spawn_signal<S: Component>(&mut self, initial_value: S) -> Signal<S> {
        let refcount = SignalRefcount::default();
        let entity = self
            .spawn((initial_value, SignalMarker, refcount.clone()))
            .id();
        Signal {
            entity,
            refcount,
            phantom_data: PhantomData,
        }
    }
}

/// A wrapper around a value that entities can subscribe to.
///
/// TODO: Example
pub struct Signal<S: Component> {
    entity: Entity,
    refcount: SignalRefcount, // TODO: Signal can't be copy because of this
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
            signal: self.clone(),
        }
    }

    /// Set this signal's value, updating all entities with subscribed components.
    pub fn set(&self, value: S, query: &mut Query<&mut S>) {
        *query
            .get_mut(self.entity)
            .expect("Signal component was removed") = value;
    }
}

impl<S: Component> Clone for Signal<S> {
    fn clone(&self) -> Self {
        Self {
            entity: self.entity,
            refcount: self.refcount.clone(),
            phantom_data: PhantomData,
        }
    }
}

/// An intermediate component for setting up a subscribed component for an entity.
///
/// Adding this component to an entity will initiate the setup for that entity, and then remove this component from it.
pub struct SubscribedComponentSetup<S: Component, C: Component> {
    f: Box<dyn (Fn(&S) -> C) + 'static + Send + Sync>,
    signal: Signal<S>,
}

impl<S: Component, C: Component> Component for SubscribedComponentSetup<S, C> {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;

    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(|mut world, subscriber_entity, _| {
            world.commands().queue(move |world: &mut World| {
                subscribed_component_setup::<S, C>(subscriber_entity, world);
            })
        });
    }
}

fn subscribed_component_setup<S: Component, C: Component>(
    subscriber_entity: Entity,
    world: &mut World,
) {
    // Get and remove the SubscribedComponentSetup component we just added to the subscriber entity
    let setup_info = world
        .entity_mut(subscriber_entity)
        .take::<SubscribedComponentSetup<S, C>>()
        .unwrap();

    // Get the signal entity
    let signal_entity = world
        .get_entity(setup_info.signal.entity)
        .expect("Tried to subscribe an entity's component to a signal that was despawned.");

    // Get the value the signal entity is holding
    let signal_value = signal_entity.get::<S>().expect(
        "Tried to subscribe an entity's component to a signal that had its component removed.",
    );

    // Call the subscription function to produce the initial subscribed component value
    let signaled_component_initial_value = (setup_info.f)(signal_value);

    // Add the initial subscription value to the subscriber entity
    world
        .entity_mut(subscriber_entity)
        .insert(signaled_component_initial_value);

    // Setup an ongoing subscription to update the subscriber entity's component value whenever the signal value changes
    let f = setup_info.f;
    let subscription = move |signal_trigger: Trigger<OnMutate, S>,
                             signal_query: Query<&S>,
                             mut subscriber_query: Query<&mut C>| {
        // Get the current value of the signal
        let signal_value = signal_query
            .get(signal_trigger.entity())
            .expect("Signal component was removed");

        // Call the subscription function to produce the subscribed component value
        let signaled_component_value = (f)(signal_value);

        // Update the subscribed entity's component value
        *subscriber_query
            .get_mut(subscriber_entity)
            .expect("Subscribed component was removed") = signaled_component_value;
    };
    world.spawn((
        SignalSubscriptionMarker,
        setup_info.signal.refcount,
        Observer::new(subscription).with_entity(setup_info.signal.entity),
    ));
}

/// Marker component for an entity backing a [`Signal`].
#[derive(Reflect)]
pub struct SignalMarker;
impl Component for SignalMarker {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}

/// Marker component for an entity backing a [`Signal`] subscription.
#[derive(Reflect)]
pub struct SignalSubscriptionMarker;
impl Component for SignalSubscriptionMarker {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}

/// Refcount for tracking the lifetime of a [`Signal`].
#[derive(Reflect, Clone, Default)]
#[reflect(opaque)]
pub struct SignalRefcount(Arc<()>);
impl Component for SignalRefcount {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}

/// System to automatically clean up unused [`Signal`]s.
pub fn signal_cleanup(signals: Query<(Entity, &SignalRefcount)>, mut commands: Commands) {
    for (signal, refcount) in &signals {
        if Arc::strong_count(&refcount.0) == 1 {
            commands.entity(signal).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::prelude::*;
    use crate::system::RunSystemOnce;

    #[derive(Component)]
    struct Foo(u32);

    #[derive(Component, PartialEq, Eq, Debug)]
    struct Bar(String);

    #[test]
    fn test_signal_subscription() {
        let mut world = World::new();
        let signal = world.spawn_signal(Foo(0));

        let e = world.spawn(signal.subscribe(|s| Bar(s.0.to_string()))).id();
        world.flush();

        assert_eq!(world.entity(e).get::<Bar>(), Some(&Bar("0".to_string())));

        world
            .run_system_once(move |mut sq: Query<&mut Foo>| signal.set(Foo(17), &mut sq))
            .unwrap();
        world.flush();

        assert_eq!(world.entity(e).get::<Bar>(), Some(&Bar("17".to_string())));
    }
}
