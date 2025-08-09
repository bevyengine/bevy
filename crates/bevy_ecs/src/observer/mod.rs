//! Observers are a push-based tool for responding to [`Event`]s.
//!
//! ## Observer targeting
//!
//! Observers can be "global", listening for events that are both targeted at and not targeted at any specific entity,
//! or they can be "entity-specific", listening for events that are targeted at specific entities.
//!
//! They can also be further refined by listening to events targeted at specific components
//! (instead of using a generic event type), as is done with the [`Add`] family of lifecycle events.
//!
//! When entities are observed, they will receive an [`ObservedBy`] component,
//! which will be updated to track the observers that are currently observing them.
//!
//! Currently, [observers cannot be retargeted after spawning](https://github.com/bevyengine/bevy/issues/19587):
//! despawn and respawn an observer as a workaround.
//!
//! ## Writing observers
//!
//! Observers are systems which implement [`IntoObserverSystem`] that listen for [`Event`]s matching their
//! type and target(s).
//! To write observer systems, use [`On`] as the first parameter of your system.
//! This parameter provides access to the specific event that triggered the observer,
//! as well as the entity that the event was targeted at, if any.
//!
//! Observers can request other data from the world, such as via a [`Query`] or [`Res`].
//! Commonly, you might want to verify that the entity that the observable event is targeting
//! has a specific component, or meets some other condition. [`Query::get`] or [`Query::contains`]
//! on the [`On::target`] entity is a good way to do this.
//!
//! [`Commands`] can also be used inside of observers.
//! This can be particularly useful for triggering other observers!
//!
//! ## Spawning observers
//!
//! Observers can be spawned via [`World::add_observer`], or the equivalent app method.
//! This will cause an entity with the [`Observer`] component to be created,
//! which will then run the observer system whenever the event it is watching is triggered.
//!
//! You can control the targets that an observer is watching by calling [`Observer::watch_entity`]
//! once the entity is spawned, or by manually spawning an entity with the [`Observer`] component
//! configured with the desired targets.
//!
//! Observers are fundamentally defined as "entities which have the [`Observer`] component"
//! allowing you to add it manually to existing entities.
//! At first, this seems convenient, but only one observer can be added to an entity at a time,
//! regardless of the event it responds to: like always, components are unique.
//!
//! Instead, a better way to achieve a similar aim is to
//! use the [`EntityWorldMut::observe`] / [`EntityCommands::observe`] method,
//! which spawns a new observer, and configures it to watch the entity it is called on.
//! Unfortunately, observers defined in this way
//! [currently cannot be spawned as part of bundles](https://github.com/bevyengine/bevy/issues/14204).
//!
//! ## Triggering observers
//!
//! Observers are most commonly triggered by [`Commands`],
//! via [`Commands::trigger`] (for untargeted [`Event`]s) or [`Commands::trigger_targets`] (for targeted [`EntityEvent`]s).
//! Like usual, equivalent methods are available on [`World`], allowing you to reduce overhead when working with exclusive world access.
//!
//! If your observer is configured to watch for a specific component or set of components instead,
//! you can pass in [`ComponentId`]s into [`Commands::trigger_targets`] by using the [`TriggerTargets`] trait.
//! As discussed in the [`On`] documentation, this use case is rare, and is currently only used
//! for [lifecycle](crate::lifecycle) events, which are automatically emitted.
//!
//! ## Observer bubbling
//!
//! When using an [`EntityEvent`] targeted at an entity, the event can optionally be propagated to other targets,
//! typically up to parents in an entity hierarchy.
//!
//! This behavior is controlled via [`EntityEvent::Traversal`] and [`EntityEvent::AUTO_PROPAGATE`],
//! with the details of the propagation path specified by the [`Traversal`](crate::traversal::Traversal) trait.
//!
//! When auto-propagation is enabled, propagation must be manually stopped to prevent the event from
//! continuing to other targets. This can be done using the [`On::propagate`] method inside of your observer.
//!
//! ## Observer timing
//!
//! Observers are triggered via [`Commands`], which imply that they are evaluated at the next sync point in the ECS schedule.
//! Accordingly, they have full access to the world, and are evaluated sequentially, in the order that the commands were sent.
//!
//! To control the relative ordering of observers sent from different systems,
//! order the systems in the schedule relative to each other.
//!
//! Currently, Bevy does not provide [a way to specify the ordering of observers](https://github.com/bevyengine/bevy/issues/14890)
//! listening to the same event relative to each other.
//!
//! Commands sent by observers are [currently not immediately applied](https://github.com/bevyengine/bevy/issues/19569).
//! Instead, all queued observers will run, and then all of the commands from those observers will be applied.
//! Careful use of [`Schedule::apply_deferred`] may help as a workaround.
//!
//! ## Lifecycle events and observers
//!
//! It is important to note that observers, just like [hooks](crate::lifecycle::ComponentHooks),
//! can listen to and respond to [lifecycle](crate::lifecycle) events.
//! Unlike hooks, observers are not treated as an "innate" part of component behavior:
//! they can be added or removed at runtime, and multiple observers
//! can be registered for the same lifecycle event for the same component.
//!
//! The ordering of hooks versus observers differs based on the lifecycle event in question:
//!
//! - when adding components, hooks are evaluated first, then observers
//! - when removing components, observers are evaluated first, then hooks
//!
//! This allows hooks to act as constructors and destructors for components,
//! as they always have the first and final say in the component's lifecycle.
//!
//! ## Cleaning up observers
//!
//! Currently, observer entities are never cleaned up, even if their target entity(s) are despawned.
//! This won't cause any runtime overhead, but is a waste of memory and can result in memory leaks.
//!
//! If you run into this problem, you could manually scan the world for observer entities and despawn them,
//! by checking if the entity in [`Observer::descriptor`] still exists.
//!
//! ## Observers vs buffered events
//!
//! By contrast, [`EventReader`] and [`EventWriter`] ("buffered events"), are pull-based.
//! They require periodically polling the world to check for new events, typically in a system that runs as part of a schedule.
//!
//! This imposes a small overhead, making observers a better choice for extremely rare events,
//! but buffered events can be more efficient for events that are expected to occur multiple times per frame,
//! as it allows for batch processing of events.
//!
//! The difference in timing is also an important consideration:
//! buffered events are evaluated at fixed points during schedules,
//! while observers are evaluated as soon as possible after the event is triggered.
//!
//! This provides more control over the timing of buffered event evaluation,
//! but allows for a more ad hoc approach with observers,
//! and enables indefinite chaining of observers triggering other observers (for both better and worse!).

