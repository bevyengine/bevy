mod default_plugins;
#[doc(hidden)]
pub use default_plugins::*;

#[doc(hidden)]
pub use bevy_app;
#[doc(hidden)]
pub use bevy_asset;
#[cfg(feature = "bevy_audio")]
#[doc(hidden)]
pub use bevy_audio;
#[doc(hidden)]
pub use bevy_core;
#[doc(hidden)]
pub use bevy_derive;
#[doc(hidden)]
pub use bevy_diagnostic;
#[cfg(feature = "bevy_dynamic_plugin")]
#[doc(hidden)]
pub use bevy_dynamic_plugin;
#[doc(hidden)]
pub use bevy_ecs;
#[cfg(feature = "bevy_gilrs")]
#[doc(hidden)]
pub use bevy_gilrs;
#[cfg(feature = "bevy_gltf")]
#[doc(hidden)]
pub use bevy_gltf;
#[doc(hidden)]
pub use bevy_input;
#[doc(hidden)]
pub use bevy_log;
#[doc(hidden)]
pub use bevy_math;
#[cfg(feature = "bevy_pbr")]
#[doc(hidden)]
pub use bevy_pbr;
#[doc(hidden)]
pub use bevy_reflect;
#[cfg(feature = "bevy_render")]
#[doc(hidden)]
pub use bevy_render;
#[doc(hidden)]
pub use bevy_scene;
#[cfg(feature = "bevy_sprite")]
#[doc(hidden)]
pub use bevy_sprite;
#[doc(hidden)]
pub use bevy_tasks;
#[cfg(feature = "bevy_text")]
#[doc(hidden)]
pub use bevy_text;
#[doc(hidden)]
pub use bevy_transform;
#[cfg(feature = "bevy_ui")]
#[doc(hidden)]
pub use bevy_ui;
#[doc(hidden)]
pub use bevy_utils;
#[cfg(feature = "bevy_wgpu")]
#[doc(hidden)]
pub use bevy_wgpu;
#[doc(hidden)]
pub use bevy_window;
#[cfg(feature = "bevy_winit")]
#[doc(hidden)]
pub use bevy_winit;

#[cfg(target_os = "android")]
#[doc(hidden)]
pub use ndk_glue;
