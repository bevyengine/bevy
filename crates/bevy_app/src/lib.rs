#![cfg_attr(
    any(docsrs, docsrs_dep),
    expect(
        internal_features,
        reason = "rustdoc_internals is needed for fake_variadic"
    )
)]
#![cfg_attr(any(docsrs, docsrs_dep), feature(doc_auto_cfg, rustdoc_internals))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]
#![no_std]

//! This crate is about everything concerning the highest-level, application layer of a Bevy app.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod app;
mod main_schedule;
mod panic_handler;
mod plugin;
mod plugin_group;
mod schedule_runner;
mod sub_app;
#[cfg(feature = "bevy_tasks")]
mod task_pool_plugin;
#[cfg(all(any(unix, windows), feature = "std"))]
mod terminal_ctrl_c_handler;

pub use app::*;
pub use main_schedule::*;
pub use panic_handler::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;
pub use sub_app::*;
#[cfg(feature = "bevy_tasks")]
pub use task_pool_plugin::*;
#[cfg(all(any(unix, windows), feature = "std"))]
pub use terminal_ctrl_c_handler::*;

/// The app prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::{App, AppExit},
        main_schedule::{
            First, FixedFirst, FixedLast, FixedPostUpdate, FixedPreUpdate, FixedUpdate, Last, Main,
            PostStartup, PostUpdate, PreStartup, PreUpdate, RunFixedMainLoop,
            RunFixedMainLoopSystem, SpawnScene, Startup, Update,
        },
        sub_app::SubApp,
        Plugin, PluginGroup,
    };

    #[cfg(feature = "bevy_tasks")]
    #[doc(hidden)]
    pub use crate::{NonSendMarker, TaskPoolOptions, TaskPoolPlugin};
}
