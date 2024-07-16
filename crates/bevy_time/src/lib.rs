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
use bevy_ecs::event::{signal_event_update_system, EventRegistry, ShouldUpdateEvents};
use bevy_ecs::prelude::*;
use bevy_utils::{tracing::warn, Duration, Instant};
pub use crossbeam_channel::TrySendError;
use crossbeam_channel::{Receiver, Sender};

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
            .init_resource::<TimeUpdateStrategy>();

        #[cfg(feature = "bevy_reflect")]
        {
            app.register_type::<Time>()
                .register_type::<Time<Real>>()
                .register_type::<Time<Virtual>>()
                .register_type::<Time<Fixed>>()
                .register_type::<Timer>();
        }

        app.add_systems(First, time_system.in_set(TimeSystem))
            .add_systems(RunFixedMainLoop, run_fixed_main_schedule);

        // Ensure the events are not dropped until `FixedMain` systems can observe them
        app.add_systems(FixedPostUpdate, signal_event_update_system);
        let mut event_registry = app.world_mut().resource_mut::<EventRegistry>();
        // We need to start in a waiting state so that the events are not updated until the first fixed update
        event_registry.should_update = ShouldUpdateEvents::Waiting;
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

/// The system used to update the [`Time`] used by app logic. If there is a render world the time is
/// sent from there to this system through channels. Otherwise the time is updated in this system.
pub fn time_system(
    mut real_time: ResMut<Time<Real>>,
    mut virtual_time: ResMut<Time<Virtual>>,
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
        TimeUpdateStrategy::Automatic => real_time.update_with_instant(new_time),
        TimeUpdateStrategy::ManualInstant(instant) => real_time.update_with_instant(*instant),
        TimeUpdateStrategy::ManualDuration(duration) => real_time.update_with_duration(*duration),
    }

    update_virtual_time(&mut time, &mut virtual_time, &real_time);
}

#[cfg(test)]
mod tests {
    use crate::{Fixed, Time, TimePlugin, TimeUpdateStrategy, Virtual};
    use bevy_app::{App, FixedUpdate, Startup, Update};
    use bevy_ecs::{
        event::{Event, EventReader, EventRegistry, EventWriter, Events, ShouldUpdateEvents},
        system::{Local, Res, ResMut, Resource},
    };
    use bevy_utils::Duration;
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

    #[derive(Event)]
    struct DummyEvent;

    #[derive(Resource, Default)]
    struct FixedUpdateCounter(u8);

    fn count_fixed_updates(mut counter: ResMut<FixedUpdateCounter>) {
        counter.0 += 1;
    }

    fn report_time(
        mut frame_count: Local<u64>,
        virtual_time: Res<Time<Virtual>>,
        fixed_time: Res<Time<Fixed>>,
    ) {
        println!(
            "Virtual time on frame {}: {:?}",
            *frame_count,
            virtual_time.elapsed()
        );
        println!(
            "Fixed time on frame {}: {:?}",
            *frame_count,
            fixed_time.elapsed()
        );

        *frame_count += 1;
    }

    #[test]
    fn fixed_main_schedule_should_run_with_time_plugin_enabled() {
        // Set the time step to just over half the fixed update timestep
        // This way, it will have not accumulated enough time to run the fixed update after one update
        // But will definitely have enough time after two updates
        let fixed_update_timestep = Time::<Fixed>::default().timestep();
        let time_step = fixed_update_timestep / 2 + Duration::from_millis(1);

        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_systems(FixedUpdate, count_fixed_updates)
            .add_systems(Update, report_time)
            .init_resource::<FixedUpdateCounter>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(time_step));

        // Frame 0
        // Fixed update should not have run yet
        app.update();

        assert!(Duration::ZERO < fixed_update_timestep);
        let counter = app.world().resource::<FixedUpdateCounter>();
        assert_eq!(counter.0, 0, "Fixed update should not have run yet");

        // Frame 1
        // Fixed update should not have run yet
        app.update();

