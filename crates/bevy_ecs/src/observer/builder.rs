use std::marker::PhantomData;

use super::*;

/// Builder struct for [`Observer`].
pub struct ObserverBuilder<'w, E: EcsEvent = NoEvent> {
    world: &'w mut World,
    descriptor: ObserverDescriptor,
    _marker: PhantomData<E>,
}

impl<'w, E: EcsEvent> ObserverBuilder<'w, E> {
    /// Constructs a new [`ObserverBuilder`].
    pub fn new(world: &'w mut World) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        let event = world.init_component::<E>();
        if event != NO_EVENT {
            descriptor.events.push(event);
        }
        Self {
            world,
            descriptor,
            _marker: PhantomData::default(),
        }
    }

    /// Adds `NewE` to the list of events listened to by this observer.
    /// Observers that listen to multiple types of events can no longer access the typed event data.
    pub fn on_event<NewE: EcsEvent>(&mut self) -> &mut ObserverBuilder<'w, NoEvent> {
        let event = self.world.init_component::<NewE>();
        self.descriptor.events.push(event);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add `events` to the list of events listened to by this observer.
    /// Observers that listen to multiple types of events can no longer access the typed event data.
    pub fn on_event_ids(
        &mut self,
        events: impl IntoIterator<Item = ComponentId>,
    ) -> &mut ObserverBuilder<'w, NoEvent> {
        self.descriptor.events.extend(events);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add [`ComponentId`] in `T` to the list of components listened to by this observer.
    pub fn components<T: Bundle>(&mut self) -> &mut Self {
        T::component_ids(
            &mut self.world.components,
            &mut self.world.storages,
            &mut |id| self.descriptor.components.push(id),
        );
        self
    }

    /// Add `ids` to the list of component sources listened to by this observer.
    pub fn component_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        self.descriptor.components.extend(ids);
        self
    }

    /// Adds `source` as the list of entity sources listened to by this observer.
    pub fn source(&mut self, source: Entity) -> &mut Self {
        self.descriptor.sources.push(source);
        self
    }

    /// Spawns the resulting observer into the world.
    pub fn run<Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        &mut self,
        callback: impl ObserverCallback<E, Q, F> + 'static,
    ) -> Entity {
        let entity = self.enqueue(callback);
        self.world.flush_commands();
        entity
    }

    /// Spawns the resulting observer into the world using a [`ObserverRunner`] callback.
    /// This is not advised unless you want to respond to events that may not be associated with an entity
    /// or otherwise want to override the default runner behaviour.
    pub fn runner<Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        &mut self,
        runner: ObserverRunner,
    ) -> Entity {
        let entity = self.enqueue_runner::<Q, F>(runner);
        self.world.flush_commands();
        entity
    }

    /// Enqueues a command to spawn the resulting observer in the world.
    pub fn enqueue<Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        &mut self,
        callback: impl ObserverCallback<E, Q, F> + 'static,
    ) -> Entity {
        self.world
            .spawn_observer::<E, Q, F>(ObserverComponent::from(self.descriptor.clone(), callback))
    }

    /// Enqueues a command to spawn the resulting observer in the world using a [`ObserverRunner`] callback.
    /// This is not advised unless you want to respond to events that may not be associated with an entity
    /// or otherwise want to override the default runner behaviour.
    pub fn enqueue_runner<Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        &mut self,
        runner: ObserverRunner,
    ) -> Entity {
        self.world
            .spawn_observer::<E, Q, F>(ObserverComponent::from_runner(
                self.descriptor.clone(),
                runner,
            ))
    }
}
