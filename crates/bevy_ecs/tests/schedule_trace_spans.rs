//! Checks the tracing spans the multi-threaded executor enters on worker threads.
//!
//! This is its own test binary because it installs a global subscriber, the only kind that
//! the executor's worker threads can see.

#![cfg(all(feature = "trace", feature = "multi_threaded"))]

use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
    thread::{self, ThreadId},
};

use bevy_ecs::{
    schedule::{MultiThreadedExecutor, Schedule},
    world::World,
};
use bevy_tasks::{ComputeTaskPool, TaskPoolBuilder};
use tracing::{span, Event, Metadata, Subscriber};
use tracing_core::span::Current;

#[derive(Default)]
struct Recorded {
    spans: Vec<&'static Metadata<'static>>,
    /// The ids of the spans each thread has entered, innermost last.
    stacks: HashMap<ThreadId, Vec<u64>>,
    /// The thread that emitted each event, and the names of the spans it had entered at the time.
    events: Vec<(ThreadId, Vec<&'static str>)>,
}

static RECORDED: LazyLock<Mutex<Recorded>> = LazyLock::new(Default::default);

/// A minimal subscriber that tracks entered spans per thread, like `tracing_subscriber::Registry`.
struct Recorder;

impl Subscriber for Recorder {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, span: &span::Attributes<'_>) -> span::Id {
        let mut recorded = RECORDED.lock().unwrap();
        recorded.spans.push(span.metadata());
        span::Id::from_u64(recorded.spans.len() as u64)
    }

    fn record(&self, _span: &span::Id, _values: &span::Record<'_>) {}

    fn record_follows_from(&self, _span: &span::Id, _follows: &span::Id) {}

    fn event(&self, _event: &Event<'_>) {
        let mut recorded = RECORDED.lock().unwrap();
        let thread = thread::current().id();
        let names = recorded
            .stacks
            .get(&thread)
            .into_iter()
            .flatten()
            .map(|&id| recorded.spans[id as usize - 1].name())
            .collect();
        recorded.events.push((thread, names));
    }

    fn enter(&self, span: &span::Id) {
        let mut recorded = RECORDED.lock().unwrap();
        recorded
            .stacks
            .entry(thread::current().id())
            .or_default()
            .push(span.into_u64());
    }

    fn exit(&self, _span: &span::Id) {
        let mut recorded = RECORDED.lock().unwrap();
        if let Some(stack) = recorded.stacks.get_mut(&thread::current().id()) {
            stack.pop();
        }
    }

    fn current_span(&self) -> Current {
        let recorded = RECORDED.lock().unwrap();
        match recorded
            .stacks
            .get(&thread::current().id())
            .and_then(|stack| stack.last())
        {
            Some(&id) => Current::new(span::Id::from_u64(id), recorded.spans[id as usize - 1]),
            None => Current::none(),
        }
    }
}

fn log_from_system() {
    tracing::info!("running");
}

#[test]
fn systems_on_worker_threads_are_traced_inside_their_schedule() {
    tracing::subscriber::set_global_default(Recorder).unwrap();
    ComputeTaskPool::get_or_init(|| TaskPoolBuilder::new().num_threads(4).build());

    let mut schedule = Schedule::default();
    schedule.set_executor(MultiThreadedExecutor::new());
    for _ in 0..8 {
        schedule.add_systems(log_from_system);
    }
    schedule.run(&mut World::new());

    let recorded = RECORDED.lock().unwrap();
    let test_thread = thread::current().id();
    assert!(
        recorded
            .events
            .iter()
            .any(|(thread, _)| *thread != test_thread),
        "expected at least one system to run on a worker thread"
    );
    for (_, stack) in &recorded.events {
        assert_eq!(
            stack.iter().filter(|name| **name == "schedule").count(),
            1,
            "expected exactly one `schedule` span around every system, got {stack:?}"
        );
    }
}
