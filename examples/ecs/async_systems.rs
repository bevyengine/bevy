use std::time::Duration;

use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .insert_resource(GreetMessage("Hello".to_owned()))
        .insert_resource(NextGreetDelay(0.))
        .insert_resource(ExecutionTime(0.))
        .add_system(async_system.system())
        .add_system(execution_time_counter.system())
        .run();
}

struct GreetMessage(String);
struct NextGreetDelay(f32);
struct ExecutionTime(f32);

type Access<'a> = (
    Res<'a, GreetMessage>,
    ResMut<'a, NextGreetDelay>,
    Res<'a, Time>,
    Res<'a, ExecutionTime>,
);

async fn async_system(mut accessor: Accessor<Access<'_>>) {
    fn sync_operation(msg: &str, execution_time: f32) {
        println!("{} @ {}", msg, execution_time);
    }

    let wait_duration = accessor
        .access(
            |(greet_msg, mut next_delay, time, execution_time): Access<'_>| {
                next_delay.0 += 1.;
                if next_delay.0 > time.delta_seconds() {
                    sync_operation(&greet_msg.0, execution_time.0);
                    next_delay.0 - time.delta_seconds()
                } else {
                    // We had a lag spike, and the frame time exceeded our waiting.
                    let mut timer = time.delta_seconds();
                    while timer > next_delay.0 {
                        timer -= next_delay.0;
                        // Do multiple operations to catch up.
                        // (This is an example so we don't actually have anything to do)
                        sync_operation(&greet_msg.0, execution_time.0)
                    }
                    timer
                }
            },
        )
        .await;

    futures_timer::Delay::new(Duration::from_secs_f32(wait_duration)).await;
}

fn execution_time_counter(mut execution_time: ResMut<ExecutionTime>, time: Res<Time>) {
    execution_time.0 += time.delta_seconds();
}
