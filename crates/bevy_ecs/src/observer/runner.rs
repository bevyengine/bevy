use alloc::{boxed::Box, vec, vec::Vec};
use core::any::Any;

use crate::{
    component::{ComponentHook, ComponentId, HookContext, Mutable, StorageType},
    error::{default_error_handler, ErrorContext},
    observer::{ObserverDescriptor, ObserverTrigger},
    prelude::*,
    query::DebugCheckedUnwrap,
    system::{IntoObserverSystem, ObserverSystem},
    world::DeferredWorld,
};
use bevy_ptr::PtrMut;

/// Contains [`Observer`] information. This defines how a given observer behaves. It is the
/// "source of truth" for a given observer entity's behavior.
pub struct ObserverState {
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) runner: ObserverRunner,
    pub(crate) last_trigger_id: u32,
    pub(crate) despawned_watched_entities: u32,
}

impl Default for ObserverState {
    fn default() -> Self {
        Self {
            runner: |_, _, _, _| {},
            last_trigger_id: 0,
            despawned_watched_entities: 0,
            descriptor: Default::default(),
        }
    }
}

impl ObserverState {
    /// Observe the given `event`. This will cause the [`Observer`] to run whenever an event with the given [`ComponentId`]
    /// is triggered.
    pub fn with_event(mut self, event: ComponentId) -> Self {
        self.descriptor.events.push(event);
        self
    }

    /// Observe the given event list. This will cause the [`Observer`] to run whenever an event with any of the given [`ComponentId`]s
    /// is triggered.
    pub fn with_events(mut self, events: impl IntoIterator<Item = ComponentId>) -> Self {
        self.descriptor.events.extend(events);
        self
    }

    /// Observe the given [`Entity`] list. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for any [`Entity`] target in the list.
    pub fn with_entities(mut self, entities: impl IntoIterator<Item = Entity>) -> Self {
        self.descriptor.entities.extend(entities);
        self
    }

    /// Observe the given [`ComponentId`] list. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for any [`ComponentId`] target in the list.
    pub fn with_components(mut self, components: impl IntoIterator<Item = ComponentId>) -> Self {
        self.descriptor.components.extend(components);
        self
    }
}

impl Component for ObserverState {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;

    fn on_add() -> Option<ComponentHook> {
        Some(|mut world, HookContext { entity, .. }| {
            world.commands().queue(move |world: &mut World| {
                world.register_observer(entity);
            });
        })
    }

    fn on_remove() -> Option<ComponentHook> {
        Some(|mut world, HookContext { entity, .. }| {
            let descriptor = core::mem::take(
                &mut world
                    .entity_mut(entity)
                    .get_mut::<ObserverState>()
                    .unwrap()
                    .as_mut()
                    .descriptor,
            );
            world.commands().queue(move |world: &mut World| {
                world.unregister_observer(entity, descriptor);
            });
        })
    }
}

/// Type for function that is run when an observer is triggered.
///
/// Typically refers to the default runner that runs the system stored in the associated [`Observer`] component,
/// but can be overridden for custom behavior.
pub type ObserverRunner = fn(DeferredWorld, ObserverTrigger, PtrMut, propagate: &mut bool);

