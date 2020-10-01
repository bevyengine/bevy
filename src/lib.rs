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
//! ```
//!use bevy::prelude::*;
//!
//!fn main() {
//!    App::build()
//!        .add_system(hello_world_system.system())
//!        .run();
//!}
//!
//!fn hello_world_system() {
//!    println!("hello world");
//!}
//! ```

//! Don't let the simplicity of the example above fool you. Bevy is a [fully featured game engine](https://bevyengine.org)
//! and it gets more powerful every day!
//!
//! ### This Crate
//! The "bevy" crate is just a container crate that makes it easier to consume Bevy components.
//! The defaults provide a "full" engine experience, but you can easily enable / disable features
//! in your project's Cargo.toml to meet your specific needs. See Bevy's Cargo.toml for a full list of features available.
//!
//! If you prefer it, you can also consume the individual bevy crates directly.

#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

mod add_default_plugins;
pub mod prelude;

pub use add_default_plugins::*;
pub use bevy_app as app;
pub use bevy_asset as asset;
pub use bevy_core as core;
pub use bevy_diagnostic as diagnostic;
pub use bevy_ecs as ecs;
pub use bevy_input as input;
pub use bevy_math as math;
pub use bevy_property as property;
pub use bevy_scene as scene;
pub use bevy_tasks as tasks;
pub use bevy_transform as transform;
pub use bevy_type_registry as type_registry;
pub use bevy_window as window;

#[cfg(feature = "bevy_audio")]
pub use bevy_audio as audio;

#[cfg(feature = "bevy_gltf")]
pub use bevy_gltf as gltf;

#[cfg(feature = "bevy_pbr")]
pub use bevy_pbr as pbr;

#[cfg(feature = "bevy_render")]
pub use bevy_render as render;

#[cfg(feature = "bevy_sprite")]
pub use bevy_sprite as sprite;

#[cfg(feature = "bevy_text")]
pub use bevy_text as text;

#[cfg(feature = "bevy_ui")]
pub use bevy_ui as ui;

#[cfg(feature = "bevy_winit")]
pub use bevy_winit as winit;

#[cfg(feature = "bevy_wgpu")]
pub use bevy_wgpu as wgpu;

#[cfg(feature = "bevy_dynamic_plugin")]
pub use bevy_dynamic_plugin as dynamic_plugin;
