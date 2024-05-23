use std::any::TypeId;

use super::*;

/// Builder struct for [`Observer`].
pub struct ObserverBuilder<'w, T = ()> {
    commands: Commands<'w, 'w>,
    descriptor: ObserverDescriptor,
    _marker: PhantomData<T>,
}

impl<'w, T: 'static> ObserverBuilder<'w, T> {
    /// Constructs a new [`ObserverBuilder`].
    pub fn new(commands: Commands<'w, 'w>) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        let trigger = commands
            .components()
            .get_id(TypeId::of::<T>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot observe trigger before it is registered with init_trigger: {}",
                    std::any::type_name::<T>(),
                )
            });
        descriptor.triggers.push(trigger);

        Self {
            commands,
            descriptor,
            _marker: PhantomData,
        }
    }

    /// Constructs an [`ObserverBuilder`] with a dynamic trigger id.
    ///
    /// # Safety
    /// Caller must ensure that the component associated with `id` is accessible as T
    #[must_use]
    pub unsafe fn new_with_id(trigger: ComponentId, commands: Commands<'w, 'w>) -> Self {
        let mut descriptor = ObserverDescriptor::default();
        descriptor.triggers.push(trigger);

        Self {
            commands,
            descriptor,
            _marker: PhantomData,
        }
    }

    /// Adds `NewT` to the list of triggers listened to by this observer.
    /// After calling this function the observer will no longer have access to typed trigger data
    /// to prtrigger accessing the data as the incorrect type.
    /// Observing the same trigger multiple times has no effect.
    pub fn on_trigger<NewT: 'static>(&mut self) -> &mut ObserverBuilder<'w, ()> {
        let trigger = self
            .commands
            .components()
            .get_id(TypeId::of::<NewT>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot observe trigger before it is registered with init_component: {}",
                    std::any::type_name::<NewT>(),
                )
            });
        self.descriptor.triggers.push(trigger);
        // SAFETY: () will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add `triggers` to the list of triggers listened to by this observer.
    /// After calling this function the observer will no longer have access to typed trigger data
    /// to prevent accessing the data as the incorrect type.
    /// Observing the same trigger multiple times has no effect.
    pub fn on_trigger_ids(
        &mut self,
        triggers: impl IntoIterator<Item = ComponentId>,
    ) -> &mut ObserverBuilder<'w, ()> {
        self.descriptor.triggers.extend(triggers);
        // SAFETY: () type will not allow bad memory access as it has no size
        unsafe { std::mem::transmute(self) }
    }

    /// Add all of the [`ComponentId`]s in `B` to the list of components listened to by this observer.
    /// For examples an `OnRemove` observer would trigger when any component in `B` was removed.
    pub fn components<B: Bundle>(&mut self) -> &mut Self {
        B::get_component_ids(self.commands.components(), &mut |id| {
            self.descriptor.components.push(id.unwrap_or_else(|| {
                panic!(
                    "Cannot observe trigger before it is registered with init_component: {}",
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
    pub fn run<B: Bundle, M>(&mut self, system: impl IntoObserverSystem<T, B, M>) -> Entity {
        B::get_component_ids(self.commands.components(), &mut |id| {
            self.descriptor.components.push(id.unwrap_or_else(|| {
                panic!(
                    "Cannot observe trigger before it is registered with init_component: {}",
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

    /// Spawns the resulting observer into the world using a custom [`ObserverRunner`] callback.
    /// This is not advised unless you want to override the default runner behaviour.
    pub fn runner(&mut self, runner: ObserverRunner) -> Entity {
        let entity = self.commands.spawn_empty().id();
        let descriptor: ObserverDescriptor = self.descriptor.clone();
        self.commands.add(move |world: &mut World| {
            let component = ObserverComponent::from_runner(descriptor, runner);
            world.entity_mut(entity).insert(component);
            world.register_observer(entity);
        });
        entity
    }
}

/// Type used to construct and emit an ECS trigger.
pub struct TriggerBuilder<'w, T = ()> {
    trigger: ComponentId,
    commands: Commands<'w, 'w>,
    targets: Vec<Entity>,
    components: Vec<ComponentId>,
    data: Option<T>,
}

impl<'w, T: Send + 'static> TriggerBuilder<'w, T> {
    /// Constructs a new builder that will write it's trigger to `world`'s command queue
    #[must_use]
    pub fn new(data: T, commands: Commands<'w, 'w>) -> Self {
        let trigger = commands
            .components()
            .get_id(TypeId::of::<T>())
            .unwrap_or_else(|| {
                panic!(
                    "Cannot emit trigger before it is registered with init_component: {}",
                    std::any::type_name::<T>()
                )
            });
        Self {
            trigger,
            commands,
            targets: Vec::new(),
            components: Vec::new(),
            data: Some(data),
        }
    }

    /// Sets the trigger id of the resulting trigger, used for dynamic triggers
    /// # Safety
    /// Caller must ensure that the component associated with `id` is accessible as E
    #[must_use]
    pub unsafe fn new_with_id(trigger: ComponentId, data: T, commands: Commands<'w, 'w>) -> Self {
        Self {
            trigger,
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

    /// Add the trigger to the command queue of world
    pub fn emit(&mut self) {
        // SAFETY: `self.trigger` is accessible as T, enforced in `Self::new` and `Self::new_with_id`.
        self.commands.add(unsafe {
            EmitTrigger::<T>::new(
                self.trigger,
                std::mem::take(&mut self.targets),
                std::mem::take(&mut self.components),
                std::mem::take(&mut self.data)
                    .expect("triggerBuilder used to send more than one trigger."),
            )
        });
    }
}

impl<'w, 's> Commands<'w, 's> {
    /// Constructs a [`TriggerBuilder`].
    #[must_use]
    pub fn trigger<T: Trigger>(&mut self, trigger: T) -> TriggerBuilder<T> {
        TriggerBuilder::new(trigger, self.reborrow())
    }

    /// Construct an [`ObserverBuilder`].
    #[must_use]
    pub fn observer_builder<T: Trigger>(&mut self) -> ObserverBuilder<T> {
        ObserverBuilder::new(self.reborrow())
    }

    /// Spawn an [`Observer`] and returns it's [`Entity`].
    pub fn observer<T: Trigger, B: Bundle, M>(
        &mut self,
        callback: impl IntoObserverSystem<T, B, M>,
    ) -> Entity {
        ObserverBuilder::new(self.reborrow()).run(callback)
    }
}
