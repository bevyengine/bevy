use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use criterion::Criterion;
use rand::random;
use std::time::{Duration, Instant};

fn s_system<const N: usize>() {
    let now = Instant::now();
    while Instant::now() - now < Duration::from_nanos(1) {
        // spin, simulating work being done
    }
}

macro_rules! chain_systems {
  ($schedule:ident;$($indent:tt),*) => {
      $schedule.add_systems(($(s_system::<$indent>,)*).chain());
  };
}

pub fn executor(c: &mut Criterion) {
    let mut group = c.benchmark_group("executor");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    group.bench_function("single-thread", |b| {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);

        // spawn 16 systems in chain
        chain_systems!(schedule;0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);

        schedule.initialize(&mut world);
        b.iter(move || {
            schedule.run(&mut world);
        });
    });

    ComputeTaskPool::get_or_init(TaskPool::default);
    let thread_num = ComputeTaskPool::get().thread_num();

    for system_count_per_batch in [1, 10, 50, 100] {
        group.bench_function(
            format!(
                "multi-thread({})-{}-per-batch",
                thread_num, system_count_per_batch
            ),
            |b| {
                let mut world = World::new();
                let mut schedule = Schedule::default();
                schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::MultiThreaded);

                // spawn 16 batches, each with `system_count_per_batch` systems per batch.
                for i in 0..system_count_per_batch {
                    chain_systems!(schedule;0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15);
                }

                schedule.initialize(&mut world);
                b.iter(move || {
                    schedule.run(&mut world);
                });
            },
        );
    }
}
