use super::*;

/// Type for function that is run when an observer is triggered
/// Typically refers to the default runner defined in [`ObserverComponent::from`]
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut);

/// Trait that is implemented for all functions that can be used as [`Observer`] callbacks
pub trait ObserverCallback<E, Q: WorldQueryData, F: WorldQueryFilter>: Send + Sync {
    /// Invokes the callback with the passed [`Observer`]
    fn call(&mut self, observer: Observer<E, Q, F>);
}

impl<E, Q: WorldQueryData, F: WorldQueryFilter, C: FnMut(Observer<E, Q, F>) + Send + Sync>
    ObserverCallback<E, Q, F> for C
{
    fn call(&mut self, observer: Observer<E, Q, F>) {
        self(observer)
    }
}

pub(crate) struct ObserverComponent {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) runner: ObserverRunner,
    pub(crate) callback: Option<Box<dyn ObserverCallback<(), (), ()>>>,
}

impl Component for ObserverComponent {
    type Storage = SparseStorage;

    fn init_component_info(info: &mut ComponentInfo) {
        info.on_add(|mut world, entity, _| {
            let (world, archetypes, observers) = unsafe {
                let world = world.as_unsafe_world_cell();
                (
                    world.into_deferred(),
                    world.archetypes_mut(),
                    world.observers_mut(),
                )
            };

            let observer = world.get::<ObserverComponent>(entity).unwrap();
            observers.register(archetypes, entity, observer);
        })
        .on_remove(|mut world, entity, _| {
            let (world, archetypes, observers) = unsafe {
                let world = world.as_unsafe_world_cell();
                (
                    world.into_deferred(),
                    world.archetypes_mut(),
                    world.observers_mut(),
                )
            };

            let observer = world.get::<ObserverComponent>(entity).unwrap();
            observers.unregister(archetypes, entity, observer);
        });
    }
}

impl ObserverComponent {
    pub(crate) fn from<E: 'static, Q: WorldQueryData + 'static, F: WorldQueryFilter + 'static>(
        descriptor: ObserverDescriptor,
        value: impl ObserverCallback<E, Q, F> + 'static,
    ) -> Self {
        Self {
            descriptor,
            runner: |mut world, trigger, ptr| {
                if trigger.source == Entity::PLACEHOLDER {
                    return;
                }
                println!("Trigger: {:?}", std::any::type_name::<(E, Q, F)>());
                let world = world.as_unsafe_world_cell();
                let observer_cell =
                    unsafe { world.get_entity(trigger.observer).debug_checked_unwrap() };
                let mut state = unsafe {
                    observer_cell
                        .get_mut::<ObserverState<Q, F>>()
                        .debug_checked_unwrap()
                };

                // This being stored in a component is not ideal, should be able to check this before fetching
                let last_event = unsafe { world.world() }.last_event_id;
                if state.last_event_id == last_event {
                    return;
                }
                state.last_event_id = last_event;

                let archetype_id = trigger.location.archetype_id;
                let archetype = &world.archetypes()[archetype_id];
                if !Q::matches_component_set(&state.fetch_state, &mut |id| archetype.contains(id))
                    || !F::matches_component_set(&state.filter_state, &mut |id| {
                        archetype.contains(id)
                    })
                {
                    return;
                }

                // TODO: Change ticks?
                unsafe {
                    let mut filter_fetch = F::init_fetch(
                        world,
                        &state.filter_state,
                        world.last_change_tick(),
                        world.change_tick(),
                    );

                    if !F::filter_fetch(
                        &mut filter_fetch,
                        trigger.source,
                        trigger.location.table_row,
                    ) {
                        return;
                    }
                }
                let mut component = unsafe {
                    observer_cell
                        .get_mut::<ObserverComponent>()
                        .debug_checked_unwrap()
                };

                if let Some(callback) = &mut component.callback {
                    // SAFETY: Pointer is valid as we just created it, ObserverState is a private type and so will not be aliased
                    let observer = Observer::new(
                        unsafe { world.into_deferred() },
                        state.as_mut(),
                        unsafe { ptr.deref_mut() },
                        trigger,
                    );
                    let callback: &mut Box<dyn FnMut(Observer<E, Q, F>) + Send + Sync> =
                        unsafe { std::mem::transmute(callback) };
                    callback.call(observer);
                }
            },
            callback: Some(unsafe {
                std::mem::transmute(Box::new(value) as Box<dyn ObserverCallback<E, Q, F>>)
            }),
        }
    }

    pub(crate) fn from_runner(descriptor: ObserverDescriptor, runner: ObserverRunner) -> Self {
        Self {
            descriptor,
            runner,
            callback: None,
        }
    }
}
