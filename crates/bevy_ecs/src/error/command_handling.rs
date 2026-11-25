use crate::{
    error::BevyError, system::entity_command::EntityCommandError,
    world::error::EntityMutableFetchError,
};

/// A trait implemented for types that can be used as the output of a [`Command`].
///
/// [`Command`]: crate::system::Command
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `Command` output type",
    label = "invalid `Command` output type",
    note = "the output type of a `Command` should be `()`, `!`, or a `Result` where the error type can be converted into `BevyError`"
)]
pub trait CommandOutput: Sized {
    /// Converts the output into an optional [`BevyError`].
    fn to_err(self) -> Option<BevyError>;
}

impl<T, E> CommandOutput for Result<T, E>
where
    E: Into<BevyError>,
{
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        self.err().map(Into::into)
    }
}

impl CommandOutput for ! {
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        None
    }
}

impl CommandOutput for () {
    #[inline]
    fn to_err(self) -> Option<BevyError> {
        None
    }
}

/// A trait implemented for types that can be used as the output of an [`EntityCommand`].
///
/// [`EntityCommand`]: crate::system::EntityCommand
pub trait EntityCommandOutput {
    /// The type returned when the command is successfully applied.
    type Out;

    /// The error type returned when the command fails to apply. The type must
    /// be convertible into a [`BevyError`] and constructible from an
    /// [`EntityMutableFetchError`].
    type Error: Into<BevyError> + From<EntityMutableFetchError>;

    /// Converts the output into a `Result` containing either the successful output or an error.
    fn into_result(self) -> Result<Self::Out, Self::Error>;
}

impl EntityCommandOutput for () {
    type Out = ();
    type Error = EntityMutableFetchError;

    #[inline]
    fn into_result(self) -> Result<Self::Out, Self::Error> {
        Ok(())
    }
}

impl<T, E> EntityCommandOutput for Result<T, E>
where
    E: Into<BevyError>,
{
    type Out = T;
    type Error = EntityCommandError<E>;

    #[inline]
    fn into_result(self) -> Result<Self::Out, Self::Error> {
        self.map_err(EntityCommandError::CommandFailed)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        error::{BevyError, ErrorContext, FallbackErrorHandler, ResultSeverityExt, Severity},
        prelude::*,
        system::EntityCommand,
        world::error::{EntityDespawnError, EntityMutableFetchError},
    };
    use alloc::string::{String, ToString};
    use bevy_platform::sync::Mutex;

    /// Queues `command` on `entity`, returning the error passed to the fallback handler.
    fn run_failing(world: &mut World, entity: Entity, command: impl EntityCommand) -> BevyError {
        // handlers are function pointers so cannot capture, and tests run in parallel
        std::thread_local! {
            static ERROR: Mutex<Option<BevyError>> = const { Mutex::new(None) };
        }
        fn handler(error: BevyError, _: ErrorContext) {
            ERROR.with(|slot| *slot.lock().unwrap() = Some(error));
        }
        world.insert_resource(FallbackErrorHandler(handler));
        world.commands().entity(entity).queue(command);
        world.flush();
        ERROR.with(|slot| slot.lock().unwrap().take()).unwrap()
    }

    fn despawned(world: &mut World) -> Entity {
        let entity = world.spawn_empty().id();
        world.despawn(entity);
        entity
    }

    #[test]
    fn entity_command_preserves_sources() {
        let mut world = World::new();
        let dead = despawned(&mut world);
        let entity = world.spawn_empty().id();
        let error = run_failing(&mut world, entity, move |entity: EntityWorldMut| {
            entity.into_world_mut().try_despawn(dead)
        });
        assert!(error.is::<EntityDespawnError>());
        assert!(error.to_string().contains(&dead.to_string()));
    }

    #[test]
    fn entity_command_preserves_severity() {
        let mut world = World::new();
        let dead = despawned(&mut world);
        let entity = world.spawn_empty().id();
        let error = run_failing(
            &mut world,
            entity,
            move |entity: EntityWorldMut| -> Result {
                entity.into_world_mut().try_despawn(dead).warn()?;
                Ok(())
            },
        );
        assert_eq!(error.severity(), Severity::Warning);
        assert!(error.is::<EntityDespawnError>());
    }

    #[test]
    fn entity_command_accepts_strings() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let error = run_failing(
            &mut world,
            entity,
            |_: EntityWorldMut| -> Result<(), String> { Err("plain".into()) },
        );
        assert!(error.to_string().starts_with("plain\n"));
    }

    #[test]
    fn entity_command_fetch_error() {
        let mut world = World::new();
        let dead = despawned(&mut world);
        let error = run_failing(&mut world, dead, |_: EntityWorldMut| -> Result { Ok(()) });
        assert!(error.is::<EntityMutableFetchError>());
        assert!(error.to_string().contains(&dead.to_string()));
    }
}
