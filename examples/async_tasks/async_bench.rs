use bevy::{
    app::{AppExit, ScheduleRunnerSettings},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use core::task::Context;
use futures_lite::{
    future::{block_on, poll_once},
    Future,
};
use std::{
    sync::{Arc, LockResult, RwLock, RwLockReadGuard},
    task::{Poll, Waker},
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
// [no_poll_once]        n_frames executed: 238, avg fps: 39.3(target:120), duration: 6.048s
// [noop_waker]          n_frames executed: 161, avg fps: 25.7(target:120), duration: 6.253s
// [handle_tasks]        n_frames executed: 54, avg fps: 9.4(target:120), duration: 5.743s
// [handle_tasks_par]    n_frames executed: 124, avg fps: 21.5(target:120), duration: 5.767s
// [handle_tasks_par_2]  n_frames executed: 90, avg fps: 15.4(target:120), duration: 5.835s
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
    App::new()
        .insert_resource(ScheduleRunnerSettings::run_loop(Duration::from_secs_f64(
            FRAME_STEP,
        )))
        .insert_resource(FrameCounter { n_frames: 0 })
        .add_plugins(MinimalPlugins)
        .add_startup_system(spawn_tasks_noop_waker)
        .add_system_to_stage(CoreStage::First, count_frame)
        .add_system(handle_tasks_noop_waker)
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

#[derive(Debug)]
struct TaskWrapper<T> {
    result: Arc<RwLock<Option<T>>>,
    _task: Task<()>,
}

impl<T> TaskWrapper<T> {
    pub fn new(result: Arc<RwLock<Option<T>>>, task: Task<()>) -> Self {
        Self {
            result,
            _task: task,
        }
    }

    pub fn poll(&self) -> LockResult<RwLockReadGuard<Option<T>>> {
        self.result.read()
    }
}

fn spawn_tasks_no_poll_once(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    for step in 0..N_STEPS {
        for _i in 0..N_TASKS {
            let result: Arc<RwLock<Option<()>>> = Arc::new(RwLock::new(None));
            let result_clone = result.clone();
            let task = thread_pool.spawn(async move {
                let start_time = Instant::now();
                let duration = Duration::from_secs_f32(TASK_DURATION_SEC * (step as f32));
                while Instant::now() - start_time < duration {
                    futures_timer::Delay::new(Duration::from_secs_f32(0.1)).await;
                }
                // println!("spawn_tasks_no_poll_once");
                let mut locked = result_clone.write().unwrap();
                *locked = Some(());
            });
            let wrapper = TaskWrapper::new(result, task);
            commands.spawn().insert(wrapper);
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
        let locked = task.poll().unwrap();
        if locked.is_some() {
            commands.entity(entity).remove::<TaskWrapper<()>>();
        }
    }
    if n_tasks == 0 {
        print_statistics("no_poll_once", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}

lazy_static::lazy_static! {
    static ref NOOP_WAKER: Waker = noop_waker::noop_waker();
}

#[derive(Debug)]
struct TaskWrapper2<T> {
    task: Task<T>,
}

impl<T: Clone> TaskWrapper2<T> {
    pub fn new(task: Task<T>) -> Self {
        Self { task }
    }

    pub fn poll(&mut self) -> Option<T> {
        let f = &mut self.task;
        futures_lite::pin!(f);
        let mut noop_ctx = Context::from_waker(&NOOP_WAKER);
        match f.poll(&mut noop_ctx) {
            Poll::Ready(o) => Some(o.clone()),
            Poll::Pending => None,
        }
    }
}

fn spawn_tasks_noop_waker(mut commands: Commands, thread_pool: Res<AsyncComputeTaskPool>) {
    for step in 0..N_STEPS {
        for _i in 0..N_TASKS {
            let task = thread_pool.spawn(async move {
                let start_time = Instant::now();
                let duration = Duration::from_secs_f32(TASK_DURATION_SEC * (step as f32));
                while Instant::now() - start_time < duration {
                    futures_timer::Delay::new(Duration::from_secs_f32(0.1)).await;
                }
                // println!("spawn_tasks_noop_waker");
            });
            let wrapper = TaskWrapper2::<()>::new(task);
            commands.spawn().insert(wrapper);
        }
    }
}

fn handle_tasks_noop_waker(
    mut commands: Commands,
    mut transform_tasks: Query<(Entity, &mut TaskWrapper2<()>)>,
    mut app_exit_events: EventWriter<AppExit>,
    time: Res<Time>,
    frame_counter: Res<FrameCounter>,
) {
    let mut n_tasks = 0;
    for (entity, mut task) in transform_tasks.iter_mut() {
        n_tasks += 1;
        if task.poll().is_some() {
            commands.entity(entity).remove::<TaskWrapper2<()>>();
        }
    }
    if n_tasks == 0 {
        print_statistics("noop_waker", &frame_counter, &time);
        app_exit_events.send(AppExit);
    }
}
