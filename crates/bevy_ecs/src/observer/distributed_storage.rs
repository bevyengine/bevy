//! Information about observers that is stored on the entities themselves.
//!
//! This allows for easier cleanup, better inspection, and more flexible querying.
//!
//! Each observer is associated with an entity, defined by the [`Observer`] component.
//! The [`Observer`] component contains the system that will be run when the observer is triggered,
//! and the [`ObserverDescriptor`] which contains information about what the observer is observing.
//!
//! When we watch entities, we add the [`ObservedBy`] component to those entities,
//! which links back to the observer entity.

use core::any::Any;

use crate::{
    component::{ComponentCloneBehavior, ComponentId, Mutable, StorageType},
    entity::Entity,
    error::{ErrorContext, ErrorHandler},
    lifecycle::{ComponentHook, HookContext},
    observer::{observer_system_runner, ObserverRunner},
    prelude::*,
    system::{IntoObserverSystem, ObserverSystem},
    world::DeferredWorld,
};
use alloc::boxed::Box;
use alloc::vec::Vec;
use bevy_utils::prelude::DebugName;

#[cfg(feature = "bevy_reflect")]
use crate::prelude::ReflectComponent;

/// An [`Observer`] system. Add this [`Component`] to an [`Entity`] to turn it into an "observer".
///
/// Observers listen for a "trigger" of a specific [`Event`]. An event can be triggered on the [`World`]
/// by calling [`World::trigger`], or if the event is an [`EntityEvent`], it can also be triggered for specific
/// entity targets using [`World::trigger_targets`].
///
/// Note that [`BufferedEvent`]s sent using [`EventReader`] and [`EventWriter`] are _not_ automatically triggered.
/// They must be triggered at a specific point in the schedule.
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
/// If the event is an [`EntityEvent`], it can be triggered for specific entities,
/// which will be passed to the [`Observer`]:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # let entity = world.spawn_empty().id();
/// #[derive(Event, EntityEvent)]
/// struct Explode;
///
/// world.add_observer(|trigger: On<Explode>, mut commands: Commands| {
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
/// # #[derive(Event, EntityEvent)]
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
/// # #[derive(Event, EntityEvent)]
/// # struct Explode;
/// world.entity_mut(e1).observe(|trigger: On<Explode>, mut commands: Commands| {
///     println!("Boom!");
///     commands.entity(trigger.target()).despawn();
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
/// # #[derive(Event, EntityEvent)]
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
    pub(crate) error_handler: Option<ErrorHandler>,
    pub(crate) system: Box<dyn AnyNamedSystem>,
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
            system: Box::new(IntoSystem::into_system(|| {})),
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
    /// The type of the `event` [`EventKey`] _must_ match the actual value
    /// of the event passed into the observer system.
    pub unsafe fn with_event(mut self, event: EventKey) -> Self {
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

    /// Returns the name of the [`Observer`]'s system .
    pub fn system_name(&self) -> DebugName {
        self.system.system_name()
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

/// Store information about what an [`Observer`] observes.
///
/// This information is stored inside of the [`Observer`] component,
#[derive(Default, Clone)]
pub struct ObserverDescriptor {
    /// The events the observer is watching.
    pub(super) events: Vec<EventKey>,

    /// The components the observer is watching.
    pub(super) components: Vec<ComponentId>,

    /// The entities the observer is watching.
    pub(super) entities: Vec<Entity>,
}

impl ObserverDescriptor {
    /// Add the given `events` to the descriptor.
    /// # Safety
    /// The type of each [`EventKey`] in `events` _must_ match the actual value
    /// of the event passed into the observer.
    pub unsafe fn with_events(mut self, events: Vec<EventKey>) -> Self {
        self.events = events;
        self
    }

    /// Add the given `components` to the descriptor.
    pub fn with_components(mut self, components: Vec<ComponentId>) -> Self {
        self.components = components;
        self
    }

    /// Add the given `entities` to the descriptor.
    pub fn with_entities(mut self, entities: Vec<Entity>) -> Self {
        self.entities = entities;
        self
    }

    /// Returns the `events` that the observer is watching.
    pub fn events(&self) -> &[EventKey] {
        &self.events
    }

    /// Returns the `components` that the observer is watching.
    pub fn components(&self) -> &[ComponentId] {
        &self.components
    }

    /// Returns the `entities` that the observer is watching.
    pub fn entities(&self) -> &[Entity] {
        &self.entities
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
        let event_key = E::register_event_key(world);
        let mut components = alloc::vec![];
        B::component_ids(&mut world.components_registrator(), &mut |id| {
            components.push(id);
        });
        if let Some(mut observer) = world.get_mut::<Observer>(entity) {
            observer.descriptor.events.push(event_key);
            observer.descriptor.components.extend(components);

            let system: &mut dyn Any = observer.system.as_mut();
            let system: *mut dyn ObserverSystem<E, B> = system.downcast_mut::<S>().unwrap();
            // SAFETY: World reference is exclusive and initialize does not touch system, so references do not alias
            unsafe {
                (*system).initialize(world);
            }
            world.register_observer(entity);
        }
    });
}

/// Tracks a list of entity observers for the [`Entity`] [`ObservedBy`] is added to.
#[derive(Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "bevy_reflect", reflect(Component, Debug))]
pub struct ObservedBy(pub(crate) Vec<Entity>);

impl ObservedBy {
    /// Provides a read-only reference to the list of entities observing this entity.
    pub fn get(&self) -> &[Entity] {
        &self.0
    }
}

impl Component for ObservedBy {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
    type Mutability = Mutable;

    fn on_remove() -> Option<ComponentHook> {
        Some(|mut world, HookContext { entity, .. }| {
            let observed_by = {
                let mut component = world.get_mut::<ObservedBy>(entity).unwrap();
                core::mem::take(&mut component.0)
            };
            for e in observed_by {
                let (total_entities, despawned_watched_entities) = {
                    let Ok(mut entity_mut) = world.get_entity_mut(e) else {
                        continue;
                    };
                    let Some(mut state) = entity_mut.get_mut::<Observer>() else {
                        continue;
                    };
                    state.despawned_watched_entities += 1;
                    (
                        state.descriptor.entities.len(),
                        state.despawned_watched_entities as usize,
                    )
                };

                // Despawn Observer if it has no more active sources.
                if total_entities == despawned_watched_entities {
                    world.commands().entity(e).despawn();
                }
            }
        })
    }

    fn clone_behavior() -> ComponentCloneBehavior {
        ComponentCloneBehavior::Ignore
    }
}

pub(crate) trait AnyNamedSystem: Any + Send + Sync + 'static {
    fn system_name(&self) -> DebugName;
}

impl<T: Any + System> AnyNamedSystem for T {
    fn system_name(&self) -> DebugName {
        self.name()
    }
}
