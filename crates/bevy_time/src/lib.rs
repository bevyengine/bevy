mod fixed_timestep;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;

pub use fixed_timestep::*;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;

use bevy_ecs::system::{Local, Res, ResMut};
use bevy_utils::{tracing::warn, Instant};
use crossbeam_channel::{Receiver, Sender};

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{Time, Timer};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Adds time functionality to Apps.
#[derive(Default)]
pub struct TimePlugin;

#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemLabel)]
/// Updates the elapsed time. Any system that interacts with [Time] component should run after
/// this.
pub struct TimeSystem;

impl Plugin for TimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Time>()
            .init_resource::<FixedTimesteps>()
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

/// The system used to update the [`Time`] used by app logic. If there is a render world the time is sent from
/// there to this system through channels. Otherwise the time is updated in this system.
fn time_system(
    mut time: ResMut<Time>,
    time_recv: Option<Res<TimeReceiver>>,
    mut has_received_time: Local<bool>,
) {
    if let Some(time_recv) = time_recv {
        // TODO: Figure out how to handle this when using pipelined rendering.
        if let Ok(new_time) = time_recv.0.try_recv() {
            time.update_with_instant(new_time);
            *has_received_time = true;
        } else if *has_received_time {
            warn!("time_system did not receive the time from the render world! Calculations depending on the time may be incorrect.");
        }
    } else {
        time.update();
    }
}
