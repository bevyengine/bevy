# bevy_tasks

A refreshingly simple task executor for bevy. :)

This is a simple threadpool with minimal dependencies. The main usecase is a scoped fork-join, i.e. spawning tasks from
a single thread and having that thread await the completion of those tasks. This is intended specifically for 
[`bevy`][bevy] as a lighter alternative to [`rayon`][rayon] for this specific usecase. There are also utilities for
generating the tasks from a slice of data. This library is intended for games and makes no attempt to ensure fairness 
or ordering of spawned tasks.

It is based on [`async-executor`][async-executor], a lightweight executor that allows the end user to manage their own threads.
`async-executor` is based on async-task, a core piece of async-std.

[bevy]: https://bevyengine.org
[rayon]: https://github.com/rayon-rs/rayon
[async-executor]: https://github.com/stjepang/async-executor

## Dependencies

A very small dependency list is a key feature of this module

```
├── async-executor
│   ├── async-task
│   ├── concurrent-queue
│   │   └── cache-padded
│   └── fastrand
├── num_cpus
│   └── libc
├── parking
└── futures-lite
```
