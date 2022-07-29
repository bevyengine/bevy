mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

pub use fixed_timestep::*;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

use bevy_ecs::schedule::ShouldRun;
use bevy_ecs::system::{Local, Res, ResMut};
use bevy_utils::{tracing::warn, Instant};
use crossbeam_channel::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{FixedTime, FixedTimestep, FixedTimestepState, Time, Timer};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds timekeeping functionality.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemLabel)]

/// Measures elapsed time since previous update, advances [`Time`], and accumulates elapsed time for
/// to advance [`FixedTime`] later.
///
/// Systems that interact with the [`Time`] resource should be scheduled after this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<FixedTime>()
            .init_resource::<FixedTimestepState>()
            .register_type::<Timer>()
            // time system is added as an "exclusive system" to ensure it runs before other systems
            // in CoreStage::First
            .add_system_to_stage(
                CoreStage::First,
                time_system.exclusive_system().at_start().label(TimeSystem),
            );
    }
}

/// Channel resource used to receive time from render world
pub struct TimeReceiver(pub Receiver<Instant>);
/// Channel resource used to send time from render world
pub struct TimeSender(pub Sender<Instant>);

/// Creates channels used for sending time between render world and app world
pub fn create_time_channels() -> (TimeSender, TimeReceiver) {
    // bound the channel to 2 since when pipelined the render phase can finish before
    // the time system runs.
    let (s, r) = crossbeam_channel::bounded::<Instant>(2);
    (TimeSender(s), TimeReceiver(r))
}

/// Advances [`Time`] and accumulates the elapsed time to advance [`FixedTime`] later.
///
/// If the render world exists, the update [`Instant`] is received from a channel.
/// Otherwise, the update `Instant` is measured inside this system.
fn time_system(
    mut time: ResMut<Time>,
    fixed_time: Option<Res<FixedTime>>,
    accumulator: Option<ResMut<FixedTimestepState>>,
    time_recv: Option<Res<TimeReceiver>>,
    mut has_received_time: Local<bool>,
) {
    let cond1 = time.first_update().is_none();

    if let Some(time_recv) = time_recv {
        // TODO: Figure out how to handle this when using pipelined rendering.
        if let Ok(instant) = time_recv.0.try_recv() {
            time.update_with_instant(instant);
            *has_received_time = true;
        } else if *has_received_time {
            warn!(
                "`time_system` did not receive `Time` from the render world! \
                Calculations depending on the time may be incorrect!"
            );
        }
    } else {
        time.update();
    }

    let cond2 = time.first_update().is_some();

    if let (Some(fixed_time), Some(mut accumulator)) = (fixed_time, accumulator) {
        // On first update, account for the exact startup delay so that `FixedTime` is synced.
        let mut delta = if cond1 && cond2 {
            time.first_update().unwrap() - time.startup()
        } else {
            time.raw_delta()
        };
        // Avoid rounding errors when the relative speed is 1.
        if time.relative_speed_f64() != 1.0 {
            delta = delta.mul_f64(time.relative_speed_f64());
        }
        // Accumulate the time advanced.
        accumulator.add_time(delta, fixed_time.delta());
    }
}

/// A run criteria that succeeds once for every [`FixedTime::delta`] seconds that [`Time`] advances.
///
/// That is different from the run criteria succeeding once every `FixedTime::delta` seconds.
/// The exact CPU time between runs depends on the frame rate and [`Time::relative_speed`].
///
/// For example, a [`Stage`](bevy_ecs::schedule::Stage) set to run 100 times per second (10ms timestep)
/// will run once for every 10ms that [`Time`] advances. But unless [`Time::delta`] happens to be a
/// constant 10ms, the actual time between runs will vary, so systems subject to this run criteria
/// should use `FixedTime` instead of `Time` to see consistent behavior.
pub struct FixedTimestep;

impl FixedTimestep {
    /// Returns `ShouldRun::YesAndCheckAgain` while there are accumulated steps remaining, `ShouldRun::No` otherwise.
    ///
    /// Also returns `ShouldRun::No` if either [`FixedTime`] or [`FixedTimestepState`] does not exist.
    pub fn step(
        fixed_time: Option<ResMut<FixedTime>>,
        accumulator: Option<ResMut<FixedTimestepState>>,
    ) -> ShouldRun {
        match (fixed_time, accumulator) {
            (Some(mut fixed_time), Some(mut accumulator)) => {
                if accumulator.sub_step().is_some() {
                    fixed_time.update();
                    ShouldRun::YesAndCheckAgain
                } else {
                    ShouldRun::No
                }
            }
            _ => ShouldRun::No,
        }
    }
}
