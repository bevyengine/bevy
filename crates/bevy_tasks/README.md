# small-task

This is a simple threadpool with minimal dependencies. The main usecase is a scoped fork-join, i.e. spawning tasks from
a single thread and having that thread await the completion of those tasks. This is intended specifically for 
[`bevy`][bevy] as a lighter alternative to [`rayon`][rayon] for this specific usecase. There are also utilities for
generating the tasks from a slice of data. This library is intended for games and makes no attempt to ensure fairness 
or ordering of spawned tasks.

It is based on [`multitask`][multitask], a lightweight executor that allows the end user to manage their own threads.
`multitask` is based on async-task, a core piece of async-std.

[bevy]: https://bevyengine.org
[rayon]: https://github.com/rayon-rs/rayon
[multitask]: https://github.com/stjepang/multitask

## Dependencies

A very small dependency list is a key feature of this library

```
├── multitask
│   ├── async-task
│   ├── concurrent-queue
│   │   └── cache-padded
│   └── fastrand
├── num_cpus
│   └── libc
├── parking
└── pollster
```

## Status

This is an unpublished prototype intended for eventual inclusion with `bevy`.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT).
