use thiserror::Error;

use crate::schedule::BoxedScheduleLabel;

/// The error type returned by [`World::try_run_schedule`] if the provided schedule does not exist.
#[derive(Error, Debug)]
#[error("The schedule with the label {0:?} was not found.")]
pub struct TryRunScheduleError(pub BoxedScheduleLabel);
