#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This module is separated into its own crate to enable simple dynamic linking for Bevy, and should not be used directly

/// `use bevy::prelude::*;` to import common components, bundles, and plugins.
pub mod prelude;

mod default_plugins;
pub use default_plugins::*;

/// Integrate with platform accessibility APIs.
pub mod a11y {
    pub use bevy_a11y::*;
}

/// Build bevy apps, create plugins, and read events.
pub mod app {
    pub use bevy_app::*;
}

/// Load and store assets and resources for Apps.
#[cfg(feature = "bevy_asset")]
pub mod asset {
    pub use bevy_asset::*;
}

/// Contains core plugins.
pub mod core {
    pub use bevy_core::*;
}

/// Shared color types and operations.
#[cfg(feature = "bevy_color")]
pub mod color {
    pub use bevy_color::*;
}

/// Useful diagnostic plugins and types for bevy apps.
pub mod diagnostic {
    pub use bevy_diagnostic::*;
}

/// Bevy's entity-component-system.
pub mod ecs {
    pub use bevy_ecs::*;
}

/// Resources and events for inputs, e.g. mouse/keyboard, touch, gamepads, etc.
pub mod input {
    pub use bevy_input::*;
}

/// Logging capabilities
pub mod log {
    pub use bevy_log::*;
}

/// Math types (Vec3, Mat4, Quat, etc) and helpers.
pub mod math {
    pub use bevy_math::*;
}

/// Utilities for working with untyped pointers in a more safe way.
pub mod ptr {
    pub use bevy_ptr::*;
}

/// Type reflection used for dynamically interacting with rust types.
pub mod reflect {
    pub use bevy_reflect::*;
}

/// Save/load collections of entities and components to/from file.
#[cfg(feature = "bevy_scene")]
pub mod scene {
    pub use bevy_scene::*;
}

/// Pools for async, IO, and compute tasks.
pub mod tasks {
    pub use bevy_tasks::*;
}

/// Contains time utilities.
pub mod time {
    pub use bevy_time::*;
}

/// Entity hierarchies and property inheritance
pub mod hierarchy {
    pub use bevy_hierarchy::*;
}

/// Local and global transforms (e.g. translation, scale, rotation).
pub mod transform {
    pub use bevy_transform::*;
}

/// Various miscellaneous utilities for easing development
pub mod utils {
    pub use bevy_utils::*;
}

/// Configuration, creation, and management of one or more windows.
pub mod window {
    pub use bevy_window::*;
}

/// Provides types and plugins for animations.
#[cfg(feature = "bevy_animation")]
pub mod animation {
    pub use bevy_animation::*;
}

/// Provides types and plugins for audio playback.
#[cfg(feature = "bevy_audio")]
pub mod audio {
    pub use bevy_audio::*;
}

/// Core render pipeline.
#[cfg(feature = "bevy_core_pipeline")]
pub mod core_pipeline {
    pub use bevy_core_pipeline::*;
}

/// Bevy interface with `GilRs` - "Game Input Library for Rust" - to handle gamepad inputs.
#[cfg(feature = "bevy_gilrs")]
pub mod gilrs {
    pub use bevy_gilrs::*;
}

/// Support for GLTF file loading.
#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    pub use bevy_gltf::*;
}

/// Physically based rendering.
#[cfg(feature = "bevy_pbr")]
pub mod pbr {
    pub use bevy_pbr::*;
}

/// Cameras, meshes, textures, shaders, and pipelines.
/// Use [`RenderDevice::features`](renderer::RenderDevice::features),
/// [`RenderDevice::limits`](renderer::RenderDevice::limits), and the
/// [`RenderAdapterInfo`](renderer::RenderAdapterInfo) resource to
/// get runtime information about the actual adapter, backend, features, and limits.
#[cfg(feature = "bevy_render")]
pub mod render {
    pub use bevy_render::*;
}

/// Items for sprites, rects, texture atlases, etc.
#[cfg(feature = "bevy_sprite")]
pub mod sprite {
    pub use bevy_sprite::*;
}

/// Text drawing, styling, and font assets.
#[cfg(feature = "bevy_text")]
pub mod text {
    pub use bevy_text::*;
}

/// User interface components and widgets.
#[cfg(feature = "bevy_ui")]
pub mod ui {
    pub use bevy_ui::*;
}

/// Window creation, configuration, and handling
#[cfg(feature = "bevy_winit")]
pub mod winit {
    pub use bevy_winit::*;
}

/// Immediate mode drawing api for visual debugging.
///
/// # Example
/// ```
/// # use bevy_gizmos::prelude::*;
/// # use bevy_render::prelude::*;
/// # use bevy_math::prelude::*;
/// # use bevy_color::palettes::basic::GREEN;
/// fn system(mut gizmos: Gizmos) {
///     gizmos.line(Vec3::ZERO, Vec3::X, GREEN);
/// }
/// # bevy_ecs::system::assert_is_system(system);
/// ```
///
/// See the documentation on [`Gizmos`](gizmos::Gizmos) for more examples.
#[cfg(feature = "bevy_gizmos")]
pub mod gizmos {
    pub use bevy_gizmos::*;
}

/// Dynamic linking of plugins
#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    pub use bevy_dynamic_plugin::*;
}

/// Collection of developer tools
#[cfg(feature = "bevy_dev_tools")]
pub mod dev_tools {
    pub use bevy_dev_tools::*;
}
