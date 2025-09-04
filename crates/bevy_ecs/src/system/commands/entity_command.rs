//! Contains the definition of the [`EntityCommand`] trait,
//! as well as the blanket implementation of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`EntityCommands`](crate::system::EntityCommands).

use alloc::vec::Vec;
use log::info;

use crate::{
    bundle::{Bundle, InsertMode},
    change_detection::MaybeLocation,
    component::{Component, ComponentId, ComponentInfo},
    entity::{Entity, EntityClonerBuilder, OptIn, OptOut},
    event::EntityEvent,
    relationship::RelationshipHookMode,
    system::IntoObserverSystem,
    world::{error::EntityMutableFetchError, EntityWorldMut, FromWorld},
};
use bevy_ptr::OwningPtr;

/// A command which gets executed for a given [`Entity`].
///
/// Should be used with [`EntityCommands::queue`](crate::system::EntityCommands::queue).
///
/// The `Out` generic parameter is the returned "output" of the command.
///
/// # Examples
///
/// ```
/// # use std::collections::HashSet;
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::system::EntityCommand;
/// #
/// # #[derive(Component, PartialEq)]
/// # struct Name(String);
/// # impl Name {
/// #   fn new(s: String) -> Self { Name(s) }
/// #   fn as_str(&self) -> &str { &self.0 }
/// # }
///
/// #[derive(Resource, Default)]
/// struct Counter(i64);
///
/// /// A `Command` which names an entity based on a global counter.
/// fn count_name(mut entity: EntityWorldMut) {
///     // Get the current value of the counter, and increment it for next time.
///     let i = {
///         let mut counter = entity.resource_mut::<Counter>();
///         let i = counter.0;
///         counter.0 += 1;
///         i
///     };
///     // Name the entity after the value of the counter.
///     entity.insert(Name::new(format!("Entity #{i}")));
/// }
///
/// // App creation boilerplate omitted...
/// # let mut world = World::new();
/// # world.init_resource::<Counter>();
/// #
/// # let mut setup_schedule = Schedule::default();
/// # setup_schedule.add_systems(setup);
/// # let mut assert_schedule = Schedule::default();
/// # assert_schedule.add_systems(assert_names);
/// #
/// # setup_schedule.run(&mut world);
/// # assert_schedule.run(&mut world);
///
/// fn setup(mut commands: Commands) {
///     commands.spawn_empty().queue(count_name);
///     commands.spawn_empty().queue(count_name);
/// }
///
/// fn assert_names(named: Query<&Name>) {
///     // We use a HashSet because we do not care about the order.
///     let names: HashSet<_> = named.iter().map(Name::as_str).collect();
///     assert_eq!(names, HashSet::from_iter(["Entity #0", "Entity #1"]));
/// }
/// ```
pub trait EntityCommand<Out = ()>: Send + 'static {
    /// Executes this command for the given [`Entity`].
    fn apply(self, entity: EntityWorldMut) -> Out;
}

/// An error that occurs when running an [`EntityCommand`] on a specific entity.
#[derive(thiserror::Error, Debug)]
pub enum EntityCommandError<E> {
    /// The entity this [`EntityCommand`] tried to run on could not be fetched.
    #[error(transparent)]
    EntityFetchError(#[from] EntityMutableFetchError),
    /// An error that occurred while running the [`EntityCommand`].
    #[error("{0}")]
    CommandFailed(E),
}

impl<Out, F> EntityCommand<Out> for F
where
    F: FnOnce(EntityWorldMut) -> Out + Send + 'static,
{
    fn apply(self, entity: EntityWorldMut) -> Out {
        self(entity)
    }
}

/// An [`EntityCommand`] that adds the components in a [`Bundle`] to an entity.
#[track_caller]
pub fn insert(bundle: impl Bundle, mode: InsertMode) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.insert_with_caller(bundle, mode, caller, RelationshipHookMode::Run);
    }
}

/// An [`EntityCommand`] that adds a dynamic component to an entity.
///
/// # Safety
///
/// - [`ComponentId`] must be from the same world as the target entity.
/// - `T` must have the same layout as the one passed during `component_id` creation.
#[track_caller]
pub unsafe fn insert_by_id<T: Send + 'static>(
    component_id: ComponentId,
    value: T,
    mode: InsertMode,
) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        // SAFETY:
        // - `component_id` safety is ensured by the caller
        // - `ptr` is valid within the `make` block
        OwningPtr::make(value, |ptr| unsafe {
            entity.insert_by_id_with_caller(
                component_id,
                ptr,
                mode,
                caller,
                RelationshipHookMode::Run,
            );
        });
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// the component's [`FromWorld`] implementation.
///
/// `T::from_world` will only be invoked if the component will actually be inserted.
/// In other words, `T::from_world` will *not* be invoked if `mode` is [`InsertMode::Keep`]
/// and the entity already has the component.
#[track_caller]
pub fn insert_from_world<T: Component + FromWorld>(mode: InsertMode) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        if !(mode == InsertMode::Keep && entity.contains::<T>()) {
            let value = entity.world_scope(|world| T::from_world(world));
            entity.insert_with_caller(value, mode, caller, RelationshipHookMode::Run);
        }
    }
}

