#![warn(clippy::undocumented_unsafe_blocks)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// Components for integrating `RngCore` PRNGs into bevy.
pub mod component;
/// Plugin for integrating `RngCore` PRNGs into bevy.
pub mod plugin;
/// Prelude for providing all necessary types for easy use.
pub mod prelude;
/// Resource for integrating `RngCore` PRNGs into bevy.
pub mod resource;
mod thread_local_entropy;
mod traits;
