# bevy_tasks

A refreshingly simple task executor for bevy. :)

This is a simple threadpool with minimal dependencies. The main usecase is a scoped fork-join, i.e. spawning tasks from
a single thread and having that thread await the completion of those tasks. This is intended specifically for
[`bevy`][bevy] as a lighter alternative to [`rayon`][rayon] for this specific usecase. There are also utilities for
generating the tasks from a slice of data. This library is intended for games and makes no attempt to ensure fairness
or ordering of spawned tasks.

It is based on [`async-executor`][async-executor], a lightweight executor that allows the end user to manage their own threads.
`async-executor` is based on async-task, a core piece of async-std.

## Usage

In order to be able to optimize task execution in multi-threaded environments,
bevy provides three different thread pools via which tasks of different kinds can be spawned.
(The same API is used in single-threaded environments, even if execution is limited to a single thread.
This currently applies to WASM targets.)
The determining factor for what kind of work should go in each pool is latency requirements:

* For CPU-intensive work (tasks that generally spin until completion) we have a standard
  [`ComputeTaskPool`] and an [`AsyncComputeTaskPool`]. Work that does not need to be completed to
  present the next frame should go to the [`AsyncComputeTaskPool`].

* For IO-intensive work (tasks that spend very little time in a "woken" state) we have an
  [`IoTaskPool`] whose tasks are expected to complete very quickly. Generally speaking, they should just
  await receiving data from somewhere (i.e. disk) and signal other systems when the data is ready
  for consumption. (likely via channels)

[bevy]: https://bevyengine.org
[rayon]: https://github.com/rayon-rs/rayon
[async-executor]: https://github.com/stjepang/async-executor
