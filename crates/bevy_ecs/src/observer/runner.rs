use super::*;

#[derive(Copy, Clone, Debug)]
pub(crate) struct ObserverCallback {
    pub(crate) run: ObserverRunner,
    pub(crate) callback: Option<fn(Observer<(), ()>)>,
}

/// Type for function that is run when an observer is triggered
/// Typically refers to the default runner defined in [`ObserverComponent::from`]
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut, Option<fn(Observer<(), ()>)>);

pub(crate) struct ObserverComponent {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) callback: ObserverCallback,
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
        value: fn(Observer<E, Q, F>),
    ) -> Self {
        Self {
            descriptor,
            callback: ObserverCallback {
                run: |mut world, trigger, ptr, callback| {
                    if trigger.source == Entity::PLACEHOLDER {
                        return;
                    }
                    let callback: fn(Observer<E, Q, F>) =
                        unsafe { std::mem::transmute(callback.debug_checked_unwrap()) };
                    let state = unsafe {
                        let mut state = world
                            .get_mut::<ObserverState<Q, F>>(trigger.observer)
                            .debug_checked_unwrap();
                        let state: *mut ObserverState<Q, F> = state.as_mut();
                        &mut *state
                    };
                    // This being stored in a component is not ideal, should be able to check this before fetching
                    let last_event = world.last_event_id;
                    if state.last_event_id == last_event {
                        return;
                    }
                    state.last_event_id = last_event;

                    let archetype_id = trigger.location.archetype_id;
                    let archetype = &world.archetypes()[archetype_id];
                    if !Q::matches_component_set(&state.fetch_state, &mut |id| {
                        archetype.contains(id)
                    }) || !F::matches_component_set(&state.filter_state, &mut |id| {
                        archetype.contains(id)
                    }) {
                        return;
                    }

                    // TODO: Change ticks?
                    unsafe {
                        let mut filter_fetch = F::init_fetch(
                            world.as_unsafe_world_cell_readonly(),
                            &state.filter_state,
                            world.last_change_tick(),
                            world.read_change_tick(),
                        );

                        if !F::filter_fetch(
                            &mut filter_fetch,
                            trigger.source,
                            trigger.location.table_row,
                        ) {
                            return;
                        }
                    }

                    // SAFETY: Pointer is valid as we just created it, ObserverState is a private type and so will not be aliased
                    let observer = Observer::new(world, state, unsafe { ptr.deref_mut() }, trigger);
                    callback(observer);
                },
                callback: Some(unsafe { std::mem::transmute(value) }),
            },
        }
    }

    pub(crate) fn from_runner(descriptor: ObserverDescriptor, run: ObserverRunner) -> Self {
        Self {
            descriptor,
            callback: ObserverCallback {
                run,
                callback: None,
            },
        }
    }
}
