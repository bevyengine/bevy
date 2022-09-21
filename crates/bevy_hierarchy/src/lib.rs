#![warn(missing_docs)]
//! `bevy_hierarchy` can be used to define hierarchies of entities.
//!
//! Most commonly, these hierarchies are used for inheriting `Transform` values
//! from the [`Parent`] to its [`Children`].

mod components;
pub use components::*;

mod hierarchy;
pub use hierarchy::*;

mod child_builder;
pub use child_builder::*;

#[cfg(feature = "events")]
mod events;
#[cfg(feature = "events")]
pub use events::*;

#[cfg(feature = "bevy_app")]
mod valid_parent_check_plugin;
#[cfg(feature = "bevy_app")]
pub use valid_parent_check_plugin::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{child_builder::*, components::*, hierarchy::*};
    #[doc(hidden)]
    #[cfg(feature = "app")]
    pub use crate::{HierarchyPlugin, ValidParentCheckPlugin};
}

/// The base plugin for handling [`Parent`] and [`Children`] components
#[derive(Default)]
#[cfg(feature = "bevy_app")]
pub struct HierarchyPlugin;

#[cfg(feature = "bevy_app")]
impl bevy_app::Plugin for HierarchyPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        #[cfg(feature = "bevy_reflect")]
        app.register_type::<Children>().register_type::<Parent>();
        app.add_event::<HierarchyEvent>();
    }
}
