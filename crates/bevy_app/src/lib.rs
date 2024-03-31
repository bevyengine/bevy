#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! This crate is about everything concerning the highest-level, application layer of a Bevy app.

mod app;
mod main_schedule;
mod panic_handler;
mod plugin;
mod plugin_group;
mod schedule_runner;

pub use app::*;
pub use bevy_derive::DynamicPlugin;
pub use main_schedule::*;
pub use panic_handler::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App,
        main_schedule::{
            First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Last, Main,
            PostStartup, PostUpdate, PreStartup, PreUpdate, SpawnScene, Startup, StateTransition,
            Update,
        },
        DynamicPlugin, Plugin, PluginGroup,
    };
}
