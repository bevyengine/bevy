---
title: Removed Simple Executor
pull_requests: [18741]
---

Bevy has removed `SimpleExecutor`, one of the `SystemExecutor`s in Bevy alongside `SingleThreadedExecutor` and `MultiThreadedExecutor` (which aren't going anywhere any time soon).
The `MultiThreadedExecutor` is great at large schedules and async heavy work, and the `SingleThreadedExecutor` is good at smaller schedules or schedules that have fewer parallelizable systems.
So what was `SimpleExecutor` good at? Not much. That's why it was removed. If you're curious, it was originally created to function without any schedule "sync points", which apply deferred commands.
That was a nice convenience in Bevy's past, but today, sync points are automatically added as needed, so `SimpleExecutor` no longer has a use case.
Removing it reduced some maintenance and consistency burdons on maintainers, allowing them to focus on more exciting features!
If you were using `SimpleExecutor`, consider upgrading to `SingleThreadedExecutor` instead, or try `MultiThreadedExecutor` if if fits the schedule.
