//! An entity inspector built on `bevy_ui` and `bevy_feathers`.
//!
//! The inspector presents the data gathered by [`bevy_dev_tools::inspection`] as interactive
//! widgets. This crate currently provides the entity tree panel; see [`entity_tree`].
//!
//! Apps are expected to add `bevy_feathers::FeathersPlugins` themselves, alongside
//! [`InspectorPlugin`].

extern crate alloc;

pub mod entity_tree;

use bevy_app::{App, Plugin, Update};
use bevy_dev_tools::inspection::label_resolution::LabelResolutionPlugin;
use bevy_ecs::{entity::Entity, reflect::ReflectResource, resource::Resource};
use bevy_reflect::{prelude::ReflectDefault, Reflect};

use crate::entity_tree::{
    sync_entity_tree, EntityTreeSync, InspectorRow, InspectorRowLabel, InspectorTreeView,
    InspectorUi, TreeRowIndex,
};

/// Where the inspector reads its data from.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub enum InspectorSource {
    /// The world the inspector itself runs in.
    #[default]
    Local,
}

/// The entity currently being inspected, as an id in the inspected world.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct InspectorSelection(pub Option<Entity>);

/// Plugin which registers the inspector resources and the entity tree synchronization systems.
///
/// This plugin does not add the widget plugins the inspector renders with; the app is expected to
/// add `bevy_feathers::FeathersPlugins` as well.
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<LabelResolutionPlugin>() {
            app.add_plugins(LabelResolutionPlugin);
        }

        app.init_resource::<InspectorSource>()
            .init_resource::<InspectorSelection>()
            .init_resource::<TreeRowIndex>()
            .init_resource::<EntityTreeSync>()
            .register_type::<InspectorSource>()
            .register_type::<InspectorSelection>()
            .register_type::<InspectorUi>()
            .register_type::<InspectorTreeView>()
            .register_type::<InspectorRow>()
            .register_type::<InspectorRowLabel>()
            .add_systems(Update, sync_entity_tree);
    }
}
