use alloc::{boxed::Box, vec};
use core::any::Any;

use crate::{
    component::{ComponentId, Mutable, StorageType},
    error::{ErrorContext, ErrorHandler},
    lifecycle::{ComponentHook, HookContext},
    observer::{ObserverDescriptor, ObserverTrigger},
    prelude::*,
    query::DebugCheckedUnwrap,
    system::{IntoObserverSystem, ObserverSystem},
    world::DeferredWorld,
};
use bevy_ptr::PtrMut;

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
/// The simplest usage of the observer pattern looks like this:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// #[derive(Event)]
/// struct Speak {
///     message: String,
/// }
///
/// world.add_observer(|trigger: On<Speak>| {
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
/// world.add_observer(|trigger: On<Speak>| {});
/// world.spawn(Observer::new(|trigger: On<Speak>| {}));
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
/// world.add_observer(|trigger: On<PrintNames>, names: Query<&Name>| {
///     for name in &names {
///         println!("{name:?}");
///     }
/// });
/// ```
///
/// Note that [`On`] must always be the first parameter.
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
/// world.add_observer(|trigger: On<SpawnThing>, mut commands: Commands| {
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
/// world.add_observer(|trigger: On<A>, mut commands: Commands| {
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
/// world.add_observer(|trigger: On<Explode>, mut commands: Commands| {
///     println!("Entity {} goes BOOM!", trigger.target().unwrap());
///     commands.entity(trigger.target().unwrap()).despawn();
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
/// world.entity_mut(e1).observe(|trigger: On<Explode>, mut commands: Commands| {
///     println!("Boom!");
///     commands.entity(trigger.target().unwrap()).despawn();
/// });
///
/// world.entity_mut(e2).observe(|trigger: On<Explode>, mut commands: Commands| {
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
/// let mut observer = Observer::new(|trigger: On<Explode>| {});
/// observer.watch_entity(entity);
/// world.spawn(observer);
/// ```
///
/// Note that the [`Observer`] component is not added to the entity it is observing. Observers should always be their own entities!
///
/// You can call [`Observer::watch_entity`] more than once, which allows you to watch multiple entities with the same [`Observer`].
/// serves as the "source of truth" of the observer.
///
/// [`SystemParam`]: crate::system::SystemParam
pub struct Observer {
    hook_on_add: ComponentHook,
    error_handler: Option<ErrorHandler>,
    system: Box<dyn Any + Send + Sync + 'static>,
    pub(crate) descriptor: ObserverDescriptor,
    pub(crate) last_trigger_id: u32,
    pub(crate) despawned_watched_entities: u32,
    pub(crate) runner: ObserverRunner,
}

impl Observer {
    /// Creates a new [`Observer`], which defaults to a "global" observer. This means it will run whenever the event `E` is triggered
    /// for _any_ entity (or no entity).
    ///
    /// # Panics
    ///
    /// Panics if the given system is an exclusive system.
    pub fn new<E: Event, B: Bundle, M, I: IntoObserverSystem<E, B, M>>(system: I) -> Self {
        let system = Box::new(IntoObserverSystem::into_system(system));
        assert!(
            !system.is_exclusive(),
            concat!(
                "Exclusive system `{}` may not be used as observer.\n",
                "Instead of `&mut World`, use either `DeferredWorld` if you do not need structural changes, or `Commands` if you do."
            ),
            system.name()
        );
        Self {
            system,
            descriptor: Default::default(),
            hook_on_add: hook_on_add::<E, B, I::System>,
            error_handler: None,
            runner: observer_system_runner::<E, B, I::System>,
            despawned_watched_entities: 0,
            last_trigger_id: 0,
        }
    }