/// An [`EntityCommand`] that adds a component to an entity using
/// some function that returns the component.
///
/// The function will only be invoked if the component will actually be inserted.
/// In other words, the function will *not* be invoked if `mode` is [`InsertMode::Keep`]
/// and the entity already has the component.
#[track_caller]
pub fn insert_with<T: Component, F>(component_fn: F, mode: InsertMode) -> impl EntityCommand
where
    F: FnOnce() -> T + Send + 'static,
{
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        if !(mode == InsertMode::Keep && entity.contains::<T>()) {
            let value = component_fn();
            entity.insert_with_caller(value, mode, caller, RelationshipHookMode::Run);
        }
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity.
#[track_caller]
pub fn remove<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes the components in a [`Bundle`] from an entity,
/// as well as the required components for each component removed.
#[track_caller]
pub fn remove_with_requires<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_with_requires_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that removes a dynamic component from an entity.
#[track_caller]
pub fn remove_by_id(component_id: ComponentId) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.remove_by_id_with_caller(component_id, caller);
    }
}

/// An [`EntityCommand`] that removes all components from an entity.
#[track_caller]
pub fn clear() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.clear_with_caller(caller);
    }
}

/// An [`EntityCommand`] that removes all components from an entity,
/// except for those in the given [`Bundle`].
#[track_caller]
pub fn retain<T: Bundle>() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.retain_with_caller::<T>(caller);
    }
}

/// An [`EntityCommand`] that despawns an entity.
///
/// # Note
///
/// This will also despawn the entities in any [`RelationshipTarget`](crate::relationship::RelationshipTarget)
/// that is configured to despawn descendants.
///
/// For example, this will recursively despawn [`Children`](crate::hierarchy::Children).
#[track_caller]
pub fn despawn() -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |entity: EntityWorldMut| {
        entity.despawn_with_caller(caller);
    }
}

/// An [`EntityCommand`] that creates an [`Observer`](crate::observer::Observer)
/// listening for events of type `E` targeting an entity
#[track_caller]
pub fn observe<E: EntityEvent, B: Bundle, M>(
    observer: impl IntoObserverSystem<E, B, M>,
) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        entity.observe_with_caller(observer, caller);
    }
}

/// An [`EntityCommand`] that sends an [`EntityEvent`] targeting an entity.
///
/// This will run any [`Observer`](crate::observer::Observer) of the given [`EntityEvent`] watching the entity.
#[track_caller]
pub fn trigger(event: impl EntityEvent) -> impl EntityCommand {
    let caller = MaybeLocation::caller();
    move |mut entity: EntityWorldMut| {
        let id = entity.id();
        entity.world_scope(|world| {
            world.trigger_targets_with_caller(event, id, caller);
        });
    }
}

/// An [`EntityCommand`] that clones parts of an entity onto another entity,
/// configured through [`EntityClonerBuilder`].
///
/// This builder tries to clone every component from the source entity except
/// for components that were explicitly denied, for example by using the
/// [`deny`](EntityClonerBuilder<OptOut>::deny) method.
///
/// Required components are not considered by denied components and must be
/// explicitly denied as well if desired.
pub fn clone_with_opt_out(
    target: Entity,
    config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clone_with_opt_out(target, config);
    }
}

/// An [`EntityCommand`] that clones parts of an entity onto another entity,
/// configured through [`EntityClonerBuilder`].
///
/// This builder tries to clone every component that was explicitly allowed
/// from the source entity, for example by using the
/// [`allow`](EntityClonerBuilder<OptIn>::allow) method.
///
/// Required components are also cloned when the target entity does not contain them.
pub fn clone_with_opt_in(
    target: Entity,
    config: impl FnOnce(&mut EntityClonerBuilder<OptIn>) + Send + Sync + 'static,
) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clone_with_opt_in(target, config);
    }
}

/// An [`EntityCommand`] that clones the specified components of an entity
/// and inserts them into another entity.
pub fn clone_components<B: Bundle>(target: Entity) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.clone_components::<B>(target);
    }
}

/// An [`EntityCommand`] moves the specified components of this entity into another entity.
///
/// Components with [`Ignore`] clone behavior will not be moved, while components that
/// have a [`Custom`] clone behavior will be cloned using it and then removed from the source entity.
/// All other components will be moved without any other special handling.
///
/// Note that this will trigger `on_remove` hooks/observers on this entity and `on_insert`/`on_add` hooks/observers on the target entity.
///
/// # Panics
///
/// The command will panic when applied if the target entity does not exist.
///
/// [`Ignore`]: crate::component::ComponentCloneBehavior::Ignore
/// [`Custom`]: crate::component::ComponentCloneBehavior::Custom
pub fn move_components<B: Bundle>(target: Entity) -> impl EntityCommand {
    move |mut entity: EntityWorldMut| {
        entity.move_components::<B>(target);
    }
}

/// An [`EntityCommand`] that logs the components of an entity.
pub fn log_components() -> impl EntityCommand {
    move |entity: EntityWorldMut| {
        let debug_infos: Vec<_> = entity
            .world()
            .inspect_entity(entity.id())
            .expect("Entity existence is verified before an EntityCommand is executed")
            .map(ComponentInfo::name)
            .collect();
        info!("Entity {}: {debug_infos:?}", entity.id());
    }
}
