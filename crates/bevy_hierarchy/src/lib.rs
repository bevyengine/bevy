#![allow(clippy::type_complexity)]
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

mod valid_parent_check_plugin;
pub use valid_parent_check_plugin::*;

mod query_extension;
pub use query_extension::*;

#[doc(hidden)]
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        child_builder::*, components::*, hierarchy::*, query_extension::*, HierarchyPlugin,
        ValidParentCheckPlugin,
    };
}

use bevy_app::prelude::*;

/// The base plugin for handling [`Parent`] and [`Children`] components
#[derive(Default)]
pub struct HierarchyPlugin;

impl Plugin for HierarchyPlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<Children>()
            .register_type::<Parent>()
            .register_type::<smallvec::SmallVec<[bevy_ecs::entity::Entity; 8]>>()
            .add_event::<HierarchyEvent>();
    }
}
