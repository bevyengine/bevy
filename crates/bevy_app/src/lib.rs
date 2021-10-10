//! This crate is about everything concerning the highest-level, application layer of a Bevy
//! app.

mod app;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "bevy_ci_testing")]
mod ci_testing;

pub use app::*;
pub use bevy_derive::DynamicPlugin;
pub use bevy_ecs::event::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{app::App, CoreStage, DynamicPlugin, Plugin, PluginGroup, StartupStage};
}

use bevy_ecs::schedule::StageLabel;

/// The names of the default App stages
///
/// The relative stages are added by [`App::add_default_stages`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum CoreStage {
    /// Runs only once at the beginning of the app.
    ///
    /// Consists of the sub-stages defined in [`StartupStage`]. Systems added here are
    /// referred to as "startup systems".
    Startup,
    /// Name of app stage that runs before all other app stages
    First,
    /// Name of app stage responsible for performing setup before an update. Runs before UPDATE.
    PreUpdate,
    /// Name of app stage responsible for doing most app logic. Systems should be registered here
    /// by default.
    Update,
    /// Name of app stage responsible for processing the results of UPDATE. Runs after UPDATE.
    PostUpdate,
    /// Name of app stage that runs after all other app stages
    Last,
}
/// The names of the default App startup stages
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum StartupStage {
    /// Name of app stage that runs once before the startup stage
    PreStartup,
    /// Name of app stage that runs once when an app starts up
    Startup,
    /// Name of app stage that runs once after the startup stage
    PostStartup,
}
