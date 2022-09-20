#![allow(unused_imports)]
use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use bevy_ecs::system::{IntoExclusiveSystem, Local, Res, ResMut};
use bevy_time::FixedTimestep;
use bevy_time::{FixedTimesteps, Instant, Stopwatch, Time, Timer};
use bevy_utils::tracing::warn;
use crossbeam_channel::{Receiver, Sender};

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
            .register_type::<Time>()
            .register_type::<Stopwatch>()
            // time system is added as an "exclusive system" to ensure it runs before other systems
            // in CoreStage::First
            .add_system_to_stage(
                CoreStage::First,
                time_system.exclusive_system().at_start().label(TimeSystem),
            );
    }
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

#[cfg(test)]
mod test {
    use super::*;
    use bevy_time::Instant;
    use std::ops::{Add, Mul};
    use std::time::Duration;

    #[derive(Resource)]
    struct Count(usize);
    const LABEL: &str = "test_step";

    #[test]
    fn test() {
        let mut world = World::default();
        let mut time = Time::default();
        let instance = Instant::now();
        time.update_with_instant(instance);
        world.insert_resource(time);
        world.insert_resource(FixedTimesteps::default());
        world.insert_resource(Count(0));
        let mut schedule = Schedule::default();

        #[derive(StageLabel)]
        struct Update;
        schedule.add_stage(
            Update,
            SystemStage::parallel()
                .with_run_criteria(FixedTimestep::step(0.5).with_label(LABEL))
                .with_system(fixed_update),
        );

        // if time does not progress, the step does not run
        schedule.run(&mut world);
        schedule.run(&mut world);
        assert_eq!(0, world.resource::<Count>().0);
        assert_eq!(0., get_accumulator_deciseconds(&world));

        // let's progress less than one step
        advance_time(&mut world, instance, 0.4);
        schedule.run(&mut world);
        assert_eq!(0, world.resource::<Count>().0);
        assert_eq!(4., get_accumulator_deciseconds(&world));

        // finish the first step with 0.1s above the step length
        advance_time(&mut world, instance, 0.6);
        schedule.run(&mut world);
        assert_eq!(1, world.resource::<Count>().0);
        assert_eq!(1., get_accumulator_deciseconds(&world));

        // runs multiple times if the delta is multiple step lengths
        advance_time(&mut world, instance, 1.7);
        schedule.run(&mut world);
        assert_eq!(3, world.resource::<Count>().0);
        assert_eq!(2., get_accumulator_deciseconds(&world));
    }

    fn fixed_update(mut count: ResMut<Count>) {
        count.0 += 1;
    }

    fn advance_time(world: &mut World, instance: Instant, seconds: f32) {
        world
            .resource_mut::<Time>()
            .update_with_instant(instance.add(Duration::from_secs_f32(seconds)));
    }

    fn get_accumulator_deciseconds(world: &World) -> f64 {
        world
            .resource::<FixedTimesteps>()
            .get(LABEL)
            .unwrap()
            .accumulator()
            .mul(10.)
            .round()
    }
}
