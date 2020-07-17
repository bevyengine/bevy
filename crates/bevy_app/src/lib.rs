pub mod stage;

mod app;
mod app_builder;
mod event;
mod plugin;
mod schedule_runner;
mod startup_stage;

pub use app::*;
pub use app_builder::*;
pub use bevy_derive::DynamicAppPlugin;
pub use event::*;
pub use plugin::*;
pub use schedule_runner::*;
pub use startup_stage::*;

pub mod prelude {
    pub use crate::{
        app::App,
        app_builder::AppBuilder,
        event::{EventReader, Events},
        plugin::AppPlugin,
        stage, DynamicAppPlugin,
    };
}
