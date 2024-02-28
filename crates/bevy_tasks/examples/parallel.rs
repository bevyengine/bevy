use std::hint::black_box;

use bevy_tasks::{ComputeTaskPool, IntoParallelRefIterator, ParallelIterator, TaskPoolBuilder};
use web_time::{Duration, Instant};

pub fn heavy_compute(v: i32) -> i32 {
    let now = Instant::now();
    while Instant::now() - now < Duration::from_micros(4) {
        // spin, simulating work being done
    }
    v
}

// 1,000,000 tasks that spin for 4us ,It's expected to take about one second to run (assuming the machine has >= 4 logical
// cores)
fn main() {
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::default()
            .num_threads(4)
            .thread_name("Compute Task Pool".to_string())
            .build()
    });

    let a: Vec<_> = (0..1000000).collect();
    let t0 = Instant::now();
    a.par_iter().for_each(|v| {
        black_box(heavy_compute(*v));
    });
    println!("foreach: {:?} elapsed", t0.elapsed());
}
