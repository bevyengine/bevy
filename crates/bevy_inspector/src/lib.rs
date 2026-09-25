//! An entity inspector built on `bevy_ui` and `bevy_feathers`.
//!
//! The inspector presents the data gathered by [`bevy_dev_tools::inspection`] as interactive
//! widgets. This crate provides an entity tree panel, see [`entity_tree`], and a details panel
//! for the selected entity, see [`details_panel`].
//!
//! Apps are expected to add `bevy_feathers::FeathersPlugins` themselves, alongside
//! [`InspectorPlugin`].

extern crate alloc;

pub mod details_panel;
pub mod entity_tree;

use alloc::string::{String, ToString};

use bevy_app::{App, Plugin, PostUpdate};
use bevy_dev_tools::inspection::label_resolution::LabelResolutionPlugin;
use bevy_ecs::{
    component::{ComponentId, ComponentInfo},
    entity::Entity,
    reflect::{AppTypeRegistry, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    world::World,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::UiSystems;

use crate::details_panel::{sync_details_panel, DetailsCollapsed, DetailsIndex, DetailsPanelSync};
use crate::entity_tree::{sync_entity_tree, EntityTreeSync, TreeRowIndex};

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

/// The short name of a component type, taken from the type registry where possible.
///
/// `ComponentInfo::name` is only populated when the `debug` feature of `bevy_utils` is enabled,
/// so the registered type path is preferred.
pub(crate) fn component_short_name(world: &World, component_id: ComponentId) -> String {
    let Some(info) = world.components().get_info(component_id) else {
        return component_id.index().to_string();
    };
    info.type_id()
        .and_then(|type_id| {
            let registry = world.get_resource::<AppTypeRegistry>()?.read();
            let registration = registry.get(type_id)?;
            Some(
                registration
                    .type_info()
                    .type_path_table()
                    .short_path()
                    .to_string(),
            )
        })
        .unwrap_or_else(|| ComponentInfo::name(info).shortname().to_string())
}

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
            .init_resource::<DetailsIndex>()
            .init_resource::<DetailsCollapsed>()
            .init_resource::<DetailsPanelSync>()
            .add_systems(
                PostUpdate,
                (sync_entity_tree, sync_details_panel).before(UiSystems::Prepare),
            );
    }
}
