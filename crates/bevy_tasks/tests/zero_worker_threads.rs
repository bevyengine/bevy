//! `ParallelSlice` on a pool with no worker threads, which `cfg(test)` can't build.

#![cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]

use bevy_tasks::{ParallelSlice, ParallelSliceMut, TaskPool, TaskPoolBuilder};

fn no_worker_threads() -> TaskPool {
    let task_pool = TaskPoolBuilder::new().num_threads(0).build();
    assert_eq!(task_pool.thread_num(), 0);
    task_pool
}

#[test]
fn par_splat_map_runs_on_a_pool_with_no_worker_threads() {
    let v = vec![42; 1000];
    let outputs = v.par_splat_map(&no_worker_threads(), None, |_, numbers| -> i32 {
        numbers.iter().sum()
    });

    assert_eq!(outputs.iter().sum::<i32>(), 1000 * 42);
}

#[test]
fn par_splat_map_mut_runs_on_a_pool_with_no_worker_threads() {
    let mut v = vec![42; 1000];
    let outputs = v.par_splat_map_mut(&no_worker_threads(), None, |_, numbers| -> i32 {
        for number in numbers.iter_mut() {
            *number *= 2;
        }
        numbers.iter().sum()
    });

    assert_eq!(outputs.iter().sum::<i32>(), 1000 * 42 * 2);
    assert_eq!(v[0], 84);
}