        assert!(time_step < fixed_update_timestep);
        let counter = app.world().resource::<FixedUpdateCounter>();
        assert_eq!(counter.0, 0, "Fixed update should not have run yet");

        // Frame 2
        // Fixed update should have run now
        app.update();

        assert!(2 * time_step > fixed_update_timestep);
        let counter = app.world().resource::<FixedUpdateCounter>();
        assert_eq!(counter.0, 1, "Fixed update should have run once");

        // Frame 3
        // Fixed update should have run exactly once still
        app.update();

        assert!(3 * time_step < 2 * fixed_update_timestep);
        let counter = app.world().resource::<FixedUpdateCounter>();
        assert_eq!(counter.0, 1, "Fixed update should have run once");

        // Frame 4
        // Fixed update should have run twice now
        app.update();

        assert!(4 * time_step > 2 * fixed_update_timestep);
        let counter = app.world().resource::<FixedUpdateCounter>();
        assert_eq!(counter.0, 2, "Fixed update should have run twice");
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

    #[test]
    fn event_update_should_wait_for_fixed_main() {
        // Set the time step to just over half the fixed update timestep
        // This way, it will have not accumulated enough time to run the fixed update after one update
        // But will definitely have enough time after two updates
        let fixed_update_timestep = Time::<Fixed>::default().timestep();
        let time_step = fixed_update_timestep / 2 + Duration::from_millis(1);

        fn send_event(mut events: ResMut<Events<DummyEvent>>) {
            events.send(DummyEvent);
        }

        let mut app = App::new();
        app.add_plugins(TimePlugin)
            .add_event::<DummyEvent>()
            .init_resource::<FixedUpdateCounter>()
            .add_systems(Startup, send_event)
            .add_systems(FixedUpdate, count_fixed_updates)
            .insert_resource(TimeUpdateStrategy::ManualDuration(time_step));

        for frame in 0..10 {
            app.update();
            let fixed_updates_seen = app.world().resource::<FixedUpdateCounter>().0;
            let events = app.world().resource::<Events<DummyEvent>>();
            let n_total_events = events.len();
            let n_current_events = events.iter_current_update_events().count();
            let event_registry = app.world().resource::<EventRegistry>();
            let should_update = event_registry.should_update;

            println!("Frame {frame}, {fixed_updates_seen} fixed updates seen. Should update: {should_update:?}");
            println!("Total events: {n_total_events} | Current events: {n_current_events}",);

            match frame {
                0 | 1 => {
                    assert_eq!(fixed_updates_seen, 0);
                    assert_eq!(n_total_events, 1);
                    assert_eq!(n_current_events, 1);
                    assert_eq!(should_update, ShouldUpdateEvents::Waiting);
                }
                2 => {
                    assert_eq!(fixed_updates_seen, 1); // Time to trigger event updates
                    assert_eq!(n_total_events, 1);
                    assert_eq!(n_current_events, 1);
                    assert_eq!(should_update, ShouldUpdateEvents::Ready); // Prepping first update
                }
                3 => {
                    assert_eq!(fixed_updates_seen, 1);
                    assert_eq!(n_total_events, 1);
                    assert_eq!(n_current_events, 0); // First update has occurred
                    assert_eq!(should_update, ShouldUpdateEvents::Waiting);
                }
                4 => {
                    assert_eq!(fixed_updates_seen, 2); // Time to trigger the second update
                    assert_eq!(n_total_events, 1);
                    assert_eq!(n_current_events, 0);
                    assert_eq!(should_update, ShouldUpdateEvents::Ready); // Prepping second update
                }
                5 => {
                    assert_eq!(fixed_updates_seen, 2);
                    assert_eq!(n_total_events, 0); // Second update has occurred
                    assert_eq!(n_current_events, 0);
                    assert_eq!(should_update, ShouldUpdateEvents::Waiting);
                }
                _ => {
                    assert_eq!(n_total_events, 0); // No more events are sent
                    assert_eq!(n_current_events, 0);
                }
            }
        }
    }
}