/// An [`Observer`] system. Add this [`Component`] to an [`Entity`] to turn it into an "observer".
///
/// Observers listen for a "trigger" of a specific [`Event`]. Events are triggered by calling [`World::trigger`] or [`World::trigger_targets`].
///
/// Note that "buffered" events sent using [`EventReader`] and [`EventWriter`] are _not_ automatically triggered. They must be triggered at a specific
/// point in the schedule.
///
/// # Usage
///
/// The simplest usage
/// of the observer pattern looks like this:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// #[derive(Event)]
/// struct Speak {
///     message: String,
/// }
///
/// world.add_observer(|trigger: Trigger<Speak>| {
///     println!("{}", trigger.event().message);
/// });
///
/// // Observers currently require a flush() to be registered. In the context of schedules,
/// // this will generally be done for you.
/// world.flush();
///
/// world.trigger(Speak {
///     message: "Hello!".into(),
/// });
/// ```
///
/// Notice that we used [`World::add_observer`]. This is just a shorthand for spawning an [`Observer`] manually:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct Speak;
/// // These are functionally the same:
/// world.add_observer(|trigger: Trigger<Speak>| {});
/// world.spawn(Observer::new(|trigger: Trigger<Speak>| {}));
/// ```
///
/// Observers are systems. They can access arbitrary [`World`] data by adding [`SystemParam`]s:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct PrintNames;
/// # #[derive(Component, Debug)]
/// # struct Name;
/// world.add_observer(|trigger: Trigger<PrintNames>, names: Query<&Name>| {
///     for name in &names {
///         println!("{name:?}");
///     }
/// });
/// ```
///
/// Note that [`Trigger`] must always be the first parameter.
///
/// You can also add [`Commands`], which means you can spawn new entities, insert new components, etc:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct SpawnThing;
/// # #[derive(Component, Debug)]
/// # struct Thing;
/// world.add_observer(|trigger: Trigger<SpawnThing>, mut commands: Commands| {
///     commands.spawn(Thing);
/// });
/// ```
///
/// Observers can also trigger new events:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct A;
/// # #[derive(Event)]
/// # struct B;
/// world.add_observer(|trigger: Trigger<A>, mut commands: Commands| {
///     commands.trigger(B);
/// });
/// ```
///
/// When the commands are flushed (including these "nested triggers") they will be
/// recursively evaluated until there are no commands left, meaning nested triggers all
/// evaluate at the same time!
///
/// Events can be triggered for entities, which will be passed to the [`Observer`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// #[derive(Event)]
/// struct Explode;
///
/// world.add_observer(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("Entity {} goes BOOM!", trigger.target());
///     commands.entity(trigger.target()).despawn();
/// });
///
/// world.flush();
///
/// world.trigger_targets(Explode, entity);
/// ```
///
/// You can trigger multiple entities at once:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let e1 = world.spawn_empty().id();
/// # let e2 = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// world.trigger_targets(Explode, [e1, e2]);
/// ```
///
/// Observers can also watch _specific_ entities, which enables you to assign entity-specific logic:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component, Debug)]
/// # struct Name(String);
/// # let mut world = World::default();
/// # let e1 = world.spawn_empty().id();
/// # let e2 = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// world.entity_mut(e1).observe(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("Boom!");
///     commands.entity(trigger.target()).despawn();
/// });
///
/// world.entity_mut(e2).observe(|trigger: Trigger<Explode>, mut commands: Commands| {
///     println!("The explosion fizzles! This entity is immune!");
/// });
/// ```
///
/// If all entities watched by a given [`Observer`] are despawned, the [`Observer`] entity will also be despawned.
/// This protects against observer "garbage" building up over time.
///
/// The examples above calling [`EntityWorldMut::observe`] to add entity-specific observer logic are (once again)
/// just shorthand for spawning an [`Observer`] directly:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// # #[derive(Event)]
/// # struct Explode;
/// let mut observer = Observer::new(|trigger: Trigger<Explode>| {});
/// observer.watch_entity(entity);
/// world.spawn(observer);
/// ```
///
/// Note that the [`Observer`] component is not added to the entity it is observing. Observers should always be their own entities!
///
/// You can call [`Observer::watch_entity`] more than once, which allows you to watch multiple entities with the same [`Observer`].
///
/// When first added, [`Observer`] will also create an [`ObserverState`] component, which registers the observer with the [`World`] and
/// serves as the "source of truth" of the observer.
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct Observer {
    system: Box<dyn Any + Send + Sync + 'static>,
    descriptor: ObserverDescriptor,
    hook_on_add: ComponentHook,
    error_handler: Option<fn(BevyError, ErrorContext)>,
}

impl Observer {
    /// Creates a new [`Observer`], which defaults to a "global" observer. This means it will run whenever the event `E` is triggered
    /// for _any_ entity (or no entity).
    pub fn new<E: Event, B: Bundle, M, I: IntoObserverSystem<E, B, M>>(system: I) -> Self {
        Self {
            system: Box::new(IntoObserverSystem::into_system(system)),
            descriptor: Default::default(),
            hook_on_add: hook_on_add::<E, B, I::System>,
            error_handler: None,
        }
    }

    /// Observe the given `entity`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for the `entity`.
    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.descriptor.entities.push(entity);
        self
    }

    /// Observe the given `entity`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// for the `entity`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn watch_entity(&mut self, entity: Entity) {
        self.descriptor.entities.push(entity);
    }

    /// Observe the given `component`. This will cause the [`Observer`] to run whenever the [`Event`] is triggered
    /// with the given component target.
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.descriptor.components.push(component);
        self
    }

    /// Observe the given `event`. This will cause the [`Observer`] to run whenever an event with the given [`ComponentId`]
    /// is triggered.
    /// # Safety
    /// The type of the `event` [`ComponentId`] _must_ match the actual value
    /// of the event passed into the observer system.
    pub unsafe fn with_event(mut self, event: ComponentId) -> Self {
        self.descriptor.events.push(event);
        self
    }

    /// Set the error handler to use for this observer.
    ///
    /// See the [`error` module-level documentation](crate::error) for more information.
    pub fn with_error_handler(mut self, error_handler: fn(BevyError, ErrorContext)) -> Self {
        self.error_handler = Some(error_handler);
        self
    }

    /// Returns the [`ObserverDescriptor`] for this [`Observer`].
    pub fn descriptor(&self) -> &ObserverDescriptor {
        &self.descriptor
    }
}

impl Component for Observer {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;
    fn on_add() -> Option<ComponentHook> {
        Some(|world, context| {
            let Some(observe) = world.get::<Self>(context.entity) else {
                return;
            };
            let hook = observe.hook_on_add;
            hook(world, context);
        })
    }
}

