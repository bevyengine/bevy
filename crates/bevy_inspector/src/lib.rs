//! An entity inspector built on `bevy_ui` and `bevy_feathers`.
//!
//! The inspector presents the data gathered by [`bevy_dev_tools::inspection`] as interactive
//! widgets. This crate provides an entity tree panel, see [`entity_tree`], and a details panel
//! for the selected entity, see [`details_panel`].
//!
//! With the `remote` feature the same panels can inspect a separate running app over the Bevy
//! Remote Protocol, see the `remote` module.
//!
//! Apps are expected to add `bevy_feathers::FeathersPlugins` themselves, alongside
//! [`InspectorPlugin`].

extern crate alloc;

pub mod column_split;
#[cfg(feature = "remote")]
pub mod connection_panel;
pub mod details_panel;
pub mod entity_tree;
#[cfg(feature = "remote")]
pub mod remote;

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
use bevy_platform::collections::HashSet;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::UiSystems;

use crate::column_split::ColumnSplitPlugin;
use crate::details_panel::{
    apply_field_edit, store_column_splits, sync_details_panel, DetailsCollapsed,
    DetailsColumnSplits, DetailsIndex, DetailsPanelSync,
};
use crate::entity_tree::{sync_entity_tree, EntityTreeSync, TreeRowIndex};

/// Where the inspector reads its data from.
#[derive(Resource, Debug, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct InspectorSources {
    /// The sources the inspector reads data from
    pub sources: HashSet<InspectorSource>,
}

impl Default for InspectorSources {
    fn default() -> Self {
        InspectorSources::local()
    }
}

impl InspectorSources {
    /// Local inspection
    pub fn local() -> Self {
        let mut t = HashSet::new();
        t.insert(InspectorSource::Local);
        InspectorSources { sources: t }
    }

    /// Remote inspection (main and render)
    #[cfg(feature = "remote")]
    pub fn remote(host: impl Into<String>, main_port: u16, render_port: u16) -> Self {
        let mut t = HashSet::new();
        let host = host.into();
        t.insert(InspectorSource::Remote(remote::RemoteSource::new(
            host.clone(),
            main_port,
            WorldKind::Main,
        )));
        t.insert(InspectorSource::Remote(remote::RemoteSource::new(
            host,
            render_port,
            WorldKind::Render,
        )));
        InspectorSources { sources: t }
    }

    /// Remote inspection (main only)
    #[cfg(feature = "remote")]
    pub fn remote_main(host: impl Into<String>, main_port: u16) -> Self {
        let mut t = HashSet::new();
        t.insert(InspectorSource::Remote(remote::RemoteSource::new(
            host,
            main_port,
            WorldKind::Main,
        )));
        InspectorSources { sources: t }
    }
}

/// The sub app of a remote app
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect, Hash)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum WorldKind {
    /// Main subapp
    #[default]
    Main,
    /// Render subapp
    Render,
}

/// A single source to read data from
#[derive(Debug, Default, Clone, PartialEq, Eq, Reflect, Hash)]
#[reflect(Debug, Default, Clone, PartialEq)]
pub enum InspectorSource {
    /// The world the inspector itself runs in.
    #[default]
    Local,
    /// A separate running app, reached over the Bevy Remote Protocol.
    #[cfg(feature = "remote")]
    Remote(remote::RemoteSource),
}

/// The world the panels read: the `RemoteWorld` while the inspector reads from a remote
/// app, and `world` otherwise.
///
/// NOTE: we currently restrict to at most 1x main app and 1x render app.
pub(crate) fn world_to_inspect(world: &World, inspector_source: InspectorSource) -> &World {
    #[cfg(feature = "remote")]
    if let InspectorSource::Remote(rem) = inspector_source
        && let Some(remote) = world.get_resource::<remote::RemoteWorlds>()
    {
        return remote.get(rem.subapp).world();
    }
    let _ = inspector_source;
    world
}

/// An inspected entity, and whether its from the main or render world.
#[derive(Debug, Clone, PartialEq, Eq, Reflect, Hash)]
#[reflect(Debug, Clone, PartialEq)]
pub struct InspectableEntity {
    /// The entity in the remote world
    pub entity: Entity,
    /// Whether its from the main or render world
    pub source: InspectorSource,
}

impl InspectableEntity {
    /// Creates an selection
    pub fn new(entity: Entity, source: InspectorSource) -> Self {
        InspectableEntity { entity, source }
    }
}

/// The entity currently being inspected, as an id in the inspected world.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct InspectorSelection(pub Option<InspectableEntity>);

/// The [`ShortName`] of a component type, taken from the type registry where possible.
///
/// [`ComponentInfo::name`] is only populated when the `debug` feature of `bevy_utils` is enabled,
/// so the registered type path is preferred. Different components can share a short name, so it
/// is only used for display.
///
/// [`ShortName`]: bevy_utils::prelude::ShortName
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
        if !app.is_plugin_added::<ColumnSplitPlugin>() {
            app.add_plugins(ColumnSplitPlugin);
        }

        app.init_resource::<InspectorSources>()
            .init_resource::<InspectorSelection>()
            .init_resource::<TreeRowIndex>()
            .init_resource::<EntityTreeSync>()
            .init_resource::<DetailsIndex>()
            .init_resource::<DetailsCollapsed>()
            .init_resource::<DetailsColumnSplits>()
            .init_resource::<DetailsPanelSync>()
            .add_observer(apply_field_edit)
            .add_systems(
                PostUpdate,
                (
                    sync_entity_tree,
                    (store_column_splits, sync_details_panel).chain(),
                )
                    .before(UiSystems::Prepare),
            );

        #[cfg(feature = "remote")]
        app.init_resource::<remote::RemoteConnections>()
            .init_resource::<remote::RemoteSnapshots>()
            .init_resource::<remote::RemoteWorlds>()
            .add_systems(
                PostUpdate,
                (
                    remote::sync_remote_sources,
                    remote::poll_remote_connections,
                    remote::sync_remote_worlds,
                )
                    .chain()
                    .before(sync_entity_tree)
                    .before(sync_details_panel),
            )
            .add_systems(
                PostUpdate,
                connection_panel::sync_connection_status
                    .after(remote::poll_remote_connections)
                    .before(UiSystems::Prepare),
            );
    }
}
