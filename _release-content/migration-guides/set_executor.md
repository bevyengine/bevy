---
title: "`set_executor` replaced `ExecutorKind`"
pull_requests: [23414]
---

`ExecutorKind` has been removed. Schedules are now configured by passing an executor instance directly via `Schedule::set_executor`.

- `Schedule::set_executor_kind` has been removed. Use `Schedule::set_executor` instead.
- `Schedule::get_executor_kind` has been removed. There is no replacement; executors are no longer identified by an enum variant.
- `SystemExecutor::kind` has been removed from the trait.
- `SystemExecutor` is now a public trait. You can implement it to provide a fully custom executor.
- `SystemSchedule::systems` is now `pub`.

```rust
// 0.18
use bevy::ecs::schedule::ExecutorKind;

schedule.set_executor_kind(ExecutorKind::SingleThreaded);
schedule.set_executor_kind(ExecutorKind::MultiThreaded);
schedule.set_executor_kind(ExecutorKind::default());

// 0.19
use bevy::ecs::schedule::{SingleThreadedExecutor, MultiThreadedExecutor, default_executor};

schedule.set_executor(SingleThreadedExecutor::new());
schedule.set_executor(MultiThreadedExecutor::new());
schedule.set_executor(default_executor());
```
