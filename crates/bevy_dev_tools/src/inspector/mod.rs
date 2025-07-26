//! Bevy Entity & Component Inspector
//!
//! A comprehensive, modular entity and component inspector for Bevy using only built-in UI components.
//! Features reflection-based component inspection, smart entity grouping, collapsible panels,
//! and configurable data sources.

pub mod components;
pub mod config;
pub mod data_sources;
pub mod plugin;
pub mod systems;
pub mod ui_widgets;

pub use components::*;
pub use config::*;
pub use data_sources::*;
pub use plugin::*;
pub use systems::*;
pub use ui_widgets::*;
