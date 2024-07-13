//! Bevy's collection of [time tracking](crate::TimeTracker) types.

mod mixed_stopwatch;
mod mixed_timer;
mod updating_stopwatch;
mod updating_timer;

pub use mixed_stopwatch::*;
pub use mixed_timer::*;
pub use updating_stopwatch::*;
pub use updating_timer::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
/// Used by mixed types
enum TrackedTime {
    #[default]
    Virtual,
    Fixed,
}
