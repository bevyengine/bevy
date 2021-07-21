mod app;
mod app_builder;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "bevy_ci_testing")]
mod ci_testing;

use std::{any::Any, hash::Hash};

pub use app::*;
pub use app_builder::*;
pub use bevy_derive::DynamicPlugin;
pub use bevy_ecs::event::*;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        app::App, app_builder::AppBuilder, CoreStage, DynamicPlugin, Plugin, PluginGroup,
        StartupStage,
    };
}

use bevy_ecs::schedule::{DynEq, DynHash, StageLabel};

/// The names of the default App stages
#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum CoreStage {
    /// Runs once at the beginning of the app.
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
#[derive(Debug, PartialEq, Clone)]
pub enum StartupStage {
    /// Name of app stage that runs once before the startup stage
    PreStartup,
    /// Name of app stage that runs once when an app starts up
    Startup,
    /// Name of app stage that runs once after the startup stage
    PostStartup,
}

impl StageLabel for StartupStage {
    fn dyn_clone(&self) -> Box<dyn StageLabel> {
        Box::new(Clone::clone(self))
    }
}

impl DynEq for StartupStage {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn dyn_eq(&self, other: &dyn DynEq) -> bool {
        match self {
            StartupStage::PreStartup => CoreStage::PreUpdate.dyn_eq(other),
            StartupStage::Startup => CoreStage::Update.dyn_eq(other),
            StartupStage::PostStartup => CoreStage::PostUpdate.dyn_eq(other),
        }
    }
}
impl DynHash for StartupStage {
    fn as_dyn_eq(&self) -> &dyn bevy_ecs::schedule::DynEq {
        match self {
            StartupStage::PreStartup => &CoreStage::PreUpdate,
            StartupStage::Startup => &CoreStage::Update,
            StartupStage::PostStartup => &CoreStage::PostUpdate,
        }
    }

    fn dyn_hash(&self, mut state: &mut dyn std::hash::Hasher) {
        match self {
            StartupStage::PreStartup => CoreStage::PreUpdate.hash(&mut state),
            StartupStage::Startup => CoreStage::Update.hash(&mut state),
            StartupStage::PostStartup => CoreStage::PostUpdate.hash(&mut state),
        }
        CoreStage::Update.type_id().hash(&mut state);
    }
}
