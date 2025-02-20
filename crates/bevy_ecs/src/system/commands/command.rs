//! This module contains the definition of the [`Command`] trait, as well as
//! blanket implementations of the trait for closures.
//!
//! It also contains functions that return closures for use with
//! [`Commands`](crate::system::Commands).

use crate::{
    bundle::{Bundle, InsertMode, NoBundleEffect},
    change_detection::MaybeLocation,
    entity::Entity,
    event::{Event, Events},
    observer::TriggerTargets,
    resource::Resource,
    result::{Error, Result},
    schedule::ScheduleLabel,
    system::{error_handler, IntoSystem, SystemId, SystemInput},
    world::{FromWorld, SpawnBatchIter, World},
};

/// A [`World`] mutation.
///
/// Should be used with [`Commands::queue`](crate::system::Commands::queue).
///
/// The `Out` generic parameter is the returned "output" of the command.
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
///     fn apply(self, world: &mut World) {
///         let mut counter = world.get_resource_or_insert_with(Counter::default);
///         counter.0 += self.0;
///     }
/// }
///
/// fn some_system(mut commands: Commands) {
///     commands.queue(AddToCounter(42));
/// }
/// ```
pub trait Command<Out = ()>: Send + 'static {
    /// Applies this command, causing it to mutate the provided `world`.
    ///
    /// This method is used to define what a command "does" when it is ultimately applied.
    /// Because this method takes `self`, you can store data or settings on the type that implements this trait.
    /// This data is set by the system or other source of the command, and then ultimately read in this method.
    fn apply(self, world: &mut World) -> Out;
}

impl<F, Out> Command<Out> for F
where
    F: FnOnce(&mut World) -> Out + Send + 'static,
{
    fn apply(self, world: &mut World) -> Out {
        self(world)
    }
}

/// Takes a [`Command`] that returns a Result and uses a given error handler function to convert it into
/// a [`Command`] that internally handles an error if it occurs and returns `()`.
pub trait HandleError<Out = ()> {
    /// Takes a [`Command`] that returns a Result and uses a given error handler function to convert it into
    /// a [`Command`] that internally handles an error if it occurs and returns `()`.
    fn handle_error_with(self, error_handler: fn(&mut World, Error)) -> impl Command;
    /// Takes a [`Command`] that returns a Result and uses the default error handler function to convert it into
    /// a [`Command`] that internally handles an error if it occurs and returns `()`.
    fn handle_error(self) -> impl Command
    where
        Self: Sized,
    {
        self.handle_error_with(error_handler::default())
    }
}

impl<C: Command<Result<T, E>>, T, E: Into<Error>> HandleError<Result<T, E>> for C {
    fn handle_error_with(self, error_handler: fn(&mut World, Error)) -> impl Command {
        move |world: &mut World| match self.apply(world) {
            Ok(_) => {}
            Err(err) => (error_handler)(world, err.into()),
        }
    }
}

impl<C: Command> HandleError for C {
    #[inline]
    fn handle_error_with(self, _error_handler: fn(&mut World, Error)) -> impl Command {
        self
    }
    #[inline]
    fn handle_error(self) -> impl Command
    where
        Self: Sized,
    {
        self
    }
}

/// A [`Command`] that consumes an iterator of [`Bundles`](Bundle) to spawn a series of entities.
///
/// This is more efficient than spawning the entities individually.
#[track_caller]
pub fn spawn_batch<I>(bundles_iter: I) -> impl Command
where
    I: IntoIterator + Send + Sync + 'static,
    I::Item: Bundle<Effect: NoBundleEffect>,
{
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        SpawnBatchIter::new(world, bundles_iter.into_iter(), caller);
    }
}

/// A [`Command`] that consumes an iterator to add a series of [`Bundles`](Bundle) to a set of entities.
///
/// If any entities do not exist in the world, this command will return a
/// [`TryInsertBatchError`](crate::world::error::TryInsertBatchError).
///
/// This is more efficient than inserting the bundles individually.
#[track_caller]
pub fn insert_batch<I, B>(batch: I, insert_mode: InsertMode) -> impl Command<Result>
where
    I: IntoIterator<Item = (Entity, B)> + Send + Sync + 'static,
    B: Bundle<Effect: NoBundleEffect>,
{
    let caller = MaybeLocation::caller();
    move |world: &mut World| -> Result {
        world.try_insert_batch_with_caller(batch, insert_mode, caller)?;
        Ok(())
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
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        world.insert_resource_with_caller(resource, caller);
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
#[track_caller]
pub fn trigger(event: impl Event) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        world.trigger_with_caller(event, caller);
    }
}

/// A [`Command`] that sends a [`Trigger`](crate::observer::Trigger) for the given targets.
pub fn trigger_targets(
    event: impl Event,
    targets: impl TriggerTargets + Send + Sync + 'static,
) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        world.trigger_targets_with_caller(event, targets, caller);
    }
}

/// A [`Command`] that sends an arbitrary [`Event`].
#[track_caller]
pub fn send_event<E: Event>(event: E) -> impl Command {
    let caller = MaybeLocation::caller();
    move |world: &mut World| {
        let mut events = world.resource_mut::<Events<E>>();
        events.send_with_caller(event, caller);
    }
}
