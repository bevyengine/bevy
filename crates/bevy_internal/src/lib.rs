#![allow(clippy::type_complexity)]
#![warn(missing_docs)]
//! This module is separated into its own crate to enable simple dynamic linking for Bevy, and should not be used directly

/// `use bevy::prelude::*;` to import common components, bundles, and plugins.
pub mod prelude;

mod default_plugins;
pub use default_plugins::*;

pub mod a11y {
    //! Integrate with platform accessibility APIs.
    pub use bevy_a11y::*;
}

pub mod app {
    //! Build bevy apps, create plugins, and read events.
    pub use bevy_app::*;
}

#[cfg(feature = "bevy_asset")]
pub mod asset {
    //! Load and store assets and resources for Apps.
    pub use bevy_asset::*;
}

pub mod core {
    //! Contains core plugins.
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

pub mod log {
    //! Logging capabilities
    pub use bevy_log::*;
}

pub mod math {
    //! Math types (Vec3, Mat4, Quat, etc) and helpers.
    pub use bevy_math::*;
}

pub mod ptr {
    //! Utilities for working with untyped pointers in a more safe way.
    pub use bevy_ptr::*;
}

pub mod reflect {
    //! Type reflection used for dynamically interacting with rust types.
    pub use bevy_reflect::*;
}

#[cfg(feature = "bevy_scene")]
pub mod scene {
    //! Save/load collections of entities and components to/from file.
    pub use bevy_scene::*;
}

pub mod tasks {
    //! Pools for async, IO, and compute tasks.
    pub use bevy_tasks::*;
}

pub mod time {
    //! Contains time utilities.
    pub use bevy_time::*;
}

pub mod hierarchy {
    //! Entity hierarchies and property inheritance
    pub use bevy_hierarchy::*;
}

pub mod transform {
    //! Local and global transforms (e.g. translation, scale, rotation).
    pub use bevy_transform::*;
}

pub mod utils {
    //! Various miscellaneous utilities for easing development
    pub use bevy_utils::*;
}

pub mod window {
    //! Configuration, creation, and management of one or more windows.
    pub use bevy_window::*;
}

#[cfg(feature = "bevy_animation")]
pub mod animation {
    //! Provides types and plugins for animations.
    pub use bevy_animation::*;
}

#[cfg(feature = "bevy_audio")]
pub mod audio {
    //! Provides types and plugins for audio playback.
    pub use bevy_audio::*;
}

#[cfg(feature = "bevy_core_pipeline")]
pub mod core_pipeline {
    //! Core render pipeline.
    pub use bevy_core_pipeline::*;
}

#[cfg(feature = "bevy_gilrs")]
pub mod gilrs {
    //! Bevy interface with `GilRs` - "Game Input Library for Rust" - to handle gamepad inputs.
    pub use bevy_gilrs::*;
}

#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    //! Support for GLTF file loading.
    pub use bevy_gltf::*;
}

#[cfg(feature = "bevy_pbr")]
pub mod pbr {
    //! Physically based rendering.
    pub use bevy_pbr::*;
}

#[cfg(feature = "bevy_render")]
pub mod render {
    //! Cameras, meshes, textures, shaders, and pipelines.
    //! Use [`RenderDevice::features`](crate::render::renderer::RenderDevice::features),
    //! [`RenderDevice::limits`](crate::render::renderer::RenderDevice::limits), and the
    //! [`RenderAdapterInfo`](crate::render::renderer::RenderAdapterInfo) resource to
    //! get runtime information about the actual adapter, backend, features, and limits.
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
    //! Window creation, configuration, and handling
    pub use bevy_winit::*;
}

#[cfg(feature = "bevy_gizmos")]
pub mod gizmos {
    //! Immediate mode drawing api for visual debugging.
    //!
    //! # Example
    //! ```
    //! # use bevy_gizmos::prelude::*;
    //! # use bevy_render::prelude::*;
    //! # use bevy_math::prelude::*;
    //! fn system(mut gizmos: Gizmos) {
    //!     gizmos.line(Vec3::ZERO, Vec3::X, Color::GREEN);
    //! }
    //! # bevy_ecs::system::assert_is_system(system);
    //! ```
    //!
    //! See the documentation on [`Gizmos`](gizmos::Gizmos) for more examples.
    pub use bevy_gizmos::*;
}

#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    //! Dynamic linking of plugins
    pub use bevy_dynamic_plugin::*;
}
