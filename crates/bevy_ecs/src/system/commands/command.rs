//! This module contains the definition of the [`Command`] trait, as well as
//! blanket implementations of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`Commands`](crate::system::Commands).

#[cfg(feature = "track_location")]
use core::panic::Location;

use crate::{
    bundle::{Bundle, InsertMode},
    entity::Entity,
    event::{Event, Events},
    observer::TriggerTargets,
    result::Result,
    schedule::ScheduleLabel,
    system::{CommandError, IntoSystem, Resource, SystemId, SystemInput},
    world::{FromWorld, SpawnBatchIter, World},
};

/// A [`World`] mutation.
///
/// Should be used with [`Commands::queue`](crate::system::Commands::queue).
///
/// # Usage
///
/// ```
/// # use bevy_ecs::prelude::*;
/// // Our world resource
/// #[derive(Resource, Default)]
/// struct Counter(u64);
///
/// // Our custom command
/// struct AddToCounter(u64);
///
/// impl Command for AddToCounter {
///     fn apply(self, world: &mut World) -> Result {
///         let mut counter = world.get_resource_or_insert_with(Counter::default);
///         counter.0 += self.0;
///         Ok(())
///     }
/// }
///
/// fn some_system(mut commands: Commands) {
///     commands.queue(AddToCounter(42));
/// }
/// ```
///
/// # Note on Generic
///
/// The `Marker` generic is necessary to allow multiple blanket implementations
/// of `Command` for closures, like so:
/// ```ignore (This would conflict with the real implementations)
/// impl Command for FnOnce(&mut World)
/// impl Command<Result> for FnOnce(&mut World) -> Result
/// ```
/// Without the generic, Rust would consider the two implementations to be conflicting.
///
/// The type used for `Marker` has no connection to anything else in the implementation.
pub trait Command<Marker = ()>: Send + 'static {
    /// Applies this command, causing it to mutate the provided `world`.
    ///
    /// This method is used to define what a command "does" when it is ultimately applied.
    /// Because this method takes `self`, you can store data or settings on the type that implements this trait.
    /// This data is set by the system or other source of the command, and then ultimately read in this method.
    fn apply(self, world: &mut World) -> Result;

    /// Applies this command and converts any resulting error into a [`CommandError`].
    ///
    /// Overwriting this method allows an implementor to return a `CommandError` directly
    /// and avoid erasing the error's type.
    fn apply_internal(self, world: &mut World) -> Result<(), CommandError>
    where
        Self: Sized,
    {
        match self.apply(world) {
            Ok(_) => Ok(()),
            Err(error) => Err(CommandError::CommandFailed(error)),
        }
    }

    /// Returns a new [`Command`] that, when applied, will apply the original command
    /// and pass any resulting error to the provided `error_handler`.
    fn with_error_handling(
        self,
        error_handler: Option<fn(&mut World, CommandError)>,
    ) -> impl Command
    where
        Self: Sized,
    {
        move |world: &mut World| {
            if let Err(error) = self.apply_internal(world) {
                // TODO: Pass the error to the global error handler if `error_handler` is `None`.
                let error_handler = error_handler.unwrap_or(|_, error| panic!("{error}"));
                error_handler(world, error);
            }
        }
    }
}

impl<F> Command for F
where
    F: FnOnce(&mut World) + Send + 'static,
{
    fn apply(self, world: &mut World) -> Result {
        self(world);
        Ok(())
    }
}

impl<F> Command<Result> for F
where
    F: FnOnce(&mut World) -> Result + Send + 'static,
{
    fn apply(self, world: &mut World) -> Result {
        self(world)
    }
}

/// Necessary to avoid erasing the type of the `CommandError` in
/// [`EntityCommand::with_entity`](crate::system::EntityCommand::with_entity).
impl<F> Command<(Result, CommandError)> for F
where
    F: FnOnce(&mut World) -> Result<(), CommandError> + Send + 'static,
{
    fn apply(self, world: &mut World) -> Result {
        self(world)?;
        Ok(())
    }

    fn apply_internal(self, world: &mut World) -> Result<(), CommandError> {
        self(world)
    }
}

