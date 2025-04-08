---
title: Deprecated Simple Executor
pull_requests: [18753]
---

Bevy has deprecated `SimpleExecutor`, one of the `SystemExecutor`s in Bevy alongside `SingleThreadedExecutor` and `MultiThreadedExecutor` (which aren't going anywhere any time soon).

The `SimpleExecutor` leaves performance on the table compared to the other executors in favor of simplicity.
Specifically, `SimpleExecutor` applies any commands a system produces right after it finishes, so every system starts with a clean `World` with no pending commands.
As a result, the default `SimpleExecutor` runs all systems in the order they are added to the schedule, though more ordering constraints can be applied, like `before`, `after`, `chain`, etc.
In other executors, these ordering onstraints also inform the executor exactly where to apply commands.
For example, if system `A` produces commands and runs `before` system `B`, `A`'s commands will be applied before `B` starts.
However, the `before` ordering is implicit in `SimpleExecutor` if `A` is added to the schedule before `B`.

The dueling behavior between ordering systems based on when they were added to a schedule as opposed to using ordering constraints is difficult to maintain and can be confusing, especially for new users.
But, if you have a strong preference for the existing behavior of `SimpleExecutor`, please make an issue and we can discuss your needs.

If you were using `SimpleExecutor`, consider upgrading to `SingleThreadedExecutor` instead, or try `MultiThreadedExecutor` if it fits the schedule.
The `MultiThreadedExecutor` is great at large schedules and async heavy work, and the `SingleThreadedExecutor` is good at smaller schedules or schedules that have fewer parallelizable systems.
So what was `SimpleExecutor` good at? Not much. That's why we plan to remove it. Removing it will reduce some maintenance and consistency burdens, allowing us to focus on more exciting features!

When migrating, you might uncover bugs where one system depends on another's commands but is not ordered to reflect that.
These bugs can be fixed by making those implicit orderings explicit via constraints like `before`, `after`, `chain`, etc.
If finding all of those implicit but necessary orderings is unrealistic, `chain` can also be used to mimic the behavior of the `SimpleExecutor`.
Again, if you run into trouble migrating, feel free to open an issue!
