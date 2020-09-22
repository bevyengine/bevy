/// The names of the default App stages
pub mod stage;
/// The names of the default App startup stages
pub mod startup_stage;

mod app;
mod app_builder;
mod event;
mod plugin;
mod schedule_runner;

pub use app::*;
pub use app_builder::*;
pub use bevy_derive::DynamicPlugin;
pub use event::*;
pub use plugin::*;
pub use schedule_runner::*;

pub mod prelude {
    pub use crate::{
        app::App,
        app_builder::AppBuilder,
        event::{EventReader, Events},
        plugin::Plugin,
        stage, DynamicPlugin,
    };
}
