#![warn(missing_docs)]
#![allow(clippy::single_component_path_imports)]

//! Forces dynamic linking of Bevy.
//!
//! Dynamically linking Bevy makes the "link" step much faster. This can be achieved by adding
//! `bevy_dylib` as a dependency and adding the following code to the `main.rs` file:
//!
//! ```rust
//! #[allow(unused_imports)]
//! use bevy_dylib;
//! ```
//!
//! It is recommended to disable the `bevy_dylib` dependency in release mode by adding the
//! following code to the `use` statement:
//!
//! ```rust
//! #[allow(unused_imports)]
//! #[cfg(debug_assertions)] // new
//! use bevy_dylib;
//! ```
//!
//! If you don't do this you will have to ship `libstd.so` and `libbevy_dylib.so` with your game.

// Force linking of the main bevy crate
#[allow(unused_imports)]
use bevy_internal;
