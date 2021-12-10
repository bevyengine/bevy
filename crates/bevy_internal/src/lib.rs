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

pub mod log {
    //! Logging capabilities
    pub use bevy_log::*;
}

pub mod math {
    //! Math types (Vec3, Mat4, Quat, etc) and helpers.
    pub use bevy_math::*;
}

pub mod reflect {
    // TODO: remove these renames once TypeRegistryArc is no longer required
    //! Type reflection used for dynamically interacting with rust types.
    pub use bevy_reflect::{
        TypeRegistry as TypeRegistryInternal, TypeRegistryArc as TypeRegistry, *,
    };
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

#[cfg(feature = "bevy_core_pipeline")]
pub mod core_pipeline {
    //! Core render pipeline.
    pub use bevy_core_pipeline::*;
}

#[cfg(feature = "bevy_gilrs")]
pub mod gilrs {
    pub use bevy_gilrs::*;
}

#[cfg(feature = "bevy_gltf")]
pub mod gltf {
    //! Support for GLTF file loading.
    pub use bevy_gltf::*;
}

#[cfg(feature = "bevy_gltf2")]
pub mod gltf2 {
    //! Support for GLTF file loading.
    pub use bevy_gltf2::*;
}

#[cfg(feature = "bevy_pbr")]
pub mod pbr {
    //! Physically based rendering.
    pub use bevy_pbr::*;
}

#[cfg(feature = "bevy_pbr2")]
pub mod pbr2 {
    //! Physically based rendering.
    pub use bevy_pbr2::*;
}

#[cfg(feature = "bevy_render")]
pub mod render {
    //! Cameras, meshes, textures, shaders, and pipelines.
    pub use bevy_render::*;
}

#[cfg(feature = "bevy_render2")]
pub mod render2 {
    //! Cameras, meshes, textures, shaders, and pipelines.
    pub use bevy_render2::*;
}

#[cfg(feature = "bevy_sprite")]
pub mod sprite {
    //! Items for sprites, rects, texture atlases, etc.
    pub use bevy_sprite::*;
}

#[cfg(feature = "bevy_sprite2")]
pub mod sprite2 {
    //! Items for sprites, rects, texture atlases, etc.
    pub use bevy_sprite2::*;
}

#[cfg(feature = "bevy_text")]
pub mod text {
    //! Text drawing, styling, and font assets.
    pub use bevy_text::*;
}

#[cfg(feature = "bevy_text2")]
pub mod text2 {
    //! Text drawing, styling, and font assets.
    pub use bevy_text2::*;
}

#[cfg(feature = "bevy_ui")]
pub mod ui {
    //! User interface components and widgets.
    pub use bevy_ui::*;
}

#[cfg(feature = "bevy_ui2")]
pub mod ui2 {
    //! User interface components and widgets.
    pub use bevy_ui2::*;
}

#[cfg(feature = "bevy_winit")]
pub mod winit {
    pub use bevy_winit::*;
}

#[cfg(feature = "bevy_wgpu")]
pub mod wgpu {
    //! A render backend utilizing [wgpu](https://wgpu.rs/).
    pub use bevy_wgpu::*;
}

#[cfg(feature = "bevy_dynamic_plugin")]
pub mod dynamic_plugin {
    pub use bevy_dynamic_plugin::*;
}

#[cfg(target_os = "android")]
pub use ndk_glue;
