use bevy_tasks::{
    ComputeTaskPool, IntoParallelRefIterator, ParallelIterator, TaskPool, TaskPoolBuilder,
};
use web_time::{Duration, Instant};

fn main() {
    ComputeTaskPool::get_or_init(|| {
        TaskPoolBuilder::default()
            .num_threads(20)
            .thread_name("Compute Task Pool".to_string())
            .build()
    });
    let a: Vec<_> = (0..40).collect();
    let t0 = Instant::now();
    a.par_iter().for_each(|v| {
        let now = Instant::now();
        while Instant::now() - now < Duration::from_millis(100) {
            // spin, simulating work being done
        }
        println!(
            "Thread {:?} index {} finished",
            std::thread::current().id(),
            v
        );
    });
    let t1 = Instant::now();
    println!("all tasks finished in {} secs", (t1 - t0).as_secs_f32());
}
