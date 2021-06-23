#![allow(clippy::single_component_path_imports)]

//! Forces dynamic linking of Bevy.
//!
//! Dynamically linking Bevy makes the "link" step much faster. This can be achieved by adding
//! `bevy_dylib` as dependency and `#[allow(unused_imports)] use bevy_dylib` to `main.rs`. It is
//! recommended to disable the `bevy_dylib` dependency in release mode by adding
//! `#[cfg(debug_assertions)]` to the `use` statement. Otherwise you will have to ship `libstd.so`
//! and `libbevy_dylib.so` with your game.

// Force linking of the main bevy crate
#[allow(unused_imports)]
use bevy_internal;
