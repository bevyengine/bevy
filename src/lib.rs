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
//! If you prefer it, you can also consume the individual bevy crates directly.

#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

pub mod prelude;

mod default_plugins;
pub use default_plugins::*;

pub mod app {
    pub use bevy_app::*;
}
pub mod asset {
    pub use bevy_asset::*;
}
pub mod core {
    pub use bevy_core::*;
}
pub mod diagnostic {
    pub use bevy_diagnostic::*;
}
pub mod ecs {
    pub use bevy_ecs::*;
}
pub mod input {
    pub use bevy_input::*;
}
pub mod math {
    pub use bevy_math::*;
}
pub mod property {
    pub use bevy_property::*;
}
pub mod scene {
    pub use bevy_scene::*;
}
pub mod tasks {
    pub use bevy_tasks::*;
}
pub mod transform {
    pub use bevy_transform::*;
}
pub mod type_registry {
    pub use bevy_type_registry::*;
}
pub mod utils {
    pub use bevy_utils::*;
}
pub mod window {
    pub use bevy_window::*;
}

#[cfg(feature = "bevy_audio")]
pub mod audio {
    pub use bevy_audio::*;
}

#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    pub use bevy_gltf::*;
}

#[cfg(feature = "bevy_pbr")]
pub mod pbr {
    pub use bevy_pbr::*;
}

#[cfg(feature = "bevy_render")]
pub mod render {
    pub use bevy_render::*;
}

#[cfg(feature = "bevy_sprite")]
pub mod sprite {
    pub use bevy_sprite::*;
}

#[cfg(feature = "bevy_text")]
pub mod text {
    pub use bevy_text::*;
}

#[cfg(feature = "bevy_ui")]
pub mod ui {
    pub use bevy_ui::*;
}

#[cfg(feature = "bevy_winit")]
pub mod winit {
    pub use bevy_winit::*;
}

#[cfg(feature = "bevy_wgpu")]
pub mod wgpu {
    pub use bevy_wgpu::*;
}

#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    pub use bevy_dynamic_plugin::*;
}