    /// Creates a new [`Observer`] with custom runner, this is mostly used for dynamic event observer
    pub fn with_dynamic_runner(runner: ObserverRunner) -> Self {
        Self {
            system: Box::new(|| {}),
            descriptor: Default::default(),
            hook_on_add: |mut world, hook_context| {
                let default_error_handler = world.default_error_handler();
                world.commands().queue(move |world: &mut World| {
                    let entity = hook_context.entity;
                    if let Some(mut observe) = world.get_mut::<Observer>(entity) {
                        if observe.descriptor.events.is_empty() {
                            return;
                        }
                        if observe.error_handler.is_none() {
                            observe.error_handler = Some(default_error_handler);
                        }
                        world.register_observer(entity);
                    }
                });
            },
            error_handler: None,
            runner,
            despawned_watched_entities: 0,
            last_trigger_id: 0,
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
    fn on_remove() -> Option<ComponentHook> {
        Some(|mut world, HookContext { entity, .. }| {
            let descriptor = core::mem::take(
                &mut world
                    .entity_mut(entity)
                    .get_mut::<Self>()
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
    // SAFETY: Observer was triggered so must have an `Observer`
    let mut state = unsafe { observer_cell.get_mut::<Observer>().debug_checked_unwrap() };

    // TODO: Move this check into the observer cache to avoid dynamic dispatch
    let last_trigger = world.last_trigger_id();
    if state.last_trigger_id == last_trigger {
        return;
    }
    state.last_trigger_id = last_trigger;

    let trigger: On<E, B> = On::new(
        // SAFETY: Caller ensures `ptr` is castable to `&mut T`
        unsafe { ptr.deref_mut() },
        propagate,
        observer_trigger,
    );

    // SAFETY:
    // - observer was triggered so must have an `Observer` component.
    // - observer cannot be dropped or mutated until after the system pointer is already dropped.
    let system: *mut dyn ObserverSystem<E, B> = unsafe {
        let system = state.system.downcast_mut::<S>().debug_checked_unwrap();
        &mut *system
    };

    // SAFETY:
    // - there are no outstanding references to world except a private component
    // - system is an `ObserverSystem` so won't mutate world beyond the access of a `DeferredWorld`
    //   and is never exclusive
    // - system is the same type erased system from above
    unsafe {
        // Always refresh hotpatch pointers
        // There's no guarantee that the `HotPatched` event would still be there once the observer is triggered.
        #[cfg(feature = "hotpatching")]
        (*system).refresh_hotpatch();

        match (*system).validate_param_unsafe(world) {
            Ok(()) => {
                if let Err(err) = (*system).run_unsafe(trigger, world) {
                    let handler = state
                        .error_handler
                        .unwrap_or_else(|| world.default_error_handler());
                    handler(
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
                    let handler = state
                        .error_handler
                        .unwrap_or_else(|| world.default_error_handler());
                    handler(
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

/// A [`ComponentHook`] used by [`Observer`] to handle its [`on-add`](`crate::lifecycle::ComponentHooks::on_add`).
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
        let mut components = vec![];
        B::component_ids(&mut world.components_registrator(), &mut |id| {
            components.push(id);
        });
        if let Some(mut observe) = world.get_mut::<Observer>(entity) {
            observe.descriptor.events.push(event_id);
            observe.descriptor.components.extend(components);

            let system: *mut dyn ObserverSystem<E, B> = observe.system.downcast_mut::<S>().unwrap();
            // SAFETY: World reference is exclusive and initialize does not touch system, so references do not alias
            unsafe {
                (*system).initialize(world);
            }
            world.register_observer(entity);
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        error::{ignore, DefaultErrorHandler},
        event::Event,
        observer::On,
    };

    #[derive(Event)]
    struct TriggerEvent;

    #[test]
    #[should_panic(expected = "I failed!")]
    fn test_fallible_observer() {
        fn system(_: On<TriggerEvent>) -> Result {
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

        fn system(_: On<TriggerEvent>, mut ran: ResMut<Ran>) -> Result {
            ran.0 = true;
            Err("I failed!".into())
        }

        // Using observer error handler
        let mut world = World::default();
        world.init_resource::<Ran>();
        world.spawn(Observer::new(system).with_error_handler(ignore));
        world.trigger(TriggerEvent);
        assert!(world.resource::<Ran>().0);

        // Using world error handler
        let mut world = World::default();
        world.init_resource::<Ran>();
        world.spawn(Observer::new(system));
        // Test that the correct handler is used when the observer was added
        // before the default handler
        world.insert_resource(DefaultErrorHandler(ignore));
        world.trigger(TriggerEvent);
        assert!(world.resource::<Ran>().0);
    }

    #[test]
    #[should_panic(
        expected = "Exclusive system `bevy_ecs::observer::runner::tests::exclusive_system_cannot_be_observer::system` may not be used as observer.\nInstead of `&mut World`, use either `DeferredWorld` if you do not need structural changes, or `Commands` if you do."
    )]
    fn exclusive_system_cannot_be_observer() {
        fn system(_: On<TriggerEvent>, _world: &mut World) {}
        let mut world = World::default();
        world.add_observer(system);
    }
}
