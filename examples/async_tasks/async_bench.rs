use bevy::{
    app::{AppExit, ScheduleRunnerSettings},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future::{block_on, poll_once};
use std::{
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

const TASK_DURATION_SEC: f32 = 0.5;
const FPS: f64 = 120.0;
const FRAME_STEP: f64 = 1.0 / FPS;
const N_STEPS: usize = 10;
const N_TASKS: usize = 100000;

struct FrameCounter {
    pub n_frames: usize,
}

// This example benchmarks performance of concurrent custom task handling
// Run with release build: cargo run --release --example async_bench
// Example output:
// windows:
// [handle_tasks]        n_frames executed: 64, avg fps: 11.2(target:120), duration: 5.704s
// [handle_tasks_par]    n_frames executed: 121, avg fps: 20.8(target:120), duration: 5.805s
// [handle_tasks_par_2]  n_frames executed: 81, avg fps: 14.0(target:120), duration: 5.769s
// linux:
// [handle_tasks]        n_frames executed: 215, avg fps: 16.2(target:120), duration: 13.307s
// [handle_tasks_par]    n_frames executed: 332, avg fps: 26.2(target:120), duration: 12.675s
// [handle_tasks_par_2]  n_frames executed: 252, avg fps: 22.1(target:120), duration: 11.389s
fn main() {
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            FRAME_STEP,
        )))
        .insert_resource(FrameCounter { n_frames: 0 })
        .add_plugins(MinimalPlugins)
        .add_startup_system(spawn_tasks_no_poll_once)
        .add_system_to_stage(CoreStage::First, count_frame)
        .add_system(handle_tasks_no_poll_once)
        .run();
    for handle_tasks_system in [handle_tasks, handle_tasks_par, handle_tasks_par_2] {
        App::new()
            .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
                FRAME_STEP,
            )))
            .insert_resource(FrameCounter { n_frames: 0 })
            .add_plugins(MinimalPlugins)
            .add_startup_system(spawn_tasks)
            .add_system_to_stage(CoreStage::First, count_frame)
            .add_system(handle_tasks_system)
            .run();
    }
}

fn spawn_tasks(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    for step in 0..N_STEPS {
        for _i in 0..N_TASKS {
            let task = thread_pool.spawn(async move {
                let start_time = Instant::now();
                let duration = Duration::from_secs_f32(TASK_DURATION_SEC * (step as f32));
                while Instant::now() - start_time < duration {
                    futures_timer::Delay::new(Duration::from_secs_f32(0.1)).await
                }
                true
            });
            commands.spawn().insert(task);
        }
    }
}

fn count_frame(mut frame_counter: ResMut<FrameCounter>) {
    frame_counter.n_frames += 1;
}

fn handle_tasks(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut Task<bool>)>,
    mut app_exit_events: EventWriter<AppExit>,
    time: Res<Time>,
    frame_counter: Res<FrameCounter>,
) {
    let mut n_tasks = 0;
    for (entity, mut task) in transform_tasks.iter_mut() {
        n_tasks += 1;
        let ret = block_on(async { poll_once(&mut *task).await });
        if ret.is_some() {
            commands.entity(entity).remove::<Task<bool>>();
        }
    }
    if n_tasks == 0 {
        print_statistics("handle_tasks", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}

fn handle_tasks_par(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut Task<bool>)>,
    mut app_exit_events: EventWriter<AppExit>,
    time: Res<Time>,
    frame_counter: Res<FrameCounter>,
) {
    let futures = transform_tasks
        .iter_mut()
        .map(|(entity, mut task)| async move {
            if poll_once(&mut *task).await.is_some() {
                Some(entity)
            } else {
                None
            }
        });
    let mut n_tasks = 0;
    block_on(async {
        for f in futures {
            n_tasks += 1;
            if let Some(entity) = f.await {
                commands.entity(entity).remove::<Task<bool>>();
            }
        }
    });
    if n_tasks == 0 {
        print_statistics("handle_tasks_par", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}

fn handle_tasks_par_2(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut Task<bool>)>,
    mut app_exit_events: EventWriter<AppExit>,
    time: Res<Time>,
    frame_counter: Res<FrameCounter>,
) {
    let futures = transform_tasks
        .iter_mut()
        .map(|(entity, mut task)| async move {
            if poll_once(&mut *task).await.is_some() {
                Some(entity)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let n_tasks = futures.len();
    block_on(async {
        for f in futures {
            if let Some(entity) = f.await {
                commands.entity(entity).remove::<Task<bool>>();
            }
        }
    });
    if n_tasks == 0 {
        print_statistics("handle_tasks_par_2", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}

fn print_statistics(name: &str, frame_counter: &Res<FrameCounter>, time: &Res<Time>) {
    let duration_sec = time.seconds_since_startup();
    println!(
        "{:width$}n_frames executed: {}, avg fps: {:.1}(target:{}), duration: {:.3}s",
        format!("[{}]", name),
        frame_counter.n_frames,
        (frame_counter.n_frames as f64) / duration_sec,
        FPS,
        duration_sec,
        width = 22,
    );
}

#[derive(Clone)]
struct TaskWrapper<T> {
    pub result: Arc<RwLock<Option<T>>>,
}

impl<T> TaskWrapper<T> {
    pub fn new() -> Self {
        Self {
            result: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn register(&mut self, t: impl std::future::Future<Output = T>) {
        let ret = t.await;
        let mut lock = self.result.write().unwrap();
        *lock = Some(ret);
    }
}

fn spawn_tasks_no_poll_once(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    for step in 0..N_STEPS {
        for _i in 0..N_TASKS {
            let wrapper = TaskWrapper::<()>::new();
            let mut wrapper_clone = wrapper.clone();
            let task = thread_pool.spawn(async move {
                wrapper_clone
                    .register(async move {
                        let start_time = Instant::now();
                        let duration = Duration::from_secs_f32(TASK_DURATION_SEC * (step as f32));
                        while Instant::now() - start_time < duration {
                            futures_timer::Delay::new(Duration::from_secs_f32(0.1)).await;
                        }
                    })
                    .await;
            });
            commands.spawn().insert(wrapper.clone()).insert(task);
        }
    }
}

fn handle_tasks_no_poll_once(
    mut commands: Commands,
    transform_tasks: Query<(Entity, &TaskWrapper<()>)>,
    mut app_exit_events: EventWriter<AppExit>,
    time: Res<Time>,
    frame_counter: Res<FrameCounter>,
) {
    let mut n_tasks = 0;
    for (entity, task) in transform_tasks.iter() {
        n_tasks += 1;
        let lock = task.result.read().unwrap();
        if lock.is_some() {
            commands
                .entity(entity)
                .remove::<TaskWrapper<()>>()
                .remove::<Task<()>>();
        }
    }
    if n_tasks == 0 {
        print_statistics("handle_tasks_no_poll_once", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}
