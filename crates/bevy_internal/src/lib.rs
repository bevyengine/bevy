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

pub use bevy_a11y as a11y;
#[cfg(feature = "bevy_animation")]
pub use bevy_animation as animation;
pub use bevy_app as app;
#[cfg(feature = "bevy_asset")]
pub use bevy_asset as asset;
#[cfg(feature = "bevy_audio")]
pub use bevy_audio as audio;
#[cfg(feature = "bevy_color")]
pub use bevy_color as color;
pub use bevy_core as core;
#[cfg(feature = "bevy_core_pipeline")]
pub use bevy_core_pipeline as core_pipeline;
#[cfg(feature = "bevy_dev_tools")]
pub use bevy_dev_tools as dev_tools;
pub use bevy_diagnostic as diagnostic;
#[cfg(feature = "bevy_dynamic_plugin")]
pub use bevy_dynamic_plugin as dynamic_plugin;
pub use bevy_ecs as ecs;
#[cfg(feature = "bevy_gilrs")]
pub use bevy_gilrs as gilrs;
#[cfg(feature = "bevy_gizmos")]
pub use bevy_gizmos as gizmos;
#[cfg(feature = "bevy_gltf")]
pub use bevy_gltf as gltf;
pub use bevy_hierarchy as hierarchy;
pub use bevy_input as input;
pub use bevy_log as log;
pub use bevy_math as math;
#[cfg(feature = "bevy_pbr")]
pub use bevy_pbr as pbr;
#[cfg(feature = "bevy_picking")]
pub use bevy_picking as picking;
pub use bevy_ptr as ptr;
pub use bevy_reflect as reflect;
#[cfg(feature = "bevy_render")]
pub use bevy_render as render;
#[cfg(feature = "bevy_scene")]
pub use bevy_scene as scene;
#[cfg(feature = "bevy_sprite")]
pub use bevy_sprite as sprite;
#[cfg(feature = "bevy_state")]
pub use bevy_state as state;
pub use bevy_tasks as tasks;
#[cfg(feature = "bevy_text")]
pub use bevy_text as text;
pub use bevy_time as time;
pub use bevy_transform as transform;
#[cfg(feature = "bevy_ui")]
pub use bevy_ui as ui;
pub use bevy_utils as utils;
pub use bevy_window as window;
#[cfg(feature = "bevy_winit")]
pub use bevy_winit as winit;
