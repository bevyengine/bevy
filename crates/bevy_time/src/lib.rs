#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
#![doc = include_str!("../README.md")]

/// Common run conditions
pub mod common_conditions;
pub mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

use fixed_timestep::FixedTime;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::warn, Duration, Instant};
pub use crossbeam_channel::TrySendError;
use crossbeam_channel::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{fixed_timestep::FixedTime, Time, Timer, TimerMode};
}

use bevy_app::{prelude::*, RunFixedUpdateLoop};
use bevy_ecs::prelude::*;

use crate::fixed_timestep::run_fixed_update_schedule;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates the elapsed time. Any system that interacts with [`Time`] component should run after
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
            .add_systems(First, time_system.in_set(TimeSystem))
            .add_systems(RunFixedUpdateLoop, run_fixed_update_schedule);

        #[cfg(feature = "bevy_ci_testing")]
        if let Some(ci_testing_config) = app
            .world
            .get_resource::<bevy_app::ci_testing::CiTestingConfig>()
        {
            if let Some(frame_time) = ci_testing_config.frame_time {
                app.world
                    .insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(
                        frame_time,
                    )));
            }
        }
    }
}

/// Configuration resource used to determine how the time system should run.
///
/// For most cases, [`TimeUpdateStrategy::Automatic`] is fine. When writing tests, dealing with networking, or similar
/// you may prefer to set the next [`Time`] value manually.
#[derive(Resource, Default)]
pub enum TimeUpdateStrategy {
    /// [`Time`] will be automatically updated each frame using an [`Instant`] sent from the render world via a [`TimeSender`].
    /// If nothing is sent, the system clock will be used instead.
    #[default]
    Automatic,
    /// [`Time`] will be updated to the specified [`Instant`] value each frame.
    /// In order for time to progress, this value must be manually updated each frame.
    ///
    /// Note that the `Time` resource will not be updated until [`TimeSystem`] runs.
    ManualInstant(Instant),
    /// [`Time`] will be incremented by the specified [`Duration`] each frame.
    ManualDuration(Duration),
}

/// Channel resource used to receive time from the render world.
#[derive(Resource)]
pub struct TimeReceiver(pub Receiver<Instant>);

/// Channel resource used to send time from the render world.
#[derive(Resource)]
pub struct TimeSender(pub Sender<Instant>);

/// Creates channels used for sending time between the render world and the main world.
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
            let last_update = time.last_update().unwrap_or_else(|| time.startup());
            time.update_with_instant(last_update + *duration);
        }
    }
}
