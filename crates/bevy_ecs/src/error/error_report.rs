use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::{
    error::Error,
    fmt::{Display, Formatter, Result},
};

/// Displays an error followed by its chain of [`Error::source`] causes.
///
/// An error's own [`Display`] is only its message, leaving its causes out.
/// Errors returned to Bevy as a [`BevyError`](crate::error::BevyError) are already reported this way,
/// so use this when logging an error directly.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::{error::ErrorReport, world::World};
/// let mut world = World::new();
/// let entity = world.spawn_empty().id();
/// world.despawn(entity);
/// let error = world.try_despawn(entity).unwrap_err();
///
/// // the message alone doesn't say why
/// assert_eq!(error.to_string(), "Could not despawn entity");
/// assert!(ErrorReport::new(&error)
///     .to_string()
///     .starts_with("Could not despawn entity\n\nCaused by:\n\tEntity despawned"));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ErrorReport<'a>(&'a (dyn Error + 'static));

impl<'a> ErrorReport<'a> {
    /// Creates a report of `error` and its causes.
    pub fn new(error: &'a (dyn Error + 'static)) -> Self {
        Self(error)
    }

    /// Iterates over the error and its chain of [`Error::source`] causes,
    /// starting with the error itself.
    pub fn chain(self) -> impl Iterator<Item = &'a (dyn Error + 'static)> {
        core::iter::successors(Some(self.0), |&error| error.source())
    }

    /// Writes the first message, then the rest as an indented "Caused by:" list.
    pub(super) fn fmt_messages(f: &mut Formatter<'_>, messages: &[String]) -> Result {
        let Some((first, causes)) = messages.split_first() else {
            return Ok(());
        };
        write!(f, "{first}")?;
        if !causes.is_empty() {
            write!(f, "\n\nCaused by:")?;
            for line in causes.iter().flat_map(|cause| cause.lines()) {
                write!(f, "\n\t{line}")?;
            }
        }
        Ok(())
    }
}

impl Display for ErrorReport<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let messages: Vec<String> = self.chain().map(|error| error.to_string()).collect();
        Self::fmt_messages(f, &messages)
    }
}

#[cfg(test)]
mod tests {
    use super::ErrorReport;
    use crate::{error::BevyError, world::World};
    use alloc::{format, string::ToString};

    #[test]
    fn without_source() {
        let error = core::fmt::Error;
        assert_eq!(ErrorReport::new(&error).to_string(), error.to_string());
    }

    #[test]
    fn matches_bevy_error() {
        let mut world = World::new();
        let entity = world.spawn_empty().id();
        world.despawn(entity);
        let error = world.try_despawn(entity).unwrap_err();

        let report = ErrorReport::new(&error).to_string();
        assert!(report.contains(&entity.to_string()));
        assert!(BevyError::from(error)
            .to_string()
            .starts_with(&format!("{report}\n")));
    }
}
