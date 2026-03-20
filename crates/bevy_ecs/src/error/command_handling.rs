use core::fmt;

use bevy_utils::prelude::DebugName;

use crate::{
    entity::Entity,
    never::Never,
    system::{entity_command::EntityCommandError, Command, EntityCommand},
    world::{error::EntityMutableFetchError, World},
};

use super::{BevyError, ErrorContext, ErrorHandler};

/// A trait implemented for types that can be used as the output of a [`Command`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `Command` output type",
    label = "invalid `Command` output type",
    note = "the output type of a `Command` should be `()`, `Never`, or a `Result` where the error type can be converted into `BevyError`"
)]
pub trait CommandOutput: Sized {
    /// Takes a [`Command`] that returns a Result and uses a given error handler function to convert it into
    /// a [`Command`] that internally handles an error if it occurs and returns `()`.
    fn handle_error_with<C: Command<Out = Self>>(
        command: C,
        error_handler: ErrorHandler,
    ) -> impl Command<Out = ()>;

    /// Takes a [`Command`] that returns a Result and uses the default error handler function to convert it into
    /// a [`Command`] that internally handles an error if it occurs and returns `()`.
    fn handle_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()>;

    /// Takes a [`Command`] that returns a Result and ignores any error that occurs.
    fn ignore_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()>;
}

impl<T, E> CommandOutput for Result<T, E>
where
    E: Into<BevyError>,
{
    fn handle_error_with<C: Command<Out = Self>>(
        command: C,
        error_handler: ErrorHandler,
    ) -> impl Command<Out = ()> {
        move |world: &mut World| match command.apply(world) {
            Ok(_) => {}
            Err(err) => (error_handler)(
                err.into(),
                ErrorContext::Command {
                    name: DebugName::type_name::<C>(),
                },
            ),
        }
    }

    fn handle_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        move |world: &mut World| match command.apply(world) {
            Ok(_) => {}
            Err(err) => world.default_error_handler()(
                err.into(),
                ErrorContext::Command {
                    name: DebugName::type_name::<C>(),
                },
            ),
        }
    }

    fn ignore_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        move |world: &mut World| {
            let _ = command.apply(world);
        }
    }
}

impl CommandOutput for Never {
    fn handle_error_with<C: Command<Out = Self>>(
        command: C,
        _error_handler: ErrorHandler,
    ) -> impl Command<Out = ()> {
        move |world: &mut World| {
            command.apply(world);
        }
    }

    #[inline]
    fn handle_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        move |world: &mut World| {
            command.apply(world);
        }
    }

    #[inline]
    fn ignore_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        move |world: &mut World| {
            command.apply(world);
        }
    }
}

impl CommandOutput for () {
    #[inline]
    fn handle_error_with<C: Command<Out = Self>>(
        command: C,
        _error_handler: ErrorHandler,
    ) -> impl Command<Out = ()> {
        command
    }
    #[inline]
    fn handle_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        command
    }
    #[inline]
    fn ignore_error<C: Command<Out = Self>>(command: C) -> impl Command<Out = ()> {
        command
    }
}

/// A trait for types that can be used as the output of an [`EntityCommand`] when
/// converted into a [`Command`] with a specific entity using [`CommandWithEntity`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `CommandWithEntity` output type",
    label = "invalid `CommandWithEntity` output type",
    note = "the output type of a `CommandWithEntity` should be `()` or a `Result` where the error type can be wrapped in an `EntityCommandError`"
)]
pub trait EntityCommandOutput: Sized {
    /// Passes in a specific entity to an [`EntityCommand`], resulting in a [`Command`] that
    /// internally runs the [`EntityCommand`] on that entity.
    fn with_entity<C: EntityCommand<Out = Self>>(command: C, entity: Entity) -> impl Command;
}

impl EntityCommandOutput for () {
    fn with_entity<C: EntityCommand<Out = Self>>(command: C, entity: Entity) -> impl Command {
        move |world: &mut World| {
            let entity = world.get_entity_mut(entity)?;
            command.apply(entity);
            Ok::<(), EntityMutableFetchError>(())
        }
    }
}

impl<T, Err> EntityCommandOutput for Result<T, Err>
where
    Err: fmt::Debug + fmt::Display + Send + Sync + 'static,
{
    fn with_entity<C: EntityCommand<Out = Self>>(command: C, entity: Entity) -> impl Command {
        move |world: &mut World| {
            let entity = world.get_entity_mut(entity)?;
            command
                .apply(entity)
                .map_err(EntityCommandError::CommandFailed)
        }
    }
}
