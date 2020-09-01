use bevy::{prelude::*, tasks::prelude::*};
use std::{
    sync::{atomic, atomic::AtomicUsize},
    thread,
    time::{Duration, Instant},
};

fn spawn_system(mut commands: Commands) {
    for i in 0..16usize {
        commands.spawn((i,));
    }
}

fn square_system(pool: Res<ComputeTaskPool>, mut nums: Query<&mut usize>) {
    let i = AtomicUsize::new(0);
    nums.iter().iter_batched(1).for_each(&pool, |mut n| {
        println!(
            "Processing entity {}",
            i.fetch_add(1, atomic::Ordering::Relaxed)
        );
        thread::sleep(Duration::from_secs(1));
        *n = *n * *n;
    });
}

fn print_threads_system(pool: Res<ComputeTaskPool>) {
    println!("Using {} threads in compute pool", pool.thread_num());
}

fn print_system(num: &usize) {
    print!("{} ", num);
}

fn main() {
    let t0 = Instant::now();
    App::build()
        .add_startup_system(spawn_system.system())
        .add_startup_system(print_threads_system.system())
        .add_system(square_system.system())
        .add_system(print_system.system())
        .run();
    let t1 = Instant::now();
    println!("\nTook {:.3}s", (t1 - t0).as_secs_f32());
}
