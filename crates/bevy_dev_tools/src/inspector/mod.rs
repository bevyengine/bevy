//! Bevy Remote Inspector
//! 
//! Out-of-process entity inspector for Bevy applications using bevy_remote.

pub mod http_client;
pub mod ui;  
pub mod inspector;

pub use inspector::InspectorPlugin;