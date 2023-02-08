pub mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

use fixed_timestep::{run_fixed_update_schedule, FixedTime};
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::warn, Duration, Instant};
use crossbeam_channel::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{fixed_timestep::FixedTime, Time, Timer, TimerMode};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates the elapsed time. Any system that interacts with [Time] component should run after
/// this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<TimeUpdateStrategy>()
            .register_type::<Timer>()
            .register_type::<Time>()
            .register_type::<Stopwatch>()
            .init_resource::<FixedTime>()
            .configure_set(TimeSystem.in_base_set(CoreSet::First))
            .add_system(time_system.in_set(TimeSystem))
            .add_system(run_fixed_update_schedule.in_base_set(CoreSet::FixedUpdate));
    }
}

/// Configuration resource used to determine how the time system should run.
///
/// For most cases, [`TimeUpdateStrategy::Automatic`] is fine. When writing tests, dealing with networking, or similar
/// you may prefer to set the next [`Time`] value manually.
#[derive(Resource, Default)]
pub enum TimeUpdateStrategy {
    #[default]
    Automatic,
    // Update [`Time`] with an exact `Instant` value
    ManualInstant(Instant),
    // Update [`Time`] with the current time + a specified `Duration`
    ManualDuration(Duration),
}

/// Channel resource used to receive time from render world
#[derive(Resource)]
pub struct TimeReceiver(pub Receiver<Instant>);

/// Channel resource used to send time from render world
#[derive(Resource)]
pub struct TimeSender(pub Sender<Instant>);

/// Creates channels used for sending time between render world and app world
pub fn create_time_channels() -> (TimeSender, TimeReceiver) {
    // bound the channel to 2 since when pipelined the render phase can finish before
    // the time system runs.
    let (s, r) = crossbeam_channel::bounded::<Instant>(2);
    (TimeSender(s), TimeReceiver(r))
}

/// The system used to update the [`Time`] used by app logic. If there is a render world the time is sent from
/// there to this system through channels. Otherwise the time is updated in this system.
fn time_system(
    mut time: ResMut<Time>,
    update_strategy: Res<TimeUpdateStrategy>,
    time_recv: Option<Res<TimeReceiver>>,
    mut has_received_time: Local<bool>,
) {
    let new_time = if let Some(time_recv) = time_recv {
        // TODO: Figure out how to handle this when using pipelined rendering.
        if let Ok(new_time) = time_recv.0.try_recv() {
            *has_received_time = true;
            new_time
        } else {
            if *has_received_time {
                warn!("time_system did not receive the time from the render world! Calculations depending on the time may be incorrect.");
            }
            Instant::now()
        }
    } else {
        Instant::now()
    };

    match update_strategy.as_ref() {
        TimeUpdateStrategy::Automatic => time.update_with_instant(new_time),
        TimeUpdateStrategy::ManualInstant(instant) => time.update_with_instant(*instant),
        TimeUpdateStrategy::ManualDuration(duration) => {
            time.update_with_instant(Instant::now() + *duration);
        }
    }
}
