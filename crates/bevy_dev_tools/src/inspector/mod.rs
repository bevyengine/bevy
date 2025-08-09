//! Bevy Remote Inspector
//!
//! Out-of-process entity inspector for Bevy applications using bevy_remote.

pub mod http_client;
pub mod inspector;
pub mod ui;

pub use inspector::InspectorPlugin;
