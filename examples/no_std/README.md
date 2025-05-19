# `no_std` Examples

This folder contains examples for how to work with `no_std` targets and Bevy.
Refer to each example individually for details around how it works and what features you may need to enable/disable to allow a particular target to work.

## What is `no_std`?

`no_std` is a Rust term for software which doesn't rely on the standard library, [`std`](https://doc.rust-lang.org/stable/std/).
The typical use for `no_std` is in embedded software, where the device simply doesn't support the standard library.
For example, a [Raspberry Pi Pico](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html) has no operating system to support threads or filesystem operations.

For these platforms, Rust has a more fundamental alternative to `std`, [`core`](https://doc.rust-lang.org/stable/core/).
A large portion of Rust's `std` actually just re-exports items from `core`, such as iterators, `Result`, and `Option`.

In addition, `std` also re-exports from another crate, [`alloc`](https://doc.rust-lang.org/stable/alloc/).
This crate is similar to `core` in that it's generally available on all platforms.
Where it differs is that its inclusion requires access to a [global allocator](https://doc.rust-lang.org/stable/std/alloc/trait.GlobalAlloc.html).
Currently, Bevy relies heavily on allocation, so we consider `alloc` to be just as available, since without it, Bevy will not compile.