/// A [`Command`] that consumes an iterator of [`Bundles`](Bundle) to spawn a series of entities.
///
/// This is more efficient than spawning the entities individually.
#[track_caller]
pub fn spawn_batch<I>(bundles_iter: I) -> impl Command
where
    I: IntoIterator + Send + Sync + 'static,
    I::Item: Bundle,
{
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |world: &mut World| {
        SpawnBatchIter::new(
            world,
            bundles_iter.into_iter(),
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will panic.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
pub fn insert_batch<I, B>(batch: I, mode: InsertMode) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.insert_batch_with_caller(
            batch,
            mode,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
/// If any entities do not exist in the world, this command will ignore them.
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
pub fn try_insert_batch<I, B>(batch: I, mode: InsertMode) -> impl Command
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle,
{
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.try_insert_batch_with_caller(
            batch,
            mode,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world using a value
/// created with the [`FromWorld`] trait.
#[track_caller]
pub fn init_resource<R: Resource + FromWorld>() -> impl Command {
    move |world: &mut World| {
        world.init_resource::<R>();
    }
}

/// A [`Command`] that inserts a [`Resource`] into the world.
#[track_caller]
pub fn insert_resource<R: Resource>(resource: R) -> impl Command {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |world: &mut World| {
        world.insert_resource_with_caller(
            resource,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}

/// A [`Command`] that removes a [`Resource`] from the world.
pub fn remove_resource<R: Resource>() -> impl Command {
    move |world: &mut World| {
        world.remove_resource::<R>();
    }
}

/// A [`Command`] that runs the system corresponding to the given [`SystemId`].
pub fn run_system<O: 'static>(id: SystemId<(), O>) -> impl Command<Result> {
    move |world: &mut World| -> Result {
        world.run_system(id)?;
        Ok(())
    }
}

/// A [`Command`] that runs the system corresponding to the given [`SystemId`]
/// and provides the given input value.
pub fn run_system_with<I>(id: SystemId<I>, input: I::Inner<'static>) -> impl Command<Result>
where
    I: SystemInput<Inner<'static>: Send> + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_with(id, input)?;
        Ok(())
    }
}

/// A [`Command`] that runs the given system,
/// caching its [`SystemId`] in a [`CachedSystemId`](crate::system::CachedSystemId) resource.
pub fn run_system_cached<M, S>(system: S) -> impl Command<Result>
where
    M: 'static,
    S: IntoSystem<(), (), M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_cached(system)?;
        Ok(())
    }
}

/// A [`Command`] that runs the given system with the given input value,
/// caching its [`SystemId`] in a [`CachedSystemId`](crate::system::CachedSystemId) resource.
pub fn run_system_cached_with<I, M, S>(system: S, input: I::Inner<'static>) -> impl Command<Result>
where
    I: SystemInput<Inner<'static>: Send> + Send + 'static,
    M: 'static,
    S: IntoSystem<I, (), M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.run_system_cached_with(system, input)?;
        Ok(())
    }
}

/// A [`Command`] that removes a system previously registered with
/// [`Commands::register_system`](crate::system::Commands::register_system) or
/// [`World::register_system`].
pub fn unregister_system<I, O>(system_id: SystemId<I, O>) -> impl Command<Result>
where
    I: SystemInput + Send + 'static,
    O: Send + 'static,
{
    move |world: &mut World| -> Result {
        world.unregister_system(system_id)?;
        Ok(())
    }
}

/// A [`Command`] that removes a system previously registered with
/// [`World::register_system_cached`].
pub fn unregister_system_cached<I, O, M, S>(system: S) -> impl Command<Result>
where
    I: SystemInput + Send + 'static,
    O: 'static,
    M: 'static,
    S: IntoSystem<I, O, M> + Send + 'static,
{
    move |world: &mut World| -> Result {
        world.unregister_system_cached(system)?;
        Ok(())
    }
}

/// A [`Command`] that runs the schedule corresponding to the given [`ScheduleLabel`].
pub fn run_schedule(label: impl ScheduleLabel) -> impl Command<Result> {
    move |world: &mut World| -> Result {
        world.try_run_schedule(label)?;
        Ok(())
    }
}

/// A [`Command`] that sends a global [`Trigger`](crate::observer::Trigger) without any targets.
pub fn trigger(event: impl Event) -> impl Command {
    move |world: &mut World| {
        world.trigger(event);
    }
}

/// A [`Command`] that sends a [`Trigger`](crate::observer::Trigger) for the given targets.
pub fn trigger_targets(
    event: impl Event,
    targets: impl TriggerTargets + Send + Sync + 'static,
) -> impl Command {
    move |world: &mut World| {
        world.trigger_targets(event, targets);
    }
}

/// A [`Command`] that sends an arbitrary [`Event`].
#[track_caller]
pub fn send_event<E: Event>(event: E) -> impl Command {
    #[cfg(feature = "track_location")]
    let caller = Location::caller();
    move |world: &mut World| {
        let mut events = world.resource_mut::<Events<E>>();
        events.send_with_caller(
            event,
            #[cfg(feature = "track_location")]
            caller,
        );
    }
}
