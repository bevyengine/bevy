use bevy_tasks::TaskPoolBuilder;
use web_time::{Duration, Instant};

// This sample demonstrates creating a thread pool with 4 tasks and spawning 40 tasks that spin
// for 100ms. It's expected to take about a second to run (assuming the machine has >= 4 logical
// cores)

fn main() {
    let pool = TaskPoolBuilder::new()
        .thread_name("Busy Behavior ThreadPool".to_string())
        .num_threads(5)
        .build();

    println!("main {:?}", std::thread::current().id());
    const iter_count: usize = 1000;
    let t0 = Instant::now();
    for _ in 0..iter_count {
        pool.scope(|s| {
            for i in 0..20 {
                s.spawn(async move {
                    let now = Instant::now();
                    while Instant::now() - now < Duration::from_micros(1000) {
                        // spin, simulating work being done
                    }

                    // println!(
                    //     "Thread {:?} index {} finished",
                    //     std::thread::current().id(),
                    //     i
                    // );
                });
            }
        });
    }

    println!(" par {:?} elapsed", t0.elapsed() / iter_count as u32);
}
