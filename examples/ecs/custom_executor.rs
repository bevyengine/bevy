//! Demonstrates how to make a custom [`SystemExecutor`].

use bevy::{
    ecs::{
        error::{BevyError, ErrorContext},
        schedule::{FixedBitSet, SystemExecutor, SystemSchedule},
    },
    prelude::*,
};

#[derive(Default)]
struct CustomExecutor;

impl SystemExecutor for CustomExecutor {
    fn init(&mut self, _schedule: &SystemSchedule) {}

    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        _skip_systems: Option<&FixedBitSet>,
        _error_handler: fn(BevyError, ErrorContext),
    ) {
        #[expect(unsafe_code, reason = "CustomExecutor's require unsafe")]
        // SAFETY: `run` is a trait method on `System`
        for entry in unsafe { schedule.systems_mut().iter_mut() } {
            let _ = entry.run((), world);
        }
    }

    fn set_apply_final_deferred(&mut self, _value: bool) {}
}

#[derive(Resource, Default)]
struct Counter(u32);

fn increment(mut counter: ResMut<Counter>) {
    counter.0 += 1;
}

fn print_counter(counter: Res<Counter>) {
    println!("Counter: {}", counter.0);
}

fn main() {
    let mut world = World::new();
    world.init_resource::<Counter>();

    let mut schedule = Schedule::default();
    schedule.set_executor(CustomExecutor);
    schedule.add_systems((increment, print_counter).chain());

    for _ in 0..5 {
        schedule.run(&mut world);
    }
}
