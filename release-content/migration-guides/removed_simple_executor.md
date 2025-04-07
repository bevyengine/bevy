---
title: Removed Simple Executor
pull_requests: [18741]
---

Bevy has removed `SimpleExecutor`, one of the `SystemExecutor`s in Bevy alongside `SingleThreadedExecutor` and `MultiThreadedExecutor` (which aren't going anywhere any time soon).
The `MultiThreadedExecutor` is great at large schedules and async heavy work, and the `SingleThreadedExecutor` is good at smaller schedules or schedules that have fewer parallelizable systems.
So what was `SimpleExecutor` good at? Not much. That's why it was removed. Removing it reduced some maintenance and consistency burdons on maintainers, allowing them to focus on more exciting features!

If you were using `SimpleExecutor`, consider upgrading to `SingleThreadedExecutor` instead, or try `MultiThreadedExecutor` if if fits the schedule.
It's worth mentioning that `SimpleExecutor` ran deferred commands inbetween *each* system, regardless of it it was needed.
The other executors are more efficient about this, but that means they need extra information about when to run those commands.
In most schedules, that information comes from the contents and ordering of systems, via `before`, `after`, `chain`, etc.
If a schedule that was previously using `SimpleExecutor` still needs commands from one system to be applied before another system runs,
make sure that ordering is enforced explicitly by these methods, rather than implicitly by the order of `add_systems`.
