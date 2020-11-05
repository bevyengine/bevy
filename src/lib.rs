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
//! The `bevy` crate is just a container crate that makes it easier to consume Bevy components.
//! The defaults provide a "full" engine experience, but you can easily enable / disable features
//! in your project's `Cargo.toml` to meet your specific needs. See Bevy's `Cargo.toml` for a full list of features available.
//!
//! If you prefer, you can also consume the individual bevy crates directly.
//! Each module in the root of this crate, except for the prelude, can be found on crates.io
//! with `bevy_` appended to the front, e.g. `app` -> [`bevy_app`](https://docs.rs/bevy_app/*/bevy_app/).

#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

/// `use bevy::prelude::*;` to import common components, bundles, and plugins.
pub mod prelude;

mod default_plugins;
pub use default_plugins::*;

pub mod app {
    //! Build bevy apps, create plugins, and read events.
    pub use bevy_app::*;
}

pub mod asset {
    //! Load and store assets and resources for Apps.
    pub use bevy_asset::*;
}

pub mod core {
    //! Contains core plugins and utilities for time.
    pub use bevy_core::*;
}

pub mod diagnostic {
    //! Useful diagnostic plugins and types for bevy apps.
    pub use bevy_diagnostic::*;
}

pub mod ecs {
    //! Bevy's entity-component-system.
    pub use bevy_ecs::*;
}

pub mod input {
    //! Resources and events for inputs, e.g. mouse/keyboard, touch, gamepads, etc.
    pub use bevy_input::*;
}

pub mod math {
    //! Math types (Vec3, Mat4, Quat, etc) and helpers.
    pub use bevy_math::*;
}

pub mod property {
    //! Dynamically interact with struct fields and names.
    pub use bevy_property::*;
}

pub mod scene {
    //! Save/load collections of entities and components to/from file.
    pub use bevy_scene::*;
}

pub mod tasks {
    //! Pools for async, IO, and compute tasks.
    pub use bevy_tasks::*;
}

pub mod transform {
    //! Local and global transforms (e.g. translation, scale, rotation).
    pub use bevy_transform::*;
}

pub mod type_registry {
    //! Registered types and components can be used when loading scenes.
    pub use bevy_type_registry::*;
}

pub mod utils {
    pub use bevy_utils::*;
}

pub mod window {
    //! Configuration, creation, and management of one or more windows.
    pub use bevy_window::*;
}

#[cfg(feature = "bevy_audio")]
pub mod audio {
    //! Provides types and plugins for audio playback.
    pub use bevy_audio::*;
}

#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    //! Support for GLTF file loading.
    pub use bevy_gltf::*;
}

#[cfg(feature = "bevy_pbr")]
pub mod pbr {
    //! Physically based rendering.
    //! **Note**: true PBR has not yet been implemented; the name `pbr` is aspirational.
    pub use bevy_pbr::*;
}

#[cfg(feature = "bevy_render")]
pub mod render {
    //! Cameras, meshes, textures, shaders, and pipelines.
    pub use bevy_render::*;
}

#[cfg(feature = "bevy_sprite")]
pub mod sprite {
    //! Items for sprites, rects, texture atlases, etc.
    pub use bevy_sprite::*;
}

#[cfg(feature = "bevy_text")]
pub mod text {
    //! Text drawing, styling, and font assets.
    pub use bevy_text::*;
}

#[cfg(feature = "bevy_ui")]
pub mod ui {
    //! User interface components and widgets.
    pub use bevy_ui::*;
}

#[cfg(feature = "bevy_winit")]
pub mod winit {
    pub use bevy_winit::*;
}

#[cfg(feature = "bevy_wgpu")]
pub mod wgpu {
    //! A render backend utilizing [wgpu](https://github.com/gfx-rs/wgpu-rs).
    pub use bevy_wgpu::*;
}

#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    pub use bevy_dynamic_plugin::*;
}
