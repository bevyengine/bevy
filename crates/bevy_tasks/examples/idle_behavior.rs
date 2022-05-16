use bevy_tasks::TaskPool;

// This sample demonstrates a thread pool with one thread per logical core and only one task
// spinning. Other than the one thread, the system should remain idle, demonstrating good behavior
// for small workloads.

fn main() {
    let pool = TaskPool::build()
        .thread_name("Idle Behavior ThreadPool")
        .build();

    pool.scope(|s| {
        for i in 0..1 {
            s.spawn(async move {
                println!("Blocking for 10 seconds");
                let now = instant::Instant::now();
                while instant::Instant::now() - now < instant::Duration::from_millis(10000) {
                    // spin, simulating work being done
                }

                println!(
                    "Thread {:?} index {} finished",
                    std::thread::current().id(),
                    i
                );
            });
        }
    });

    println!("all tasks finished");
}
