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
    component::{
        ComponentCloneBehavior, ComponentId, Mutable, RequiredComponentsRegistrator, StorageType,
    },
    entity::Entity,
    entity_disabling::Internal,
    error::{ErrorContext, ErrorHandler},
    event::{Event, EventKey},
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
/// Observers watch for a "trigger" of a specific [`Event`]. An event can be triggered on the [`World`]
/// by calling [`World::trigger`]. It can also be queued up as a [`Command`] using [`Commands::trigger`].
///
/// When a [`World`] triggers an [`Event`], it will immediately run every [`Observer`] that watches for that [`Event`].
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
/// world.add_observer(|event: On<Speak>| {
///     println!("{}", event.message);
/// });
///
/// world.trigger(Speak {
///     message: "Hello!".into(),
/// });
/// ```
///
/// Notice that we used [`World::add_observer`]. This is just a shorthand for spawning an [`Entity`] with an [`Observer`] manually:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct Speak;
/// // These are functionally the same:
/// world.add_observer(|event: On<Speak>| {});
/// world.spawn(Observer::new(|event: On<Speak>| {}));
/// ```
///
/// Observers are a specialized [`System`] called an [`ObserverSystem`]. The first parameter must be [`On`], which provides access
/// to the [`Event`], the [`Trigger`], and some additional execution context.
///
/// Because they are systems, they can access arbitrary [`World`] data by adding [`SystemParam`]s:
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # let mut world = World::default();
/// # #[derive(Event)]
/// # struct PrintNames;
/// # #[derive(Component, Debug)]
/// # struct Name;
/// world.add_observer(|event: On<PrintNames>, names: Query<&Name>| {
///     for name in &names {
///         println!("{name:?}");
///     }
/// });
/// ```
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
/// world.add_observer(|event: On<SpawnThing>, mut commands: Commands| {
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
/// world.add_observer(|event: On<A>, mut commands: Commands| {
///     commands.trigger(B);
/// });
/// ```
///
/// When the commands are flushed (including these "nested triggers") they will be
/// recursively evaluated until there are no commands left, meaning nested triggers all
/// evaluate at the same time!
///
/// ## Event [`Trigger`] behavior
///
/// Each [`Event`] defines a [`Trigger`] behavior, which determines _which_ observers will run for the given [`Event`] and _how_ they will be run.
///
/// [`Event`] by default (when derived) uses [`GlobalTrigger`](crate::event::GlobalTrigger). When it is triggered any [`Observer`] watching for it will be run.
///
/// ## Event sub-types
///
/// There are some built-in specialized [`Event`] types with custom [`Trigger`] logic:
///
/// - [`EntityEvent`] / [`EntityTrigger`](crate::event::EntityTrigger): An [`Event`] that targets a _specific_ entity. This also has opt-in support for
///   "event bubbling" behavior. See [`EntityEvent`] for details.
/// - [`EntityComponentsTrigger`](crate::event::EntityComponentsTrigger): An [`Event`] that targets an entity _and_ one or more components on that entity.
///   This is used for [component lifecycle events](crate::lifecycle).
///
/// You can also define your own!
///
/// ## Observer execution timing
///
/// Observers triggered via [`World::trigger`] are evaluated immediately, as are all commands they queue up.
///
/// Observers triggered via [`Commands::trigger`] are evaluated at the next sync point in the ECS schedule, just like any other [`Command`].
///
/// To control the relative ordering of observer trigger commands sent from different systems,
/// order the systems in the schedule relative to each other.
///
/// Currently, Bevy does not provide [a way to specify the relative ordering of observers](https://github.com/bevyengine/bevy/issues/14890)
/// watching for the same event. Their ordering is considered to be arbitrary. It is recommended to make no
/// assumptions about their execution order.
///
/// Commands sent by observers are [currently not immediately applied](https://github.com/bevyengine/bevy/issues/19569).
/// Instead, all queued observers will run, and then all of the commands from those observers will be applied.
///
/// ## [`ObservedBy`]
///
/// When entities are observed, they will receive an [`ObservedBy`] component,
/// which will be updated to track the observers that are currently observing them.
///
/// ## Manual [`Observer`] target configuration
///
/// You can manually control the targets that an observer is watching by calling builder methods like [`Observer::with_entity`]
/// _before_ inserting the [`Observer`] component.
///
/// In general, it is better to use the [`EntityWorldMut::observe`] or [`EntityCommands::observe`] methods,
/// which spawns a new observer, and configures it to watch the entity it is called on.
///
/// ## Cleaning up observers
///
/// If an [`EntityEvent`] [`Observer`] targets specific entities, and all of those entities are despawned, the [`Observer`] entity will also be despawned.
/// This protects against observer "garbage" building up over time.
///
/// ## Component lifecycle events: Observers vs Hooks
///
/// It is important to note that observers, just like [hooks](crate::lifecycle::ComponentHooks),
/// can watch for and respond to [lifecycle](crate::lifecycle) events.
/// Unlike hooks, observers are not treated as an "innate" part of component behavior:
/// they can be added or removed at runtime, and multiple observers
/// can be registered for the same lifecycle event for the same component.
///
/// The ordering of hooks versus observers differs based on the lifecycle event in question:
///
/// - when adding components, hooks are evaluated first, then observers
/// - when removing components, observers are evaluated first, then hooks
///
/// This allows hooks to act as constructors and destructors for components,
/// as they always have the first and final say in the component's lifecycle.
///
/// ## Observer re-targeting
///
/// Currently, [observers cannot be retargeted after spawning](https://github.com/bevyengine/bevy/issues/19587):
/// despawn and respawn an observer as a workaround.
///
/// ## Internal observer cache
///
/// For more efficient observer triggering, Observers make use of the internal [`CachedObservers`](crate::observer::CachedObservers) storage.
/// In general, this is an implementation detail developers don't need to worry about, but it can be used when implementing custom [`Trigger`](crate::event::Trigger)
/// types, or to add "dynamic" observers for cases like scripting / modding.
///
/// [`SystemParam`]: crate::system::SystemParam
/// [`Trigger`]: crate::event::Trigger
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
    /// Creates a new [`Observer`], which defaults to a "global" observer. This means it will run _whenever_ an event of type `E` is triggered.
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

    /// Creates a new [`Observer`] with custom runner, this is mostly used for dynamic event observers
    pub fn with_dynamic_runner(runner: ObserverRunner) -> Self {
        Self {
            system: Box::new(IntoSystem::into_system(|| {})),
            descriptor: Default::default(),
            hook_on_add: |mut world, hook_context| {
                let default_error_handler = world.default_error_handler();
                world.commands().queue(move |world: &mut World| {
                    let entity = hook_context.entity;
                    if let Some(mut observe) = world.get_mut::<Observer>(entity) {
                        if observe.descriptor.event_keys.is_empty() {
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

    /// Observes the given `entity` (in addition to any entity already being observed).
    /// This will cause the [`Observer`] to run whenever an [`EntityEvent::event_target`] is the given `entity`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.watch_entity(entity);
        self
    }

    /// Observes the given `entities` (in addition to any entity already being observed).
    /// This will cause the [`Observer`] to run whenever an [`EntityEvent::event_target`] is any of the `entities`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn with_entities<I: IntoIterator<Item = Entity>>(mut self, entities: I) -> Self {
        self.watch_entities(entities);
        self
    }

    /// Observes the given `entity` (in addition to any entity already being observed).
    /// This will cause the [`Observer`] to run whenever an [`EntityEvent::event_target`] is the given `entity`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn watch_entity(&mut self, entity: Entity) {
        self.descriptor.entities.push(entity);
    }

    /// Observes the given `entity` (in addition to any entity already being observed).
    /// This will cause the [`Observer`] to run whenever an [`EntityEvent::event_target`] is any of the `entities`.
    /// Note that if this is called _after_ an [`Observer`] is spawned, it will produce no effects.
    pub fn watch_entities<I: IntoIterator<Item = Entity>>(&mut self, entities: I) {
        self.descriptor.entities.extend(entities);
    }

    /// Observes the given `component`. This will cause the [`Observer`] to run whenever the [`Event`] has
    /// an [`EntityComponentsTrigger`](crate::event::EntityComponentsTrigger) that targets the given `component`.
    pub fn with_component(mut self, component: ComponentId) -> Self {
        self.descriptor.components.push(component);
        self
    }

    /// Observes the given `event_key`. This will cause the [`Observer`] to run whenever an event with the given [`EventKey`]
    /// is triggered.
    /// # Safety
    /// The type of the `event_key` [`EventKey`] _must_ match the actual value
    /// of the event passed into the observer system.
    pub unsafe fn with_event_key(mut self, event_key: EventKey) -> Self {
        self.descriptor.event_keys.push(event_key);
        self
    }

    /// Sets the error handler to use for this observer.
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

    fn register_required_components(
        _component_id: ComponentId,
        required_components: &mut RequiredComponentsRegistrator,
    ) {
        required_components.register_required(Internal::default);
    }
}

/// Store information about what an [`Observer`] observes.
///
/// This information is stored inside of the [`Observer`] component,
#[derive(Default, Clone)]
pub struct ObserverDescriptor {
    /// The event keys the observer is watching.
    pub(super) event_keys: Vec<EventKey>,

    /// The components the observer is watching.
    pub(super) components: Vec<ComponentId>,

    /// The entities the observer is watching.
    pub(super) entities: Vec<Entity>,
}

impl ObserverDescriptor {
    /// Add the given `event_keys` to the descriptor.
    /// # Safety
    /// The type of each [`EventKey`] in `event_keys` _must_ match the actual value
    /// of the event passed into the observer.
    pub unsafe fn with_event_keys(mut self, event_keys: Vec<EventKey>) -> Self {
        self.event_keys = event_keys;
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

    /// Returns the `event_keys` that the observer is watching.
    pub fn event_keys(&self) -> &[EventKey] {
        &self.event_keys
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
        let event_key = world.register_event_key::<E>();
        let mut components = alloc::vec![];
        B::component_ids(&mut world.components_registrator(), &mut |id| {
            components.push(id);
        });
        if let Some(mut observer) = world.get_mut::<Observer>(entity) {
            observer.descriptor.event_keys.push(event_key);
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
