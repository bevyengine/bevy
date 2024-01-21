use std::hint::black_box;

use bevy_tasks::{
    compute_task_pool_thread_num, ComputeTaskPool, IntoParallelRefIterator, ParallelIterator,
    TaskPool, TaskPoolBuilder,
};
use web_time::{Duration, Instant};

pub fn heavy_compute(v: i32) -> i32 {
    let now = Instant::now();
    while Instant::now() - now < Duration::from_micros(1) {
        // spin, simulating work being done
    }
    v
}
fn main() {
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::default()
            .num_threads(5)
            .thread_name("Compute Task Pool".to_string())
            .build()
    });
    println!("main {:?}", std::thread::current().id());
    let a: Vec<_> = (0..20000).collect();
    let t0 = Instant::now();
    a.iter().for_each(|v| {
        black_box(v);
        black_box(heavy_compute(*v));
    });
    println!(" iter {:?} elapsed", t0.elapsed());
    const iter_count: usize = 1000;
    let t0 = Instant::now();
    for _ in 0..iter_count {
        a.par_iter().for_each(|v| {
            // println!("Thread {:?}  finished", std::thread::current().id(),);
            black_box(v);
            black_box(heavy_compute(*v));
        });
    }
    println!(" par {:?} elapsed", t0.elapsed() / iter_count as u32);
}
