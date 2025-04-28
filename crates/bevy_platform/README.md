# Bevy Platform Support

[![License](https://img.shields.io/badge/license-MIT%2FApache-blue.svg)](https://github.com/bevyengine/bevy#license)
[![Crates.io](https://img.shields.io/crates/v/bevy_platform.svg)](https://crates.io/crates/bevy_platform)
[![Downloads](https://img.shields.io/crates/d/bevy_platform.svg)](https://crates.io/crates/bevy_platform)
[![Docs](https://docs.rs/bevy_platform/badge.svg)](https://docs.rs/bevy_platform/latest/bevy_platform/)
[![Discord](https://img.shields.io/discord/691052431525675048.svg?label=&logo=discord&logoColor=ffffff&color=7389D8&labelColor=6A7EC2)](https://discord.gg/bevy)

Rust is a fantastic multi-platform language with extensive support for modern targets through its [standard library](https://doc.rust-lang.org/stable/std/).
However, some items within the standard library have alternatives that are better suited for [Bevy](https://crates.io/crates/bevy) and game engines in general.
Additionally, to support embedded and other esoteric platforms, it's often necessary to shed reliance on `std`, making your crate [`no_std`](https://docs.rust-embedded.org/book/intro/no-std.html).

These needs are handled by this crate, `bevy_platform`.
The goal of this crate is to provide alternatives and extensions to the Rust standard library which minimize friction when developing with and for Bevy across multiple platforms.

## Getting Started

Like any dependency from [crates.io](https://crates.io/), use `cargo` to add it to your `Cargo.toml` file:

```sh
cargo add bevy_platform
```

Now, instead of importing from `std` you can use `bevy_platform` for items it has alternative for.
See the documentation for what items are available, and explanations for _why_ you may want to use them.

## `no_std` Support

By default, `bevy_platform` will activate the `std` feature, requiring access to the `std` crate for whichever platforms you're targeting.
To use this crate on `no_std` platforms, disable default features:

```toml
bevy_platform = { version = "x.y.z", default-features = false }
```

## Features

### `std` (_default_)

Enables usage of the standard library. Note that where this crate has alternatives to the standard library that it considers _better_ than what's provided, it will provide the alternative even when `std` is enabled.
This is explicitly incompatible with `no_std` targets.

### `alloc` (_default_)

Enables usage of the [`alloc`](https://doc.rust-lang.org/stable/alloc/) crate. Note that this feature is automatically enabled when enabling `std`.
This is compatible with most `no_std` targets, but not all.

### `critical-section`

Switches to using [`critical-section`](https://docs.rs/critical-section/latest/critical_section/) as a backend for synchronization.
You may need to enable this feature on platforms with little to no support for atomic operations.
