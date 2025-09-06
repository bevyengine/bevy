//! Observers are a push-based tool for responding to [`Event`]s. The [`Observer`] component holds a [`System`] that runs whenever a matching [`Event`]
//! is triggered.
//!
//! See [`Event`] and [`Observer`] for in-depth documentation and usage examples.

mod centralized_storage;
mod distributed_storage;
mod entity_cloning;
mod runner;
mod system_param;

pub use centralized_storage::*;
pub use distributed_storage::*;
pub use runner::*;
pub use system_param::*;

use crate::{
    change_detection::MaybeLocation,
    event::Event,
    prelude::*,
    system::IntoObserverSystem,
    world::{DeferredWorld, *},
};

impl World {
    /// Spawns a "global" [`Observer`] which will watch for the given event.
    /// Returns its [`Entity`] as a [`EntityWorldMut`].
    ///
    /// `system` can be any system whose first parameter is [`On`].
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component)]
    /// struct A;
    ///
    /// # let mut world = World::new();
    /// world.add_observer(|_: On<Add, A>| {
    ///     // ...
    /// });
    /// world.add_observer(|_: On<Remove, A>| {
    ///     // ...
    /// });
    /// ```
    ///
    /// **Calling [`observe`](EntityWorldMut::observe) on the returned
    /// [`EntityWorldMut`] will observe the observer itself, which you very
    /// likely do not want.**
    ///
    /// # Panics
    ///
    /// Panics if the given system is an exclusive system.
    pub fn add_observer<E: Event, B: Bundle, M>(
        &mut self,
        system: impl IntoObserverSystem<E, B, M>,
    ) -> EntityWorldMut<'_> {
        self.spawn(Observer::new(system))
    }

    /// Triggers the given [`Event`], which will run any [`Observer`]s watching for it.
    ///
    /// For a variant that borrows the `event` rather than consuming it, use [`World::trigger_ref`] instead.
    #[track_caller]
    pub fn trigger<'a, E: Event<Trigger<'a>: Default>>(&mut self, mut event: E) {
        self.trigger_ref_with_caller(
            &mut event,
            &mut <E::Trigger<'a> as Default>::default(),
            MaybeLocation::caller(),
        );
    }

    /// A deprecated alias for [`trigger`](Self::trigger) to ease migration.
    ///
    /// Instead of specifying the trigger target separately,
    /// information about the target of the event is embedded in the data held by
    /// the event type itself.
    #[deprecated(since = "0.17.0", note = "Use `World::trigger` instead.")]
    pub fn trigger_targets<'a>(&mut self, event: impl Event<Trigger<'a>: Default>) {
        self.trigger(event);
    }

    /// Triggers the given [`Event`] using the given [`Trigger`](crate::event::Trigger), which will run any [`Observer`]s watching for it.
    ///
    /// For a variant that borrows the `event` rather than consuming it, use [`World::trigger_ref`] instead.
    #[track_caller]
    pub fn trigger_with<'a, E: Event>(&mut self, mut event: E, mut trigger: E::Trigger<'a>) {
        self.trigger_ref_with_caller(&mut event, &mut trigger, MaybeLocation::caller());
    }

    /// Triggers the given mutable [`Event`] reference, which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    #[track_caller]
    pub fn trigger_ref<'a, E: Event<Trigger<'a>: Default>>(&mut self, event: &mut E) {
        self.trigger_ref_with_caller(
            event,
            &mut <E::Trigger<'a> as Default>::default(),
            MaybeLocation::caller(),
        );
    }

    /// Triggers the given mutable [`Event`] reference using the given mutable [`Trigger`](crate::event::Trigger) reference, which
    /// will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    pub fn trigger_ref_with<'a, E: Event>(&mut self, event: &mut E, trigger: &mut E::Trigger<'a>) {
        self.trigger_ref_with_caller(event, trigger, MaybeLocation::caller());
    }

    pub(crate) fn trigger_ref_with_caller<'a, E: Event>(
        &mut self,
        event: &mut E,
        trigger: &mut E::Trigger<'a>,
        caller: MaybeLocation,
    ) {
        let event_key = self.register_event_key::<E>();
        // SAFETY: event_key was just registered and matches `event`
        unsafe {
            DeferredWorld::from(self).trigger_raw(event_key, event, trigger, caller);
        }
    }

    /// Register an observer to the cache, called when an observer is created
    pub(crate) fn register_observer(&mut self, observer_entity: Entity) {
        // SAFETY: References do not alias.
        let (observer_state, archetypes, observers) = unsafe {
            let observer_state: *const Observer = self.get::<Observer>(observer_entity).unwrap();
            // Populate ObservedBy for each observed entity.
            for watched_entity in (*observer_state).descriptor.entities.iter().copied() {
                let mut entity_mut = self.entity_mut(watched_entity);
                let mut observed_by = entity_mut.entry::<ObservedBy>().or_default().into_mut();
                observed_by.0.push(observer_entity);
            }
            (&*observer_state, &mut self.archetypes, &mut self.observers)
        };
        let descriptor = &observer_state.descriptor;

        for &event_key in &descriptor.event_keys {
            let cache = observers.get_observers_mut(event_key);

            if descriptor.components.is_empty() && descriptor.entities.is_empty() {
                cache
                    .global_observers
                    .insert(observer_entity, observer_state.runner);
            } else if descriptor.components.is_empty() {
                // Observer is not targeting any components so register it as an entity observer
                for &watched_entity in &observer_state.descriptor.entities {
                    let map = cache.entity_observers.entry(watched_entity).or_default();
                    map.insert(observer_entity, observer_state.runner);
                }
            } else {
                // Register observer for each watched component
                for &component in &descriptor.components {
                    let observers =
                        cache
                            .component_observers
                            .entry(component)
                            .or_insert_with(|| {
                                if let Some(flag) = Observers::is_archetype_cached(event_key) {
                                    archetypes.update_flags(component, flag, true);
                                }
                                CachedComponentObservers::default()
                            });
                    if descriptor.entities.is_empty() {
                        // Register for all triggers targeting the component
                        observers
                            .global_observers
                            .insert(observer_entity, observer_state.runner);
                    } else {
                        // Register for each watched entity
                        for &watched_entity in &descriptor.entities {
                            let map = observers
                                .entity_component_observers
                                .entry(watched_entity)
                                .or_default();
                            map.insert(observer_entity, observer_state.runner);
                        }
                    }
                }
            }
        }
    }

    /// Remove the observer from the cache, called when an observer gets despawned
    pub(crate) fn unregister_observer(&mut self, entity: Entity, descriptor: ObserverDescriptor) {
        let archetypes = &mut self.archetypes;
        let observers = &mut self.observers;

        for &event_key in &descriptor.event_keys {
            let cache = observers.get_observers_mut(event_key);
            if descriptor.components.is_empty() && descriptor.entities.is_empty() {
                cache.global_observers.remove(&entity);
            } else if descriptor.components.is_empty() {
                for watched_entity in &descriptor.entities {
                    // This check should be unnecessary since this observer hasn't been unregistered yet
                    let Some(observers) = cache.entity_observers.get_mut(watched_entity) else {
                        continue;
                    };
                    observers.remove(&entity);
                    if observers.is_empty() {
                        cache.entity_observers.remove(watched_entity);
                    }
                }
            } else {
                for component in &descriptor.components {
                    let Some(observers) = cache.component_observers.get_mut(component) else {
                        continue;
                    };
                    if descriptor.entities.is_empty() {
                        observers.global_observers.remove(&entity);
                    } else {
                        for watched_entity in &descriptor.entities {
                            let Some(map) =
                                observers.entity_component_observers.get_mut(watched_entity)
                            else {
                                continue;
                            };
                            map.remove(&entity);
                            if map.is_empty() {
                                observers.entity_component_observers.remove(watched_entity);
                            }
                        }
                    }

                    if observers.global_observers.is_empty()
                        && observers.entity_component_observers.is_empty()
                    {
                        cache.component_observers.remove(component);
                        if let Some(flag) = Observers::is_archetype_cached(event_key) {
                            if let Some(by_component) = archetypes.by_component.get(component) {
                                for archetype in by_component.keys() {
                                    let archetype = &mut archetypes.archetypes[archetype.index()];
                                    if archetype.contains(*component) {
                                        let no_longer_observed = archetype
                                            .iter_components()
                                            .all(|id| !cache.component_observers.contains_key(&id));

                                        if no_longer_observed {
                                            archetype.flags.set(flag, false);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::{vec, vec::Vec};

    use bevy_ptr::OwningPtr;

    use crate::{
        change_detection::MaybeLocation,
        entity_disabling::Internal,
        event::{EntityComponentsTrigger, Event, GlobalTrigger},
        hierarchy::ChildOf,
        observer::{Observer, Replace},
        prelude::*,
        world::DeferredWorld,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct S;

    #[derive(Event)]
    struct EventA;

    #[derive(EntityEvent)]
    struct EntityEventA(Entity);

    #[derive(EntityEvent)]
    #[entity_event(trigger = EntityComponentsTrigger<'a>)]
    struct EntityComponentsEvent(Entity);

    #[derive(Event)]
    struct EventWithData {
        counter: usize,
    }

    #[derive(Resource, Default)]
    struct Order(Vec<&'static str>);

    impl Order {
        #[track_caller]
        fn observed(&mut self, name: &'static str) {
            self.0.push(name);
        }
    }

    #[derive(Component, EntityEvent)]
    #[entity_event(propagate, auto_propagate)]
    struct EventPropagating(Entity);

    #[test]
    fn observer_order_spawn_despawn() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<Add, A>, mut res: ResMut<Order>| res.observed("add"));
        world.add_observer(|_: On<Insert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.add_observer(|_: On<Replace, A>, mut res: ResMut<Order>| {
            res.observed("replace");
        });
        world.add_observer(|_: On<Remove, A>, mut res: ResMut<Order>| res.observed("remove"));

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_insert_remove() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<Add, A>, mut res: ResMut<Order>| res.observed("add"));
        world.add_observer(|_: On<Insert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.add_observer(|_: On<Replace, A>, mut res: ResMut<Order>| {
            res.observed("replace");
        });
        world.add_observer(|_: On<Remove, A>, mut res: ResMut<Order>| res.observed("remove"));

        let mut entity = world.spawn_empty();
        entity.insert(A);
        entity.remove::<A>();
        entity.flush();
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_insert_remove_sparse() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<Add, S>, mut res: ResMut<Order>| res.observed("add"));
        world.add_observer(|_: On<Insert, S>, mut res: ResMut<Order>| res.observed("insert"));
        world.add_observer(|_: On<Replace, S>, mut res: ResMut<Order>| {
            res.observed("replace");
        });
        world.add_observer(|_: On<Remove, S>, mut res: ResMut<Order>| res.observed("remove"));

        let mut entity = world.spawn_empty();
        entity.insert(S);
        entity.remove::<S>();
        entity.flush();
        assert_eq!(
            vec!["add", "insert", "replace", "remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_order_replace() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world.spawn(A).id();

        world.add_observer(|_: On<Add, A>, mut res: ResMut<Order>| res.observed("add"));
        world.add_observer(|_: On<Insert, A>, mut res: ResMut<Order>| res.observed("insert"));
        world.add_observer(|_: On<Replace, A>, mut res: ResMut<Order>| {
            res.observed("replace");
        });
        world.add_observer(|_: On<Remove, A>, mut res: ResMut<Order>| res.observed("remove"));

        let mut entity = world.entity_mut(entity);
        entity.insert(A);
        entity.flush();
        assert_eq!(vec!["replace", "insert"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_order_recursive() {
        let mut world = World::new();
        world.init_resource::<Order>();
        world.add_observer(
            |add: On<Add, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_a");
                commands.entity(add.entity).insert(B);
            },
        );
        world.add_observer(
            |remove: On<Remove, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("remove_a");
                commands.entity(remove.entity).remove::<B>();
            },
        );

        world.add_observer(
            |add: On<Add, B>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_b");
                commands.entity(add.entity).remove::<A>();
            },
        );
        world.add_observer(|_: On<Remove, B>, mut res: ResMut<Order>| {
            res.observed("remove_b");
        });

        let entity = world.spawn(A).flush();
        let entity = world.get_entity(entity).unwrap();
        assert!(!entity.contains::<A>());
        assert!(!entity.contains::<B>());
        assert_eq!(
            vec!["add_a", "add_b", "remove_a", "remove_b"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_trigger_ref() {
        let mut world = World::new();

        world.add_observer(|mut event: On<EventWithData>| event.counter += 1);
        world.add_observer(|mut event: On<EventWithData>| event.counter += 2);
        world.add_observer(|mut event: On<EventWithData>| event.counter += 4);

        let mut event = EventWithData { counter: 0 };
        world.trigger_ref(&mut event);
        assert_eq!(7, event.counter);
    }

    #[test]
    fn observer_multiple_listeners() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<Add, A>, mut res: ResMut<Order>| res.observed("add_1"));
        world.add_observer(|_: On<Add, A>, mut res: ResMut<Order>| res.observed("add_2"));

        world.spawn(A).flush();
        assert_eq!(vec!["add_2", "add_1"], world.resource::<Order>().0);
        // we have one A entity and two observers
        assert_eq!(world.query::<&A>().query(&world).count(), 1);
        assert_eq!(
            world
                .query_filtered::<&Observer, Allow<Internal>>()
                .query(&world)
                .count(),
            2
        );
    }

    #[test]
    fn observer_multiple_events() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let on_remove = world.register_event_key::<Remove>();
        world.spawn(
            // SAFETY: Add and Remove are both unit types, so this is safe
            unsafe {
                Observer::new(|_: On<Add, A>, mut res: ResMut<Order>| {
                    res.observed("add/remove");
                })
                .with_event_key(on_remove)
            },
        );

        let entity = world.spawn(A).id();
        world.despawn(entity);
        assert_eq!(
            vec!["add/remove", "add/remove"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_multiple_components() {
        let mut world = World::new();
        world.init_resource::<Order>();
        world.register_component::<A>();
        world.register_component::<B>();

        world.add_observer(|_: On<Add, (A, B)>, mut res: ResMut<Order>| {
            res.observed("add_ab");
        });

        let entity = world.spawn(A).id();
        world.entity_mut(entity).insert(B);
        assert_eq!(vec!["add_ab", "add_ab"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_despawn() {
        let mut world = World::new();

        let system: fn(On<Add, A>) = |_| {
            panic!("Observer triggered after being despawned.");
        };
        let observer = world.add_observer(system).id();
        world.despawn(observer);
        world.spawn(A).flush();
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/14961
    #[test]
    fn observer_despawn_archetype_flags() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world.spawn((A, B)).flush();

        world.add_observer(|_: On<Remove, A>, mut res: ResMut<Order>| {
            res.observed("remove_a");
        });

        let system: fn(On<Remove, B>) = |_: On<Remove, B>| {
            panic!("Observer triggered after being despawned.");
        };

        let observer = world.add_observer(system).flush();
        world.despawn(observer);

        world.despawn(entity);

        assert_eq!(vec!["remove_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_multiple_matches() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<Add, (A, B)>, mut res: ResMut<Order>| {
            res.observed("add_ab");
        });

        world.spawn((A, B)).flush();
        assert_eq!(vec!["add_ab"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_entity_routing() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let system: fn(On<EntityEventA>) = |_| {
            panic!("Trigger routed to non-targeted entity.");
        };

        world.spawn_empty().observe(system);
        let entity = world
            .spawn_empty()
            .observe(|_: On<EntityEventA>, mut res: ResMut<Order>| res.observed("a_1"))
            .id();
        world.add_observer(move |event: On<EntityEventA>, mut res: ResMut<Order>| {
            assert_eq!(event.event_target(), entity);
            res.observed("a_2");
        });

        world.trigger(EntityEventA(entity));
        assert_eq!(vec!["a_2", "a_1"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_multiple_targets() {
        #[derive(Resource, Default)]
        struct R(i32);

        let mut world = World::new();
        let component_a = world.register_component::<A>();
        let component_b = world.register_component::<B>();
        world.init_resource::<R>();

        // targets (entity_1, A)
        let entity_1 = world
            .spawn_empty()
            .observe(|_: On<EntityComponentsEvent, A>, mut res: ResMut<R>| res.0 += 1)
            .id();
        // targets (entity_2, B)
        let entity_2 = world
            .spawn_empty()
            .observe(|_: On<EntityComponentsEvent, B>, mut res: ResMut<R>| res.0 += 10)
            .id();
        // targets any entity or component
        world.add_observer(|_: On<EntityComponentsEvent>, mut res: ResMut<R>| res.0 += 100);
        // targets any entity, and components A or B
        world
            .add_observer(|_: On<EntityComponentsEvent, (A, B)>, mut res: ResMut<R>| res.0 += 1000);
        // test all tuples
        world.add_observer(
            |_: On<EntityComponentsEvent, (A, B, (A, B))>, mut res: ResMut<R>| res.0 += 10000,
        );
        world.add_observer(
            |_: On<EntityComponentsEvent, (A, B, (A, B), ((A, B), (A, B)))>, mut res: ResMut<R>| {
                res.0 += 100000;
            },
        );
        world.add_observer(
            |_: On<EntityComponentsEvent, (A, B, (A, B), (B, A), (A, B, ((A, B), (B, A))))>,
             mut res: ResMut<R>| res.0 += 1000000,
        );

        // trigger for an entity and a component
        world.trigger_with(
            EntityComponentsEvent(entity_1),
            EntityComponentsTrigger {
                components: &[component_a],
            },
        );
        // only observer that doesn't trigger is the one only watching entity_2
        assert_eq!(1111101, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger for both entities, but no components: trigger once per entity target
        world.trigger_with(
            EntityComponentsEvent(entity_1),
            EntityComponentsTrigger { components: &[] },
        );
        world.trigger_with(
            EntityComponentsEvent(entity_2),
            EntityComponentsTrigger { components: &[] },
        );

        // only the observer that doesn't require components triggers - once per entity
        assert_eq!(200, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger for both entities and both components: trigger once per entity target
        // we only get 2222211 because a given observer can trigger only once per entity target
        world.trigger_with(
            EntityComponentsEvent(entity_1),
            EntityComponentsTrigger {
                components: &[component_a, component_b],
            },
        );
        world.trigger_with(
            EntityComponentsEvent(entity_2),
            EntityComponentsTrigger {
                components: &[component_a, component_b],
            },
        );
        assert_eq!(2222211, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;
    }

    #[test]
    fn observer_dynamic_component() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let component_id = world.register_component::<A>();
        world.spawn(
            Observer::new(|_: On<Add>, mut res: ResMut<Order>| res.observed("event_a"))
                .with_component(component_id),
        );

        let mut entity = world.spawn_empty();
        OwningPtr::make(A, |ptr| {
            // SAFETY: we registered `component_id` above.
            unsafe { entity.insert_by_id(component_id, ptr) };
        });

        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_dynamic_trigger() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let event_a = world.register_event_key::<EventA>();

        // SAFETY: we registered `event_a` above and it matches the type of EventA
        let observe = unsafe {
            Observer::with_dynamic_runner(
                |mut world, _observer, _trigger_context, _event, _trigger| {
                    world.resource_mut::<Order>().observed("event_a");
                },
            )
            .with_event_key(event_a)
        };
        world.spawn(observe);

        world.commands().queue(move |world: &mut World| {
            // SAFETY: we registered `event_a` above and it matches the type of EventA
            unsafe {
                DeferredWorld::from(world).trigger_raw(
                    event_a,
                    &mut EventA,
                    &mut GlobalTrigger,
                    MaybeLocation::caller(),
                );
            }
        });
        world.flush();
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world.spawn_empty().id();
        let child = world.spawn(ChildOf(parent)).id();

        world.entity_mut(parent).observe(
            move |event: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");

                assert_eq!(event.event_target(), parent);
                assert_eq!(event.original_event_target(), child);
            },
        );

        world.entity_mut(child).observe(
            move |event: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
                assert_eq!(event.event_target(), child);
                assert_eq!(event.original_event_target(), child);
            },
        );

        world.trigger(EventPropagating(child));

        assert_eq!(vec!["child", "parent"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_redundant_dispatch_same_entity() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(ChildOf(parent))
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
            })
            .id();

        world.trigger(EventPropagating(child));
        world.trigger(EventPropagating(child));

        assert_eq!(
            vec!["child", "parent", "child", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_redundant_dispatch_parent_child() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(ChildOf(parent))
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
            })
            .id();

        world.trigger(EventPropagating(child));
        world.trigger(EventPropagating(parent));

        assert_eq!(
            vec!["child", "parent", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_halt() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child = world
            .spawn(ChildOf(parent))
            .observe(|mut event: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
                event.propagate(false);
            })
            .id();

        world.trigger(EventPropagating(child));

        assert_eq!(vec!["child"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_join() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");
            })
            .id();

        let child_a = world
            .spawn(ChildOf(parent))
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_a");
            })
            .id();

        let child_b = world
            .spawn(ChildOf(parent))
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_b");
            })
            .id();

        world.trigger(EventPropagating(child_a));
        world.trigger(EventPropagating(child_b));

        assert_eq!(
            vec!["child_a", "parent", "child_b", "parent"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_no_next() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let entity = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("event");
            })
            .id();

        world.trigger(EventPropagating(entity));
        assert_eq!(vec!["event"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_parallel_propagation() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let parent_a = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent_a");
            })
            .id();

        let child_a = world
            .spawn(ChildOf(parent_a))
            .observe(|mut event: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_a");
                event.propagate(false);
            })
            .id();

        let parent_b = world
            .spawn_empty()
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent_b");
            })
            .id();

        let child_b = world
            .spawn(ChildOf(parent_b))
            .observe(|_: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child_b");
            })
            .id();

        world.trigger(EventPropagating(child_a));
        world.trigger(EventPropagating(child_b));

        assert_eq!(
            vec!["child_a", "child_b", "parent_b"],
            world.resource::<Order>().0
        );
    }

    #[test]
    fn observer_propagating_world() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(|_: On<EventPropagating>, mut res: ResMut<Order>| {
            res.observed("event");
        });

        let grandparent = world.spawn_empty().id();
        let parent = world.spawn(ChildOf(grandparent)).id();
        let child = world.spawn(ChildOf(parent)).id();

        world.trigger(EventPropagating(child));

        assert_eq!(vec!["event", "event", "event"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_world_skipping() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(
            |event: On<EventPropagating>, query: Query<&A>, mut res: ResMut<Order>| {
                if query.get(event.event_target()).is_ok() {
                    res.observed("event");
                }
            },
        );

        let grandparent = world.spawn(A).id();
        let parent = world.spawn(ChildOf(grandparent)).id();
        let child = world.spawn((A, ChildOf(parent))).id();

        world.trigger(EventPropagating(child));

        assert_eq!(vec!["event", "event"], world.resource::<Order>().0);
    }

    // Originally for https://github.com/bevyengine/bevy/issues/18452
    #[test]
    fn observer_modifies_relationship() {
        fn on_add(add: On<Add, A>, mut commands: Commands) {
            commands
                .entity(add.entity)
                .with_related_entities::<crate::hierarchy::ChildOf>(|rsc| {
                    rsc.spawn_empty();
                });
        }

        let mut world = World::new();
        world.add_observer(on_add);
        world.spawn(A);
    }

    // Regression test for https://github.com/bevyengine/bevy/issues/14467
    // Fails prior to https://github.com/bevyengine/bevy/pull/15398
    #[test]
    fn observer_on_remove_during_despawn_spawn_empty() {
        let mut world = World::new();

        // Observe the removal of A - this will run during despawn
        world.add_observer(|_: On<Remove, A>, mut cmd: Commands| {
            // Spawn a new entity - this reserves a new ID and requires a flush
            // afterward before Entities::free can be called.
            cmd.spawn_empty();
        });

        let ent = world.spawn(A).id();

        // Despawn our entity, which runs the Remove observer and allocates a
        // new Entity.
        // Should not panic - if it does, then Entities was not flushed properly
        // after the observer's spawn_empty.
        world.despawn(ent);
    }

    #[test]
    #[should_panic]
    fn observer_invalid_params() {
        #[derive(Resource)]
        struct ResA;

        #[derive(Resource)]
        struct ResB;

        let mut world = World::new();
        // This fails because `ResA` is not present in the world
        world.add_observer(|_: On<EventA>, _: Res<ResA>, mut commands: Commands| {
            commands.insert_resource(ResB);
        });
        world.trigger(EventA);
    }

    #[test]
    fn observer_apply_deferred_from_param_set() {
        #[derive(Resource)]
        struct ResA;

        let mut world = World::new();
        world.add_observer(
            |_: On<EventA>, mut params: ParamSet<(Query<Entity>, Commands)>| {
                params.p1().insert_resource(ResA);
            },
        );

        world.trigger(EventA);
        world.flush();

        assert!(world.get_resource::<ResA>().is_some());
    }

    #[test]
    #[track_caller]
    fn observer_caller_location_event() {
        #[derive(Event)]
        struct EventA;

        let caller = MaybeLocation::caller();
        let mut world = World::new();
        world.add_observer(move |event: On<EventA>| {
            assert_eq!(event.caller(), caller);
        });
        world.trigger(EventA);
    }

    #[test]
    #[track_caller]
    fn observer_caller_location_command_archetype_move() {
        #[derive(Component)]
        struct Component;

        let caller = MaybeLocation::caller();
        let mut world = World::new();
        world.add_observer(move |event: On<Add, Component>| {
            assert_eq!(event.caller(), caller);
        });
        world.add_observer(move |event: On<Remove, Component>| {
            assert_eq!(event.caller(), caller);
        });
        world.commands().spawn(Component).clear();
    }

    #[test]
    fn observer_watch_entities() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let entities = world
            .spawn_batch(core::iter::repeat_n((), 4))
            .collect::<Vec<_>>();
        let observer = Observer::new(|_: On<EntityEventA>, mut order: ResMut<Order>| {
            order.observed("a");
        });
        world.spawn(observer.with_entities(entities.iter().copied().take(2)));

        world.trigger(EntityEventA(entities[0]));
        world.trigger(EntityEventA(entities[1]));
        assert_eq!(vec!["a", "a"], world.resource::<Order>().0);
        world.trigger(EntityEventA(entities[2]));
        world.trigger(EntityEventA(entities[3]));
        assert_eq!(vec!["a", "a"], world.resource::<Order>().0);
    }

    #[test]
    fn unregister_global_observer() {
        let mut world = World::new();
        let mut observer = world.add_observer(|_: On<EventA>| {});
        observer.remove::<Observer>();
        let id = observer.id();
        let event_key = world.event_key::<EventA>().unwrap();
        assert!(!world
            .observers
            .get_observers_mut(event_key)
            .global_observers
            .contains_key(&id));
    }

    #[test]
    fn unregister_entity_observer() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let observer = Observer::new(|_: On<EventA>| {}).with_entity(entity);
        let mut observer = world.spawn(observer);
        observer.remove::<Observer>();
        let event_key = world.event_key::<EventA>().unwrap();
        assert!(!world
            .observers
            .get_observers_mut(event_key)
            .entity_observers
            .contains_key(&entity));
    }

    #[test]
    fn unregister_component_observer() {
        let mut world = World::new();
        let a = world.register_component::<A>();
        let observer = Observer::new(|_: On<EventA>| {}).with_component(a);
        let mut observer = world.spawn(observer);
        observer.remove::<Observer>();
        let event_key = world.event_key::<EventA>().unwrap();
        assert!(!world
            .observers
            .get_observers_mut(event_key)
            .component_observers()
            .contains_key(&a));
    }
}