fn observer_system_runner<E: Event, B: Bundle, S: ObserverSystem<E, B>>(
    mut world: DeferredWorld,
    observer_trigger: ObserverTrigger,
    ptr: PtrMut,
    propagate: &mut bool,
) {
    let world = world.as_unsafe_world_cell();
    // SAFETY: Observer was triggered so must still exist in world
    let observer_cell = unsafe {
        world
            .get_entity(observer_trigger.observer)
            .debug_checked_unwrap()
    };
    // SAFETY: Observer was triggered so must have an `ObserverState`
    let mut state = unsafe {
        observer_cell
            .get_mut::<ObserverState>()
            .debug_checked_unwrap()
    };

    // TODO: Move this check into the observer cache to avoid dynamic dispatch
    let last_trigger = world.last_trigger_id();
    if state.last_trigger_id == last_trigger {
        return;
    }
    state.last_trigger_id = last_trigger;

    // SAFETY: Observer was triggered so must have an `Observer` component.
    let error_handler = unsafe {
        observer_cell
            .get::<Observer>()
            .debug_checked_unwrap()
            .error_handler
            .debug_checked_unwrap()
    };

    let trigger: Trigger<E, B> = Trigger::new(
        // SAFETY: Caller ensures `ptr` is castable to `&mut T`
        unsafe { ptr.deref_mut() },
        propagate,
        observer_trigger,
    );
    // SAFETY:
    // - observer was triggered so must have an `Observer` component.
    // - observer cannot be dropped or mutated until after the system pointer is already dropped.
    let system: *mut dyn ObserverSystem<E, B> = unsafe {
        let mut observe = observer_cell.get_mut::<Observer>().debug_checked_unwrap();
        let system = observe.system.downcast_mut::<S>().unwrap();
        &mut *system
    };

    // SAFETY:
    // - `update_archetype_component_access` is called first
    // - there are no outstanding references to world except a private component
    // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
    //   and is never exclusive
    // - system is the same type erased system from above
    unsafe {
        (*system).update_archetype_component_access(world);
        match (*system).validate_param_unsafe(world) {
            Ok(()) => {
                if let Err(err) = (*system).run_unsafe(trigger, world) {
                    error_handler(
                        err,
                        ErrorContext::Observer {
                            name: (*system).name(),
                            last_run: (*system).get_last_run(),
                        },
                    );
                };
                (*system).queue_deferred(world.into_deferred());
            }
            Err(e) => {
                if !e.skipped {
                    error_handler(
                        e.into(),
                        ErrorContext::Observer {
                            name: (*system).name(),
                            last_run: (*system).get_last_run(),
                        },
                    );
                }
            }
        }
    }
}

/// A [`ComponentHook`] used by [`Observer`] to handle its [`on-add`](`crate::component::ComponentHooks::on_add`).
///
/// This function exists separate from [`Observer`] to allow [`Observer`] to have its type parameters
/// erased.
///
/// The type parameters of this function _must_ match those used to create the [`Observer`].
/// As such, it is recommended to only use this function within the [`Observer::new`] method to
/// ensure type parameters match.
fn hook_on_add<E: Event, B: Bundle, S: ObserverSystem<E, B>>(
    mut world: DeferredWorld<'_>,
    HookContext { entity, .. }: HookContext,
) {
    world.commands().queue(move |world: &mut World| {
        let event_id = E::register_component_id(world);
        let mut components = Vec::new();
        B::component_ids(&mut world.components_registrator(), &mut |id| {
            components.push(id);
        });
        let mut descriptor = ObserverDescriptor {
            events: vec![event_id],
            components,
            ..Default::default()
        };

        let error_handler = default_error_handler();

        // Initialize System
        let system: *mut dyn ObserverSystem<E, B> =
            if let Some(mut observe) = world.get_mut::<Observer>(entity) {
                descriptor.merge(&observe.descriptor);
                if observe.error_handler.is_none() {
                    observe.error_handler = Some(error_handler);
                }
                let system = observe.system.downcast_mut::<S>().unwrap();
                &mut *system
            } else {
                return;
            };
        // SAFETY: World reference is exclusive and initialize does not touch system, so references do not alias
        unsafe {
            (*system).initialize(world);
        }

        {
            let mut entity = world.entity_mut(entity);
            if let crate::world::Entry::Vacant(entry) = entity.entry::<ObserverState>() {
                entry.insert(ObserverState {
                    descriptor,
                    runner: observer_system_runner::<E, B, S>,
                    ..Default::default()
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{event::Event, observer::Trigger};

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    #[should_panic(expected = "I failed!")]
    fn test_fallible_observer() {
        fn system(_: Trigger<TriggerEvent>) -> Result {
            Err("I failed!".into())
        }

        let mut world = World::default();
        world.add_observer(system);
        Schedule::default().run(&mut world);
        world.trigger(TriggerEvent);
    }

    #[test]
    fn test_fallible_observer_ignored_errors() {
        #[derive(Resource, Default)]
        struct Ran(bool);

        fn system(_: Trigger<TriggerEvent>, mut ran: ResMut<Ran>) -> Result {
            ran.0 = true;
            Err("I failed!".into())
        }

        let mut world = World::default();
        world.init_resource::<Ran>();
        let observer = Observer::new(system).with_error_handler(crate::error::ignore);
        world.spawn(observer);
        Schedule::default().run(&mut world);
        world.trigger(TriggerEvent);
        assert!(world.resource::<Ran>().0);
    }
}
