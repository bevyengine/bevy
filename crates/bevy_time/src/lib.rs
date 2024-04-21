#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

/// Common run conditions
pub mod common_conditions;
mod fixed;
mod real;
mod stopwatch;
#[allow(clippy::module_inception)]
mod time;
mod timer;
mod virt;

pub use fixed::*;
pub use real::*;
pub use stopwatch::*;
pub use time::*;
pub use timer::*;
pub use virt::*;

pub mod prelude {
    //! The Bevy Time Prelude.
    #[doc(hidden)]
    pub use crate::{Fixed, Real, Time, Timer, TimerMode, Virtual};
}

use bevy_app::{prelude::*, RunFixedMainLoop};
use bevy_ecs::event::signal_event_update_system;
use bevy_ecs::prelude::*;
use bevy_utils::{tracing::warn, Duration, Instant};
use concurrent_queue::ConcurrentQueue;
pub use concurrent_queue::PushError;
use std::sync::Arc;

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
            .init_resource::<Time<Real>>()
            .init_resource::<Time<Virtual>>()
            .init_resource::<Time<Fixed>>()
            .init_resource::<TimeUpdateStrategy>()
            .register_type::<Time>()
            .register_type::<Time<Real>>()
            .register_type::<Time<Virtual>>()
            .register_type::<Time<Fixed>>()
            .register_type::<Timer>()
            .add_systems(First, time_system.in_set(TimeSystem))
            .add_systems(RunFixedMainLoop, run_fixed_main_schedule);

        // ensure the events are not dropped until `FixedMain` systems can observe them
        app.add_systems(FixedPostUpdate, signal_event_update_system);
    }
}

/// Configuration resource used to determine how the time system should run.
///
/// For most cases, [`TimeUpdateStrategy::Automatic`] is fine. When writing tests, dealing with
/// networking or similar, you may prefer to set the next [`Time`] value manually.
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

/// Queue resource used to receive time from the render world.
#[derive(Resource, Clone)]
pub struct TimeEventQueue(pub Arc<ConcurrentQueue<Instant>>);

/// Creates channels used for sending time between the render world and the main world.
pub fn create_time_event_queue() -> TimeEventQueue {
    // bound the channel to 2 since when pipelined the render phase can finish before
    // the time system runs.
    TimeEventQueue(Arc::new(ConcurrentQueue::bounded(2)))
}

/// The system used to update the [`Time`] used by app logic. If there is a render world the time is
/// sent from there to this system through channels. Otherwise the time is updated in this system.
fn time_system(
    mut real_time: ResMut<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
    mut time: ResMut<Time>,
    update_strategy: Res<TimeUpdateStrategy>,
    time_event_queue: Option<Res<TimeEventQueue>>,
    mut has_received_time: Local<bool>,
) {
    let new_time = if let Some(time_event_queue) = time_event_queue {
        // TODO: Figure out how to handle this when using pipelined rendering.
        if let Ok(new_time) = time_event_queue.0.pop() {
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
        TimeUpdateStrategy::Automatic => real_time.update_with_instant(new_time),
        TimeUpdateStrategy::ManualInstant(instant) => real_time.update_with_instant(*instant),
        TimeUpdateStrategy::ManualDuration(duration) => real_time.update_with_duration(*duration),
    }

    update_virtual_time(&mut time, &mut virtual_time, &real_time);
}

#[cfg(test)]
mod tests {
    use crate::{Fixed, Time, TimePlugin, TimeUpdateStrategy};
    use bevy_app::{App, Startup, Update};
    use bevy_ecs::event::{Event, EventReader, EventWriter};
    use std::error::Error;

    #[derive(Event)]
    struct TestEvent<T: Default> {
        sender: std::sync::mpsc::Sender<T>,
    }

    impl<T: Default> Drop for TestEvent<T> {
        fn drop(&mut self) {
            self.sender
                .send(T::default())
                .expect("Failed to send drop signal");
        }
    }

    #[test]
    fn events_get_dropped_regression_test_11528() -> Result<(), impl Error> {
        let (tx1, rx1) = std::sync::mpsc::channel();
        let (tx2, rx2) = std::sync::mpsc::channel();
        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_event::<TestEvent<i32>>()
            .add_event::<TestEvent<()>>()
            .add_systems(Startup, move |mut ev2: EventWriter<TestEvent<()>>| {
                ev2.send(TestEvent {
                    sender: tx2.clone(),
                });
            })
            .add_systems(Update, move |mut ev1: EventWriter<TestEvent<i32>>| {
                // Keep adding events so this event type is processed every update
                ev1.send(TestEvent {
                    sender: tx1.clone(),
                });
            })
            .add_systems(
                Update,
                |mut ev1: EventReader<TestEvent<i32>>, mut ev2: EventReader<TestEvent<()>>| {
                    // Read events so they can be dropped
                    for _ in ev1.read() {}
                    for _ in ev2.read() {}
                },
            )
            .insert_resource(TimeUpdateStrategy::ManualDuration(
                Time::<Fixed>::default().timestep(),
            ));

        for _ in 0..10 {
            app.update();
        }

        // Check event type 1 as been dropped at least once
        let _drop_signal = rx1.try_recv()?;
        // Check event type 2 has been dropped
        rx2.try_recv()
    }
}
