#![cfg_attr(docsrs, feature(doc_cfg))]
#![expect(
    clippy::doc_markdown,
    reason = "Clippy lints for un-backticked identifiers within the cargo features list, which we don't want."
)]
//! [![Bevy Logo](https://bevy.org/assets/bevy_logo_docs.svg)](https://bevy.org)
//!
//! Bevy is an open-source, modular game engine built in Rust, with a focus on developer productivity
//! and performance.
//!
//! Check out the [Bevy website](https://bevy.org) for more information, read the
//! [Quick Start Guide](https://bevy.org/learn/quick-start/introduction) for a step-by-step introduction, and [engage with our
//! community](https://bevy.org/community/) if you have any questions or ideas!
//!
//! ## Example
//!
//! Here is a simple "Hello, World!" Bevy app:
//! ```
//! use bevy::prelude::*;
//!
//! fn main() {
//!    App::new()
//!        .add_systems(Update, hello_world_system)
//!        .run();
//! }
//!
//! fn hello_world_system() {
//!    println!("hello world");
//! }
//! ```
//!
//! Don't let the simplicity of the example above fool you. Bevy is a [fully featured game engine](https://bevy.org),
//! and it gets more powerful every day!
//!
//! ## This Crate
//!
//! The `bevy` crate is a container crate that makes it easier to consume Bevy subcrates.
//! The defaults provide a "full engine" experience, but you can easily enable or disable features
//! in your project's `Cargo.toml` to meet your specific needs. See Bevy's `Cargo.toml` for a full
//! list of available features.
//!
//! If you prefer, you can also use the individual Bevy crates directly.
//! Each module in the root of this crate, except for the prelude, can be found on crates.io
//! with `bevy_` appended to the front, e.g., `app` -> [`bevy_app`](https://docs.rs/bevy_app/*/bevy_app/).
#![doc = include_str!("../docs/cargo_features.md")]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

pub use bevy_internal::*;

// Wasm does not support dynamic linking.
#[cfg(all(feature = "dynamic_linking", not(target_family = "wasm")))]
#[expect(
    unused_imports,
    clippy::single_component_path_imports,
    reason = "This causes Bevy to be compiled as a dylib when using dynamic linking and therefore cannot be removed or changed without affecting dynamic linking."
)]
use bevy_dylib;
