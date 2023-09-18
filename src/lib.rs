#![allow(clippy::single_component_path_imports)]

//! [![](https://bevyengine.org/assets/bevy_logo_docs.svg)](https://bevyengine.org)
//!
//! Bevy is an open-source modular game engine built in Rust, with a focus on developer productivity
//! and performance.
//!
//! Check out the [Bevy website](https://bevyengine.org) for more information, read the
//! [Bevy Book](https://bevyengine.org/learn/book/introduction) for a step-by-step guide, and [engage with our
//! community](https://bevyengine.org/community/) if you have any questions or ideas!
//!
//! ## Example
//!
//! Here is a simple "Hello World" Bevy app:
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
//! Don't let the simplicity of the example above fool you. Bevy is a [fully featured game engine](https://bevyengine.org)
//! and it gets more powerful every day!
//!
//! ## This Crate
//!
//! The `bevy` crate is just a container crate that makes it easier to consume Bevy subcrates.
//! The defaults provide a "full" engine experience, but you can easily enable / disable features
//! in your project's `Cargo.toml` to meet your specific needs. See Bevy's `Cargo.toml` for a full
//! list of features available.
//!
//! If you prefer, you can also consume the individual bevy crates directly.
//! Each module in the root of this crate, except for the prelude, can be found on crates.io
//! with `bevy_` appended to the front, e.g. `app` -> [`bevy_app`](https://docs.rs/bevy_app/*/bevy_app/).
#![doc = include_str!("../docs/cargo_features.md")]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub use bevy_internal::*;

#[cfg(feature = "dynamic_linking")]
#[allow(unused_imports)]
use bevy_dylib;
