use bevy_app::{App, Plugin, Update};

use crate::fbs_benchmark::fps_benchmark_system;

mod fbs_benchmark;

/// Output frame rate in Bevy benchmarks.
pub struct BenchmarkPlugin;

impl Plugin for BenchmarkPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, fps_benchmark_system);
    }
}
