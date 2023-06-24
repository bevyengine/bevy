#![allow(clippy::type_complexity)]

mod clock;
/// Common run conditions
pub mod common_conditions;
pub mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

use fixed_timestep::*;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

use bevy_ecs::system::{Res, ResMut};
use bevy_utils::{tracing::warn, Duration, Instant};
use crossbeam_channel::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{fixed_timestep::FixedTimestep, RealTime, Time, Timer, TimerMode};
}

use bevy_app::{prelude::*, RunFixedUpdateLoop};
use bevy_ecs::prelude::*;

use crate::fixed_timestep::run_fixed_update_schedule;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
/// Updates [`Time`]. All systems that access [`Time`] should run after this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Timer>()
            .register_type::<Time>()
            .register_type::<Stopwatch>();

        // initialize clocks w/ same startup time
        let startup = Instant::now();
        app.insert_resource(Time::new(startup))
            .insert_resource(RealTime::new(startup))
            .init_resource::<FixedTimestep>()
            .init_resource::<TimeUpdateStrategy>()
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
    #[default]
    Automatic,
    // Update [`Time`] with an exact `Instant` value
    ManualInstant(Instant),
    // Update [`Time`] with the last update time + a specified `Duration`
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

/// The system used to update all clocks. If there is a render world, the previous frame duration
/// is sent from there to this system through a channel. Otherwise, it's calculated in this system.
#[allow(clippy::too_many_arguments)]
fn time_system(
    mut time: ResMut<Time>,
    mut real_time: ResMut<RealTime>,
    mut fixed_timestep: ResMut<FixedTimestep>,
    update_strategy: Res<TimeUpdateStrategy>,
    time_recv: Option<Res<TimeReceiver>>,
    mut has_received_time: Local<bool>,
) {
    let real_frame_start = if let Some(time_recv) = time_recv {
        // TODO: Figure out how to handle this when using pipelined rendering.
        if let Ok(instant) = time_recv.0.try_recv() {
            *has_received_time = true;
            instant
        } else {
            if *has_received_time {
                warn!("time_system did not receive the time from the render world! Calculations depending on the time may be incorrect.");
            }
            Instant::now()
        }
    } else {
        Instant::now()
    };

    // update real time clock
    real_time.update_with_instant(real_frame_start);

    let virtual_frame_start = match update_strategy.as_ref() {
        TimeUpdateStrategy::Automatic => real_frame_start,
        TimeUpdateStrategy::ManualInstant(instant) => *instant,
        TimeUpdateStrategy::ManualDuration(duration) => {
            let last_update = time.last_update().unwrap_or(time.startup());
            last_update.checked_add(*duration).unwrap()
        }
    };

    // update virtual time clock
    // TODO: limit how much time can be advanced in a single frame (e.g. Unity's maximumDeltaTime)
    time.update_with_instant(virtual_frame_start);

    // accumulate virtual time delta
    fixed_timestep.accumulate(time.delta());
}
