//! Bevy Remote Inspector
//!
//! Out-of-process entity inspector for Bevy applications using `bevy_remote`.

pub mod http_client;
mod plugin;
pub mod ui;

pub use plugin::InspectorPlugin;
