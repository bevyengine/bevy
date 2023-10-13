//! This crate is about everything concerning the highest-level, application layer of a Bevy app.

#![warn(missing_docs)]
#![allow(clippy::type_complexity)]

mod app;
mod main_schedule;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "bevy_ci_testing")]
pub mod ci_testing;

pub use app::*;
pub use bevy_derive::DynamicPlugin;
pub use main_schedule::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App,
        main_schedule::{
            First, FixedUpdate, Last, Main, PostStartup, PostUpdate, PreStartup, PreUpdate,
            SpawnScene, Startup, StateTransition, Update,
        },
        DynamicPlugin, Plugin, PluginGroup,
    };
}
