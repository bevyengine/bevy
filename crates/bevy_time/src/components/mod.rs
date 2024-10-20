//! Bevy's collection of [time tracking](crate::TimeTracker) types.

mod mixed_stopwatch;
mod mixed_timer;
mod updating_stopwatch;
mod updating_timer;

pub use mixed_stopwatch::*;
pub use mixed_timer::*;
pub use updating_stopwatch::*;
pub use updating_timer::*;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Default))]
/// Used by mixed types
enum TrackedTime {
    #[default]
    Virtual,
    Fixed,
}
