//! [![](https://bevyengine.org/assets/bevy_logo_docs.svg)](https://bevyengine.org)
//!
//! Bevy is an open-source modular game engine built in Rust, with a focus on developer productivity and performance.
//!
//! Check out the [Bevy website](https://bevyengine.org) for more information, read the
//! [Bevy Book](https://bevyengine.org/learn/book/introduction) for a step-by-step guide, and [engage with our
//! community](https://bevyengine.org/community/) if you have any questions or ideas!
//!
//! ## Example
//!Here is a simple "Hello World" Bevy app:
//! ```no_run
//!use bevy::prelude::*;
//!
//!fn main() {
//!    App::build()
//!        .add_default_plugins()
//!        .add_system(hello_world_system.system())
//!        .run();
//!}
//!
//!fn hello_world_system() {
//!    println!("hello world");
//!}
//! ```

//! Don't let the simplicity of the example above fool you. Bevy is a [fully featured game engine](https://bevyengine.org/learn/book/introduction/features/)
//! and it gets more powerful every day!
//!
//! ### This Crate
//! The "bevy" crate is just a container crate that makes it easier to consume Bevy components.
//! The defaults provide a "full" engine experience, but you can easily enable / disable features
//! in your project's Cargo.toml to meet your specific needs. See Bevy's Cargo.toml for a full list of features available.
//!
//! If you prefer it, you can also consume the individual bevy crates directly.

#![feature(min_specialization)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

mod add_default_plugins;
pub mod prelude;

pub use add_default_plugins::*;
pub use bevy_app as app;
pub use glam as math;
pub use legion;

#[cfg(feature = "asset")]
pub use bevy_asset as asset;
#[cfg(feature = "type_registry")]
pub use bevy_type_registry as type_registry;
#[cfg(feature = "core")]
pub use bevy_core as core;
#[cfg(feature = "derive")]
pub use bevy_derive as derive;
#[cfg(feature = "diagnostic")]
pub use bevy_diagnostic as diagnostic;
#[cfg(feature = "gltf")]
pub use bevy_gltf as gltf;
#[cfg(feature = "input")]
pub use bevy_input as input;
#[cfg(feature = "pbr")]
pub use bevy_pbr as pbr;
#[cfg(feature = "property")]
pub use bevy_property as property;
#[cfg(feature = "render")]
pub use bevy_render as render;
#[cfg(feature = "scene")]
pub use bevy_scene as scene;
#[cfg(feature = "text")]
pub use bevy_text as text;
#[cfg(feature = "transform")]
pub use bevy_transform as transform;
#[cfg(feature = "ui")]
pub use bevy_ui as ui;
#[cfg(feature = "wgpu")]
pub use bevy_wgpu as wgpu;
#[cfg(feature = "window")]
pub use bevy_window as window;
#[cfg(feature = "winit")]
pub use bevy_winit as winit;
