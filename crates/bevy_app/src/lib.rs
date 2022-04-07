//! This crate is about everything concerning the highest-level, application layer of a Bevy app.

#![warn(missing_docs)]

mod app;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "bevy_ci_testing")]
mod ci_testing;

pub use app::*;
pub use bevy_derive::DynamicPlugin;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App, CoreStage, DynamicPlugin, Plugin, PluginGroup, StartupSchedule, StartupStage,
    };
}

use bevy_ecs::schedule::StageLabel;

/// The names of the default [`App`] stages.
///
/// The relative [`Stages`](bevy_ecs::schedule::Stage) are added by [`App::add_default_stages`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum CoreStage {
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs before all other app stages.
    First,
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs before [`CoreStage::Update`].
    PreUpdate,
    /// The [`Stage`](bevy_ecs::schedule::Stage) responsible for doing most app logic. Systems should be registered here by default.
    Update,
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs after [`CoreStage::Update`].
    PostUpdate,
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs after all other app stages.
    Last,
}

/// The label for the startup [`Schedule`](bevy_ecs::schedule::Schedule),
/// which runs once at the beginning of the [`App`].
///
/// When targeting a [`Stage`](bevy_ecs::schedule::Stage) inside this [`Schedule`](bevy_ecs::schedule::Schedule),
/// you need to use [`StartupStage`] instead.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub struct StartupSchedule;

/// The names of the default [`App`] startup stages.
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum StartupStage {
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs once before [`StartupStage::Startup`].
    PreStartup,
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs once when an [`App`] starts up.
    Startup,
    /// The [`Stage`](bevy_ecs::schedule::Stage) that runs once after [`StartupStage::Startup`].
    PostStartup,
}
