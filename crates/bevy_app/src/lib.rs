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
    #[cfg(feature = "bevy_reflect")]
    #[doc(hidden)]
    pub use crate::AppTypeRegistry;
    #[doc(hidden)]
    pub use crate::{
        app::App, CoreSet, DynamicPlugin, Plugin, PluginGroup, StartupSchedule, StartupSet,
    };
}

use bevy_ecs::schedule::StageLabel;

/// The names of the default [`App`] schedules.
///
/// The corresponding [`Schedule`](bevy_ecs::schedule::Schedule) objects are added by [`App::add_default_schedules`].
pub enum CoreSchedule {
    /// The schedule that runs once when the app starts.
    Startup,
    /// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
    Main,
}

/// The names of the default [`App`] system sets.
///
/// The relative [`SystemSet`](bevy_ecs::schedule::SystemSet) are added by [`App::add_default_sets`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum CoreSet {
    /// Runs before all other app stages.
    First,
    /// Runs before [`CoreStage::Update`].
    PreUpdate,
    /// Responsible for doing most app logic. Systems should be registered here by default.
    Update,
    /// Runs after [`CoreStage::Update`].
    PostUpdate,
    /// Runs after all other app stages.
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
pub enum StartupSet {
    /// Runs once before [`StartupSet::Startup`].
    PreStartup,
    /// Runs once when an [`App`] starts up.
    Startup,
    /// Runs once after [`StartupSet::Startup`].
    PostStartup,
}
