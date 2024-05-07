use std::any::TypeId;

use super::*;

/// Builder struct for [`Observer`].
pub struct ObserverBuilder<'w, E = ()> {
    commands: Commands<'w, 'w>,
    descriptor: ObserverDescriptor,
    _marker: PhantomData<E>,
}

impl<'w, E: 'static> ObserverBuilder<'w, E> {
    /// Constructs a new [`ObserverBuilder`].
    pub fn new(commands: Commands<'w, 'w>) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        let event = commands
            .components()
            .get_id(TypeId::of::<E>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot observe event before it is registered with init_component: {}",
                    std::any::type_name::<E>(),
                )
            });
        descriptor.events.push(event);

        Self {
            commands,
            descriptor,
            _marker: PhantomData,
        }
    }

    /// Constructs an [`ObserverBuilder`] with a dynamic event id.
    /// # Safety
    /// Caller must ensure that the component associated with `id` is accessible as E
    #[must_use]
    pub unsafe fn new_with_id(event: ComponentId, commands: Commands<'w, 'w>) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        descriptor.events.push(event);

        Self {
            commands,
            descriptor,
            _marker: PhantomData,
        }
    }

    /// Adds `NewE` to the list of events listened to by this observer.
    /// After calling this function the observer will no longer have access to typed event data
    /// to prevent accessing the data as the incorrect type.
    /// Observing the same event multiple times has no effect.
    pub fn on_event<NewE: 'static>(&mut self) -> &mut ObserverBuilder<'w, ()> {
        let event = self
            .commands
            .components()
            .get_id(TypeId::of::<NewE>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot observe event before it is registered with init_component: {}",
                    std::any::type_name::<NewE>(),
                )
            });
        self.descriptor.events.push(event);
        // SAFETY: () will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add `events` to the list of events listened to by this observer.
    /// After calling this function the observer will no longer have access to typed event data
    /// to prevent accessing the data as the incorrect type.
    /// Observing the same event multiple times has no effect.
    pub fn on_event_ids(
        &mut self,
        events: impl IntoIterator<Item = ComponentId>,
    ) -> &mut ObserverBuilder<'w, ()> {
        self.descriptor.events.extend(events);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add [`ComponentId`] in `T` to the list of components listened to by this observer.
    /// For examples an `OnRemove` observer would trigger when any component in `B` was removed.
    pub fn components<B: Bundle>(&mut self) -> &mut Self {
        B::get_component_ids(self.commands.components(), &mut |id| {
            self.descriptor.components.push(id.unwrap_or_else(|| {
                panic!(
                    "Cannot observe event before it is registered with init_component: {}",
                    std::any::type_name::<B>(),
                )
            }));
        });
        self
    }

    /// Add `ids` to the list of component sources listened to by this observer.
    pub fn component_ids<'c>(
        &mut self,
        ids: impl IntoIterator<Item = &'c ComponentId>,
    ) -> &mut Self {
        self.descriptor.components.extend(ids.into_iter().cloned());
        self
    }

    /// Adds `source` as the list of entity sources listened to by this observer.
    pub fn source(&mut self, source: Entity) -> &mut Self {
        self.descriptor.sources.push(source);
        self
    }

    /// Spawns the resulting observer into the world.
    pub fn run<B: Bundle, M>(&mut self, system: impl IntoObserverSystem<E, B, M>) -> Entity {
        B::get_component_ids(self.commands.components(), &mut |id| {
            self.descriptor.components.push(id.unwrap_or_else(|| {
                panic!(
                    "Cannot observe event before it is registered with init_component: {}",
                    std::any::type_name::<B>(),
                )
            }));
        });
        let entity = self.commands.spawn_empty().id();
        let descriptor = self.descriptor.clone();
        self.commands.add(move |world: &mut World| {
            let component = ObserverComponent::from(world, descriptor, system);
            world.entity_mut(entity).insert(component);
            world.register_observer(entity);
        });
        entity
    }

    /// Spawns the resulting observer into the world using an [`ObserverRunner`] callback.
    /// This is not advised unless you want to override the default runner behaviour.
    pub fn runner(&mut self, runner: ObserverRunner) -> Entity {
        let entity = self.commands.spawn_empty().id();
        let descriptor = self.descriptor.clone();
        self.commands.add(move |world: &mut World| {
            let component = ObserverComponent::from_runner(descriptor, runner);
            world.entity_mut(entity).insert(component);
            world.register_observer(entity);
        });
        entity
    }
}

/// Type used to construct and emit an ECS event.
pub struct EventBuilder<'w, E = ()> {
    event: ComponentId,
    commands: Commands<'w, 'w>,
    targets: Vec<Entity>,
    components: Vec<ComponentId>,
    data: Option<E>,
}

impl<'w, E: Send + 'static> EventBuilder<'w, E> {
    /// Constructs a new builder that will write it's event to `world`'s command queue
    #[must_use]
    pub fn new(data: E, commands: Commands<'w, 'w>) -> Self {
        let event = commands
            .components()
            .get_id(TypeId::of::<E>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot emit event before it is registered with init_component: {}",
                    std::any::type_name::<E>()
                )
            });
        Self {
            event,
            commands,
            targets: Vec::new(),
            components: Vec::new(),
            data: Some(data),
        }
    }

    /// Sets the event id of the resulting event, used for dynamic events
    /// # Safety
    /// Caller must ensure that the component associated with `id` is accessible as E
    #[must_use]
    pub unsafe fn new_with_id(event: ComponentId, data: E, commands: Commands<'w, 'w>) -> Self {
        Self {
            event,
            commands,
            targets: Vec::new(),
            components: Vec::new(),
            data: Some(data),
        }
    }

    /// Adds `target` to the list of entities targeted by `self`
    #[must_use]
    pub fn entity(&mut self, target: Entity) -> &mut Self {
        self.targets.push(target);
        self
    }

    /// Adds `component_id` to the list of components targeted by `self`
    #[must_use]
    pub fn component(&mut self, component_id: ComponentId) -> &mut Self {
        self.components.push(component_id);
        self
    }

    /// Add the event to the command queue of world
    pub fn emit(&mut self) {
        // SAFETY: `self.event` is accessible as E, enforced in `Self::new` and `Self::new_with_id`.
        self.commands.add(unsafe {
            EmitEcsEvent::<E>::new(
                self.event,
                std::mem::take(&mut self.targets),
                std::mem::take(&mut self.components),
                std::mem::take(&mut self.data)
                    .expect("EventBuilder used to send more than one event."),
            )
        });
    }
}

impl<'w, 's> Commands<'w, 's> {
    /// Constructs an [`EventBuilder`].
    #[must_use]
    pub fn event<E: Component>(&mut self, event: E) -> EventBuilder<E> {
        EventBuilder::new(event, self.reborrow())
    }

    /// Construct an [`ObserverBuilder`].
    #[must_use]
    pub fn observer_builder<E: Component>(&mut self) -> ObserverBuilder<E> {
        ObserverBuilder::new(self.reborrow())
    }

    /// Spawn an [`Observer`] and returns it's [`Entity`].
    pub fn observer<E: Component, B: Bundle, M>(
        &mut self,
        callback: impl IntoObserverSystem<E, B, M>,
    ) -> Entity {
        ObserverBuilder::new(self.reborrow()).run(callback)
    }
}
