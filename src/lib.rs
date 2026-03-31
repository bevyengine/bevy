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

// Workaround for https://github.com/rust-lang/rust/issues/141256. Normally, if
// a user references a module that's behind a disabled feature then the compiler
// diagnostic will mention the feature. But this doesn't happen if the module
// came through a glob import like the above `bevy_internal::*`. The workaround
// is to directly import the modules and duplicate the bevy_internal feature gates.
#[cfg(feature = "bevy_window")]
pub use bevy_internal::a11y;
#[cfg(feature = "bevy_animation")]
pub use bevy_internal::animation;
#[cfg(feature = "bevy_anti_aliasing")]
pub use bevy_internal::anti_aliasing;
pub use bevy_internal::app;
#[cfg(feature = "bevy_asset")]
pub use bevy_internal::asset;
#[cfg(feature = "bevy_audio")]
pub use bevy_internal::audio;
#[cfg(feature = "bevy_color")]
pub use bevy_internal::color;
#[cfg(feature = "bevy_core_pipeline")]
pub use bevy_internal::core_pipeline;
#[cfg(feature = "bevy_dev_tools")]
pub use bevy_internal::dev_tools;
pub use bevy_internal::diagnostic;
pub use bevy_internal::ecs;
#[cfg(feature = "bevy_gilrs")]
pub use bevy_internal::gilrs;
#[cfg(feature = "bevy_gizmos")]
pub use bevy_internal::gizmos;
#[cfg(feature = "bevy_gltf")]
pub use bevy_internal::gltf;
#[cfg(feature = "bevy_image")]
pub use bevy_internal::image;
pub use bevy_internal::input;
#[cfg(feature = "bevy_input_focus")]
pub use bevy_internal::input_focus;
#[cfg(feature = "bevy_log")]
pub use bevy_internal::log;
pub use bevy_internal::math;
#[cfg(feature = "bevy_pbr")]
pub use bevy_internal::pbr;
#[cfg(feature = "bevy_picking")]
pub use bevy_internal::picking;
pub use bevy_internal::platform;
pub use bevy_internal::ptr;
pub use bevy_internal::reflect;
#[cfg(feature = "bevy_remote")]
pub use bevy_internal::remote;
#[cfg(feature = "bevy_render")]
pub use bevy_internal::render;
#[cfg(feature = "bevy_scene")]
pub use bevy_internal::scene;
#[cfg(feature = "bevy_sprite")]
pub use bevy_internal::sprite;
#[cfg(feature = "bevy_state")]
pub use bevy_internal::state;
pub use bevy_internal::tasks;
#[cfg(feature = "bevy_text")]
pub use bevy_internal::text;
pub use bevy_internal::time;
pub use bevy_internal::transform;
#[cfg(feature = "bevy_ui")]
pub use bevy_internal::ui;
pub use bevy_internal::utils;
#[cfg(feature = "bevy_window")]
pub use bevy_internal::window;
#[cfg(feature = "bevy_winit")]
pub use bevy_internal::winit;

// Wasm does not support dynamic linking.
#[cfg(all(feature = "dynamic_linking", not(target_family = "wasm")))]
#[expect(
    unused_imports,
    clippy::single_component_path_imports,
    reason = "This causes Bevy to be compiled as a dylib when using dynamic linking and therefore cannot be removed or changed without affecting dynamic linking."
)]
use bevy_dylib;
