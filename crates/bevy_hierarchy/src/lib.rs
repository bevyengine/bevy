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

mod events;
pub use events::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{child_builder::*, components::*, hierarchy::*, HierarchyPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::chedule::IntoSystemLabel;

/// The base plugin for handling [`Parent`] and [`Children`] components
#[derive(Default)]
pub struct HierarchyPlugin;

/// Label enum for the systems relating to hierarchy upkeep
#[derive(Debug, Hash, PartialEq, Eq, Clone, IntoSystemLabel)]
pub enum HierarchySystem {
    /// Updates [`Parent`] when changes in the hierarchy occur
    ParentUpdate,
}

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .add_event::<HierarchyEvent>();
    }
}
