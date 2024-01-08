use bevy_app::{App, AppExit, Plugin, Update};
use bevy_ecs::event::EventWriter;
use bevy_log::info;
use std::env;

/// Output frame rate in Bevy benchmarks.
pub struct BenchmarkPlugin;

impl Plugin for BenchmarkPlugin {
    fn build(&self, app: &mut App) {
        if let Ok(arg) = env::var("BEVY_BENCHMARK_ITER_COUNT") {
            let mut iter_count = arg
                .parse::<u64>()
                .expect("BEVY_BENCHMARK_ITER_COUNT must be a number");
            info!("Will stop after {} iterations", iter_count);
            app.add_systems(Update, move |mut app_exit_events: EventWriter<AppExit>| {
                match iter_count.checked_sub(1) {
                    Some(count) => iter_count = count,
                    None => {
                        app_exit_events.send(AppExit);
                    }
                }
            });
        }
    }
}