mod centralized_storage;
mod distributed_storage;
mod entity_cloning;
mod runner;
mod system_param;
mod trigger_targets;

pub use centralized_storage::*;
pub use distributed_storage::*;
pub use runner::*;
pub use system_param::*;
pub use trigger_targets::*;

use crate::{
    change_detection::MaybeLocation,
    component::ComponentId,
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
    /// **Calling [`observe`](EntityWorldMut::observe) on the returned
    /// [`EntityWorldMut`] will observe the observer itself, which you very
    /// likely do not want.**
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
    /// While event types commonly implement [`Copy`],
    /// those that don't will be consumed and will no longer be accessible.
    /// If you need to use the event after triggering it, use [`World::trigger_ref`] instead.
    #[track_caller]
    pub fn trigger<E: Event>(&mut self, event: E) {
        self.trigger_with_caller(event, MaybeLocation::caller());
    }

    pub(crate) fn trigger_with_caller<E: Event>(&mut self, mut event: E, caller: MaybeLocation) {
        let event_key = E::register_event_key(self);
        // SAFETY: We just registered `event_key` with the type of `event`
        unsafe {
            self.trigger_dynamic_ref_with_caller(event_key, &mut event, caller);
        }
    }

    /// Triggers the given [`Event`] as a mutable reference, which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    #[track_caller]
    pub fn trigger_ref<E: Event>(&mut self, event: &mut E) {
        let event_key = E::register_event_key(self);
        // SAFETY: We just registered `event_key` with the type of `event`
        unsafe { self.trigger_dynamic_ref_with_caller(event_key, event, MaybeLocation::caller()) };
    }

    unsafe fn trigger_dynamic_ref_with_caller<E: Event>(
        &mut self,
        event_key: EventKey,
        event_data: &mut E,
        caller: MaybeLocation,
    ) {
        let mut world = DeferredWorld::from(self);
        // SAFETY: `event_data` is accessible as the type represented by `event_key`
        unsafe {
            world.trigger_observers_with_data::<_, ()>(
                event_key,
                None,
                None,
                core::iter::empty::<ComponentId>(),
                event_data,
                false,
                caller,
            );
        };
    }

    /// Triggers the given [`EntityEvent`] for the given `targets`, which will run any [`Observer`]s watching for it.
    ///
    /// While event types commonly implement [`Copy`],
    /// those that don't will be consumed and will no longer be accessible.
    /// If you need to use the event after triggering it, use [`World::trigger_targets_ref`] instead.
    #[track_caller]
    pub fn trigger_targets<E: EntityEvent>(&mut self, event: E, targets: impl TriggerTargets) {
        self.trigger_targets_with_caller(event, targets, MaybeLocation::caller());
    }

    pub(crate) fn trigger_targets_with_caller<E: EntityEvent>(
        &mut self,
        mut event: E,
        targets: impl TriggerTargets,
        caller: MaybeLocation,
    ) {
        let event_key = E::register_event_key(self);
        // SAFETY: We just registered `event_key` with the type of `event`
        unsafe {
            self.trigger_targets_dynamic_ref_with_caller(event_key, &mut event, targets, caller);
        }
    }

    /// Triggers the given [`EntityEvent`] as a mutable reference for the given `targets`,
    /// which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger_targets`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    #[track_caller]
    pub fn trigger_targets_ref<E: EntityEvent>(
        &mut self,
        event: &mut E,
        targets: impl TriggerTargets,
    ) {
        let event_key = E::register_event_key(self);
        // SAFETY: We just registered `event_key` with the type of `event`
        unsafe { self.trigger_targets_dynamic_ref(event_key, event, targets) };
    }

    /// Triggers the given [`EntityEvent`] for the given `targets`, which will run any [`Observer`]s watching for it.
    ///
    /// While event types commonly implement [`Copy`],
    /// those that don't will be consumed and will no longer be accessible.
    /// If you need to use the event after triggering it, use [`World::trigger_targets_dynamic_ref`] instead.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `event_data` is accessible as the type represented by `event_key`.
    #[track_caller]
    pub unsafe fn trigger_targets_dynamic<E: EntityEvent, Targets: TriggerTargets>(
        &mut self,
        event_key: EventKey,
        mut event_data: E,
        targets: Targets,
    ) {
        // SAFETY: `event_data` is accessible as the type represented by `event_key`
        unsafe {
            self.trigger_targets_dynamic_ref(event_key, &mut event_data, targets);
        };
    }

    /// Triggers the given [`EntityEvent`] as a mutable reference for the given `targets`,
    /// which will run any [`Observer`]s watching for it.
    ///
    /// Compared to [`World::trigger_targets_dynamic`], this method is most useful when it's necessary to check
    /// or use the event after it has been modified by observers.
    ///
    /// # Safety
    ///
    /// Caller must ensure that `event_data` is accessible as the type represented by `event_key`.
    #[track_caller]
    pub unsafe fn trigger_targets_dynamic_ref<E: EntityEvent, Targets: TriggerTargets>(
        &mut self,
        event_key: EventKey,
        event_data: &mut E,
        targets: Targets,
    ) {
        self.trigger_targets_dynamic_ref_with_caller(
            event_key,
            event_data,
            targets,
            MaybeLocation::caller(),
        );
    }

    /// # Safety
    ///
    /// See `trigger_targets_dynamic_ref`
    unsafe fn trigger_targets_dynamic_ref_with_caller<E: EntityEvent, Targets: TriggerTargets>(
        &mut self,
        event_key: EventKey,
        event_data: &mut E,
        targets: Targets,
        caller: MaybeLocation,
    ) {
        let mut world = DeferredWorld::from(self);
        let mut entity_targets = targets.entities().peekable();
        if entity_targets.peek().is_none() {
            // SAFETY: `event_data` is accessible as the type represented by `event_key`
            unsafe {
                world.trigger_observers_with_data::<_, E::Traversal>(
                    event_key,
                    None,
                    None,
                    targets.components(),
                    event_data,
                    false,
                    caller,
                );
            };
        } else {
            for target_entity in entity_targets {
                // SAFETY: `event_data` is accessible as the type represented by `event_key`
                unsafe {
                    world.trigger_observers_with_data::<_, E::Traversal>(
                        event_key,
                        Some(target_entity),
                        Some(target_entity),
                        targets.components(),
                        event_data,
                        E::AUTO_PROPAGATE,
                        caller,
                    );
                };
            }
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
                                            .components()
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

    use bevy_platform::collections::HashMap;
    use bevy_ptr::OwningPtr;

    use crate::{
        change_detection::MaybeLocation,
        component::ComponentId,
        entity_disabling::Internal,
        observer::{Observer, Replace},
        prelude::*,
        traversal::Traversal,
    };

    #[derive(Component)]
    struct A;

    #[derive(Component)]
    struct B;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct S;

    #[derive(EntityEvent)]
    struct EventA;

    #[derive(EntityEvent)]
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

    #[derive(Component)]
    struct ChildOf(Entity);

    impl<D> Traversal<D> for &'_ ChildOf {
        fn traverse(item: Self::Item<'_, '_>, _: &D) -> Option<Entity> {
            Some(item.0)
        }
    }

    #[derive(Component, EntityEvent)]
    #[entity_event(traversal = &'static ChildOf, auto_propagate)]
    struct EventPropagating;

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
            |obs: On<Add, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_a");
                commands.entity(obs.target()).insert(B);
            },
        );
        world.add_observer(
            |obs: On<Remove, A>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("remove_a");
                commands.entity(obs.target()).remove::<B>();
            },
        );

        world.add_observer(
            |obs: On<Add, B>, mut res: ResMut<Order>, mut commands: Commands| {
                res.observed("add_b");
                commands.entity(obs.target()).remove::<A>();
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

        world.add_observer(|mut trigger: On<EventWithData>| trigger.event_mut().counter += 1);
        world.add_observer(|mut trigger: On<EventWithData>| trigger.event_mut().counter += 2);
        world.add_observer(|mut trigger: On<EventWithData>| trigger.event_mut().counter += 4);

        let mut event = EventWithData { counter: 0 };
        world.trigger_ref(&mut event);
        assert_eq!(7, event.counter);
    }

    #[test]
    fn observer_trigger_targets_ref() {
        let mut world = World::new();

        world.add_observer(|mut trigger: On<EventWithData, A>| {
            trigger.event_mut().counter += 1;
        });
        world.add_observer(|mut trigger: On<EventWithData, B>| {
            trigger.event_mut().counter += 2;
        });
        world.add_observer(|mut trigger: On<EventWithData, A>| {
            trigger.event_mut().counter += 4;
        });

        let mut event = EventWithData { counter: 0 };
        let component_a = world.register_component::<A>();
        world.trigger_targets_ref(&mut event, component_a);
        assert_eq!(5, event.counter);
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
                .query_filtered::<&Observer, Allows<Internal>>()
                .query(&world)
                .count(),
            2
        );
    }

    #[test]
    fn observer_multiple_events() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let on_remove = Remove::register_event_key(&mut world);
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
    fn observer_no_target() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let system: fn(On<EventA>) = |_| {
            panic!("Trigger routed to non-targeted entity.");
        };
        world.spawn_empty().observe(system);
        world.add_observer(move |obs: On<EventA>, mut res: ResMut<Order>| {
            assert_eq!(obs.target(), Entity::PLACEHOLDER);
            res.observed("event_a");
        });

        world.trigger(EventA);
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_entity_routing() {
        let mut world = World::new();
        world.init_resource::<Order>();

        let system: fn(On<EventA>) = |_| {
            panic!("Trigger routed to non-targeted entity.");
        };

        world.spawn_empty().observe(system);
        let entity = world
            .spawn_empty()
            .observe(|_: On<EventA>, mut res: ResMut<Order>| res.observed("a_1"))
            .id();
        world.add_observer(move |obs: On<EventA>, mut res: ResMut<Order>| {
            assert_eq!(obs.target(), entity);
            res.observed("a_2");
        });

        world.trigger_targets(EventA, entity);
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
            .observe(|_: On<EventA, A>, mut res: ResMut<R>| res.0 += 1)
            .id();
        // targets (entity_2, B)
        let entity_2 = world
            .spawn_empty()
            .observe(|_: On<EventA, B>, mut res: ResMut<R>| res.0 += 10)
            .id();
        // targets any entity or component
        world.add_observer(|_: On<EventA>, mut res: ResMut<R>| res.0 += 100);
        // targets any entity, and components A or B
        world.add_observer(|_: On<EventA, (A, B)>, mut res: ResMut<R>| res.0 += 1000);
        // test all tuples
        world.add_observer(|_: On<EventA, (A, B, (A, B))>, mut res: ResMut<R>| res.0 += 10000);
        world.add_observer(
            |_: On<EventA, (A, B, (A, B), ((A, B), (A, B)))>, mut res: ResMut<R>| {
                res.0 += 100000;
            },
        );
        world.add_observer(
            |_: On<EventA, (A, B, (A, B), (B, A), (A, B, ((A, B), (B, A))))>,
             mut res: ResMut<R>| res.0 += 1000000,
        );

        // trigger for an entity and a component
        world.trigger_targets(EventA, (entity_1, component_a));
        // only observer that doesn't trigger is the one only watching entity_2
        assert_eq!(1111101, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger for both entities, but no components: trigger once per entity target
        world.trigger_targets(EventA, (entity_1, entity_2));
        // only the observer that doesn't require components triggers - once per entity
        assert_eq!(200, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger for both components, but no entities: trigger once
        world.trigger_targets(EventA, (component_a, component_b));
        // all component observers trigger, entities are not observed
        assert_eq!(1111100, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger for both entities and both components: trigger once per entity target
        // we only get 2222211 because a given observer can trigger only once per entity target
        world.trigger_targets(EventA, ((component_a, component_b), (entity_1, entity_2)));
        assert_eq!(2222211, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger to test complex tuples: (A, B, (A, B))
        world.trigger_targets(
            EventA,
            (component_a, component_b, (component_a, component_b)),
        );
        // the duplicate components in the tuple don't cause multiple triggers
        assert_eq!(1111100, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger to test complex tuples: (A, B, (A, B), ((A, B), (A, B)))
        world.trigger_targets(
            EventA,
            (
                component_a,
                component_b,
                (component_a, component_b),
                ((component_a, component_b), (component_a, component_b)),
            ),
        );
        // the duplicate components in the tuple don't cause multiple triggers
        assert_eq!(1111100, world.resource::<R>().0);
        world.resource_mut::<R>().0 = 0;

        // trigger to test the most complex tuple: (A, B, (A, B), (B, A), (A, B, ((A, B), (B, A))))
        world.trigger_targets(
            EventA,
            (
                component_a,
                component_b,
                (component_a, component_b),
                (component_b, component_a),
                (
                    component_a,
                    component_b,
                    ((component_a, component_b), (component_b, component_a)),
                ),
            ),
        );
        // the duplicate components in the tuple don't cause multiple triggers
        assert_eq!(1111100, world.resource::<R>().0);
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
        let entity = entity.flush();

        world.trigger_targets(EventA, entity);
        assert_eq!(vec!["event_a"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_dynamic_trigger() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let event_a = Remove::register_event_key(&mut world);

        // SAFETY: we registered `event_a` above and it matches the type of EventA
        let observe = unsafe {
            Observer::with_dynamic_runner(|mut world, _trigger, _ptr, _propagate| {
                world.resource_mut::<Order>().observed("event_a");
            })
            .with_event_key(event_a)
        };
        world.spawn(observe);

        world.commands().queue(move |world: &mut World| {
            // SAFETY: we registered `event_a` above and it matches the type of EventA
            unsafe { world.trigger_targets_dynamic(event_a, EventA, ()) };
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
            move |trigger: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("parent");

                assert_eq!(trigger.target(), parent);
                assert_eq!(trigger.original_target(), child);
            },
        );

        world.entity_mut(child).observe(
            move |trigger: On<EventPropagating>, mut res: ResMut<Order>| {
                res.observed("child");
                assert_eq!(trigger.target(), child);
                assert_eq!(trigger.original_target(), child);
            },
        );

        world.trigger_targets(EventPropagating, child);

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

        world.trigger_targets(EventPropagating, [child, child]);

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

        world.trigger_targets(EventPropagating, [child, parent]);
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
            .observe(
                |mut trigger: On<EventPropagating>, mut res: ResMut<Order>| {
                    res.observed("child");
                    trigger.propagate(false);
                },
            )
            .id();

        world.trigger_targets(EventPropagating, child);

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

        world.trigger_targets(EventPropagating, [child_a, child_b]);

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

        world.trigger_targets(EventPropagating, entity);
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
            .observe(
                |mut trigger: On<EventPropagating>, mut res: ResMut<Order>| {
                    res.observed("child_a");
                    trigger.propagate(false);
                },
            )
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

        world.trigger_targets(EventPropagating, [child_a, child_b]);

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

        world.trigger_targets(EventPropagating, child);

        assert_eq!(vec!["event", "event", "event"], world.resource::<Order>().0);
    }

    #[test]
    fn observer_propagating_world_skipping() {
        let mut world = World::new();
        world.init_resource::<Order>();

        world.add_observer(
            |trigger: On<EventPropagating>, query: Query<&A>, mut res: ResMut<Order>| {
                if query.get(trigger.target()).is_ok() {
                    res.observed("event");
                }
            },
        );

        let grandparent = world.spawn(A).id();
        let parent = world.spawn(ChildOf(grandparent)).id();
        let child = world.spawn((A, ChildOf(parent))).id();

        world.trigger_targets(EventPropagating, child);

        assert_eq!(vec!["event", "event"], world.resource::<Order>().0);
    }

    // Originally for https://github.com/bevyengine/bevy/issues/18452
    #[test]
    fn observer_modifies_relationship() {
        fn on_add(trigger: On<Add, A>, mut commands: Commands) {
            commands
                .entity(trigger.target())
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
        world.add_observer(move |trigger: On<EventA>| {
            assert_eq!(trigger.caller(), caller);
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
        world.add_observer(move |trigger: On<Add, Component>| {
            assert_eq!(trigger.caller(), caller);
        });
        world.add_observer(move |trigger: On<Remove, Component>| {
            assert_eq!(trigger.caller(), caller);
        });
        world.commands().spawn(Component).clear();
    }

    #[test]
    fn observer_triggered_components() {
        #[derive(Resource, Default)]
        struct Counter(HashMap<ComponentId, usize>);

        let mut world = World::new();
        world.init_resource::<Counter>();
        let a_id = world.register_component::<A>();
        let b_id = world.register_component::<B>();

        world.add_observer(
            |trigger: On<EventA, (A, B)>, mut counter: ResMut<Counter>| {
                for &component in trigger.components() {
                    *counter.0.entry(component).or_default() += 1;
                }
            },
        );

        world.trigger_targets(EventA, [a_id, b_id]);
        world.trigger_targets(EventA, a_id);
        world.trigger_targets(EventA, b_id);
        world.trigger_targets(EventA, [a_id, b_id]);
        world.trigger_targets(EventA, a_id);

        let counter = world.resource::<Counter>();
        assert_eq!(4, *counter.0.get(&a_id).unwrap());
        assert_eq!(3, *counter.0.get(&b_id).unwrap());
    }

    #[test]
    fn observer_watch_entities() {
        let mut world = World::new();
        world.init_resource::<Order>();
        let entities = world
            .spawn_batch(core::iter::repeat_n((), 4))
            .collect::<Vec<_>>();
        let observer = Observer::new(|_: On<EventA>, mut order: ResMut<Order>| {
            order.observed("a");
        });
        world.spawn(observer.with_entities(entities.iter().copied().take(2)));

        world.trigger_targets(EventA, [entities[0], entities[1]]);
        assert_eq!(vec!["a", "a"], world.resource::<Order>().0);
        world.trigger_targets(EventA, [entities[2], entities[3]]);
        assert_eq!(vec!["a", "a"], world.resource::<Order>().0);
    }
}
