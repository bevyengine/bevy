# Bevy Tasks

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy.svg)](https://crates.io/crates/bevy_tasks)
[![Downloads](https://img.shields.io/crates/d/bevy_tasks.svg)](https://crates.io/crates/bevy_tasks)
[![Docs](https://docs.rs/bevy_tasks/badge.svg)](https://docs.rs/bevy_tasks/latest/bevy_tasks/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

A refreshingly simple task executor for bevy. :)

This is a simple threadpool with minimal dependencies. The main usecase is a scoped fork-join, i.e. spawning tasks from
a single thread and having that thread await the completion of those tasks. This is intended specifically for
[`bevy`][bevy] as a lighter alternative to [`rayon`][rayon] for this specific usecase. There are also utilities for
generating the tasks from a slice of data. This library is intended for games and makes no attempt to ensure fairness
or ordering of spawned tasks.

It is based on a fork of [`async-executor`][async-executor], a lightweight executor that allows the end user to manage their own threads.
`async-executor` is based on [`async-task`][async-task], a core piece of [`smol`][smol].

## Usage

In order to be able to optimize task execution in multi-threaded environments,
Bevy supports a thread pool via which tasks of different priorities can be spawned.
(The same API is used in single-threaded environments, even if execution is limited to a single thread.
This currently applies to Wasm targets.)
The determining factor for how work is prioritized based on latency requirements:

* For CPU-intensive work (tasks that generally spin until completion) we have a standard
  `Compute` priority, the default. Work that does not need to be completed to present the 
  next frame be set to the `BlockingCompute` priority.

* For IO-intensive work (tasks that spend very little time in a "woken" state) we have an
  [`AsyncIO`] priority whose tasks are expected to complete very quickly. Generally speaking, they should just
  await receiving data from somewhere (i.e. network) and signal other systems when the data is ready
  for consumption. (likely via channels)

## `no_std` Support

To enable `no_std` support in this crate, you will need to disable default features, and enable the `edge_executor` and `critical-section` features.

[bevy]: https://bevy.org
[rayon]: https://github.com/rayon-rs/rayon
[async-executor]: https://github.com/smol-rs/async-executor
[smol]: https://github.com/smol-rs/smol
[async-task]: https://github.com/smol-rs/async-task
