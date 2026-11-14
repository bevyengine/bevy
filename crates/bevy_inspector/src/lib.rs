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
//! [`InspectorPlugin`]. The `inspector` cargo feature of `bevy` instead adds the
//! [`InspectorOverlayPlugin`](overlay::InspectorOverlayPlugin) to `DefaultPlugins`, so that any
//! app can open the inspector with a key.

extern crate alloc;

pub mod column_split;
#[cfg(feature = "remote")]
pub mod connection_panel;
pub mod details_panel;
pub mod entity_tree;
pub mod overlay;
#[cfg(feature = "remote")]
pub mod remote;

use alloc::string::{String, ToString};
use core::fmt::Display;

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::visibility::Visibility;
use bevy_dev_tools::inspection::label_resolution::LabelResolutionPlugin;
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    hierarchy::ChildOf,
    query::With,
    reflect::{AppTypeRegistry, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{In, IntoSystem, Local, Query, ResMut},
    world::World,
};
use bevy_platform::collections::HashSet;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::UiSystems;

use crate::column_split::ColumnSplitPlugin;
use crate::details_panel::{
    apply_field_edit, store_column_splits, sync_details_panel, DetailsCollapsed,
    DetailsColumnSplits, DetailsIndex, DetailsPanelSync, InspectorDetailsBody,
};
use crate::entity_tree::{sync_entity_tree, EntityTreeSync, InspectorTreeView, TreeRowIndex};

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

impl Display for WorldKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorldKind::Main => write!(f, "Main"),
            WorldKind::Render => write!(f, "Render"),
        }
    }
}

/// A single source to read data from
#[derive(Debug, Default, Clone, PartialEq, Copy, Eq, Reflect, Hash)]
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
pub(crate) fn world_to_inspect<'a>(
    world: &'a World,
    inspector_source: &'a InspectorSource,
) -> &'a World {
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

/// Returns `true` if an entity with the marker `M` exists with no [`Visibility::Hidden`] on itself
/// or its ancestors.
fn panel_visible<M: Component>(
    panels: Query<Entity, With<M>>,
    visibilities: Query<&Visibility>,
    parents: Query<&ChildOf>,
) -> bool {
    panels.iter().any(|panel| {
        core::iter::once(panel)
            .chain(parents.iter_ancestors(panel))
            .all(|entity| visibilities.get(entity) != Ok(&Visibility::Hidden))
    })
}

/// Returns `true` if an entity tree panel is showing.
pub fn entity_tree_visible(
    panels: Query<Entity, With<InspectorTreeView>>,
    visibilities: Query<&Visibility>,
    parents: Query<&ChildOf>,
) -> bool {
    panel_visible(panels, visibilities, parents)
}

/// Returns `true` if a details panel is showing.
pub fn details_panel_visible(
    panels: Query<Entity, With<InspectorDetailsBody>>,
    visibilities: Query<&Visibility>,
    parents: Query<&ChildOf>,
) -> bool {
    panel_visible(panels, visibilities, parents)
}

fn sync_tree_when_shown(
    In(visible): In<bool>,
    mut was_visible: Local<bool>,
    mut sync: ResMut<EntityTreeSync>,
) {
    if visible && !*was_visible {
        let duration = sync.timer.duration();
        sync.timer.set_elapsed(duration);
    }
    *was_visible = visible;
}

fn sync_details_when_shown(
    In(visible): In<bool>,
    mut was_visible: Local<bool>,
    mut sync: ResMut<DetailsPanelSync>,
) {
    if visible && !*was_visible {
        let duration = sync.timer.duration();
        sync.timer.set_elapsed(duration);
    }
    *was_visible = visible;
}

/// Plugin which registers the inspector resources and the entity tree synchronization systems.
///
/// This plugin does not add the widget plugins the inspector renders with; the app is expected to
/// add `bevy_feathers::FeathersPlugins` as well. Adding this plugin again has no effect.
///
/// The local tree and details synchronization systems only run while [`entity_tree_visible`] and
/// [`details_panel_visible`] are `true`.
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<Self>() {
            return;
        }
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
                    (
                        entity_tree_visible.pipe(sync_tree_when_shown),
                        sync_entity_tree.run_if(entity_tree_visible),
                    )
                        .chain(),
                    (
                        details_panel_visible.pipe(sync_details_when_shown),
                        store_column_splits,
                        sync_details_panel.run_if(details_panel_visible),
                    )
                        .chain(),
                )
                    .chain()
                    .ambiguous_with_all()
                    .before(UiSystems::Prepare)
                    .before(UiSystems::Stack),
            );

        #[cfg(feature = "remote")]
        app.init_resource::<remote::RemoteConnections>()
            .init_resource::<remote::RemoteSnapshots>()
            .init_resource::<remote::RemoteWorlds>()
            .init_resource::<remote::stepping::RemoteStepping>()
            .init_resource::<remote::stepping::SteppingStatus>()
            .add_systems(
                PostUpdate,
                (
                    remote::sync_remote_sources,
                    remote::poll_remote_connections,
                    remote::stepping::poll_remote_stepping,
                    remote::sync_remote_worlds,
                )
                    .chain()
                    .before(sync_entity_tree)
                    .before(sync_details_panel),
            )
            .add_systems(
                PostUpdate,
                (
                    connection_panel::init_address_input,
                    connection_panel::sync_connection_status,
                    remote::stepping::sync_stepping_controls
                        .after(remote::stepping::poll_remote_stepping),
                )
                    .after(remote::poll_remote_connections)
                    .before(UiSystems::Prepare),
            );
    }
}
