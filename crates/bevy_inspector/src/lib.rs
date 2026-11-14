//! An entity inspector built on `bevy_ui` and `bevy_feathers`.
//!
//! The inspector presents the data gathered by [`bevy_dev_tools::inspection`] as interactive
//! widgets. This crate provides an entity tree panel, see [`entity_tree`], a list of the local
//! world's resources, see [`resource_list`], and a details panel for the selected entity, see
//! [`details_panel`].
//!
//! With the `remote` feature the same panels can inspect a separate running app over the Bevy
//! Remote Protocol, see the `remote` module.
//!
//! Apps are expected to add `bevy_feathers::FeathersPlugins` themselves, alongside
//! [`InspectorPlugin`]. The `inspector` cargo feature of `bevy` instead adds the
//! [`InspectorOverlayPlugin`](overlay::InspectorOverlayPlugin) to `DefaultPlugins`, so that any
//! app can open the inspector with a key.
//!
//! While the inspector is shown in [`InspectorMode::Edit`], clicking an entity in the app selects
//! it, and with the overlay the app does not receive those clicks. Hits come from the app's own
//! picking backends. Meshes can be clicked without `MeshPickingPlugin`, and when it is added its
//! settings and filters apply instead. In Edit mode the selected entity is also outlined with a
//! gizmo box. [`InspectorMode::Preview`] leaves clicks to the app and draws no outline.

extern crate alloc;

pub mod column_split;
#[cfg(feature = "remote")]
pub mod connection_panel;
pub mod details_panel;
pub mod entity_tree;
mod outline;
pub mod overlay;
mod picking;
#[cfg(feature = "remote")]
pub mod remote;
pub mod resource_list;
pub mod world_tabs;

use alloc::string::{String, ToString};
use core::{fmt::Display, marker::PhantomData};

use bevy_app::{App, Plugin, PostUpdate, PreUpdate};
use bevy_camera::visibility::Visibility;
use bevy_dev_tools::inspection::label_resolution::LabelResolutionPlugin;
use bevy_ecs::{
    component::{Component, ComponentId, ComponentInfo},
    entity::Entity,
    hierarchy::ChildOf,
    query::{Added, Has, Or, With},
    reflect::{AppTypeRegistry, ReflectResource},
    resource::Resource,
    schedule::{
        common_conditions::{any_match_filter, resource_changed},
        IntoScheduleConfigs, SystemCondition,
    },
    system::{In, IntoSystem, Local, Query, Res, ResMut},
    world::World,
};
use bevy_picking::{backend::PointerHits, pointer::PointerInput, PickingSystems};
use bevy_platform::collections::HashSet;
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_time::{Time, Timer, TimerMode};
use bevy_ui::{Display as NodeDisplay, Node, UiSystems};
use bevy_ui_widgets::SelectedTreeItem;

use crate::column_split::ColumnSplitPlugin;
use crate::details_panel::{
    apply_field_edit, store_column_splits, sync_details_panel, DetailsCollapsed,
    DetailsColumnSplits, DetailsIndex, DetailsPanelSync, InspectorDetailsBody,
};
use crate::entity_tree::{
    reveal_in_tree, sync_entity_tree, EntityTreeSync, InspectorEntityRow, InspectorHeaderRow,
    InspectorTreeView, TreeReveal, TreeRowIndex,
};
use crate::overlay::InspectorOverlay;
use crate::picking::select_clicked_entity;
use crate::resource_list::{
    sync_resource_list, InspectorResourceList, InspectorResourceRow, ResourceListSync,
};

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

/// Whether clicks in the app pick entities for the inspector or reach the app.
///
/// Only the overlay keeps clicks from the app, for its window and the UI below it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub enum InspectorMode {
    /// Clicks in the app select the entity under the cursor and do not reach the app. The
    /// selected entity is outlined.
    #[default]
    Edit,
    /// Clicks reach the app, nothing is picked and the selection is not outlined.
    Preview,
}

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

/// Pacing of the synchronization passes of the panels marked with `M`.
///
/// The first pass runs on the first tick, and a pass also runs on the tick a panel is shown again.
#[derive(Resource)]
pub struct PanelSync<M: Component> {
    /// Time between synchronization passes.
    pub timer: Timer,
    marker: PhantomData<fn() -> M>,
}

impl<M: Component> Default for PanelSync<M> {
    fn default() -> Self {
        let mut timer = Timer::from_seconds(0.25, TimerMode::Repeating);
        timer.set_elapsed(timer.duration());
        Self {
            timer,
            marker: PhantomData,
        }
    }
}

impl<M: Component> core::fmt::Debug for PanelSync<M> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PanelSync")
            .field("timer", &self.timer)
            .finish()
    }
}

impl<M: Component> PanelSync<M> {
    /// Forces a synchronization pass on the next tick.
    pub fn set_dirty(&mut self) {
        self.timer.almost_finish();
    }

    /// Advances the timer by the frame's [`Time`] delta, returning `true` if a pass is due.
    pub(crate) fn tick(world: &mut World) -> bool {
        let delta = world
            .get_resource::<Time>()
            .map(Time::delta)
            .unwrap_or_default();
        world
            .resource_mut::<Self>()
            .timer
            .tick(delta)
            .just_finished()
    }
}

fn sync_when_shown<M: Component>(
    In(visible): In<bool>,
    mut was_visible: Local<bool>,
    mut sync: ResMut<PanelSync<M>>,
) {
    if visible && !*was_visible {
        let duration = sync.timer.duration();
        sync.timer.set_elapsed(duration);
    }
    *was_visible = visible;
}

/// Returns `true` if an entity with the marker `M` exists with no [`Visibility::Hidden`] or
/// [`Display::None`](NodeDisplay::None) on itself or its ancestors.
fn panel_visible<M: Component>(
    panels: Query<Entity, With<M>>,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) -> bool {
    panels
        .iter()
        .any(|panel| shown(panel, &visibilities, &nodes, &parents))
}

pub(crate) fn shown(
    entity: Entity,
    visibilities: &Query<&Visibility>,
    nodes: &Query<&Node>,
    parents: &Query<&ChildOf>,
) -> bool {
    core::iter::once(entity)
        .chain(parents.iter_ancestors(entity))
        .all(|entity| {
            visibilities.get(entity) != Ok(&Visibility::Hidden)
                && !nodes
                    .get(entity)
                    .is_ok_and(|node| node.display == NodeDisplay::None)
        })
}

pub(crate) fn inspector_shown(
    overlays: Query<Entity, With<InspectorOverlay>>,
    panels: Query<
        Entity,
        Or<(
            With<InspectorTreeView>,
            With<InspectorResourceList>,
            With<InspectorDetailsBody>,
        )>,
    >,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) -> bool {
    let is_shown = |root: Entity| shown(root, &visibilities, &nodes, &parents);
    if overlays.is_empty() {
        panels.iter().any(is_shown)
    } else {
        overlays.iter().any(is_shown)
    }
}

/// Returns `true` if an entity tree panel is showing.
pub fn entity_tree_visible(
    panels: Query<Entity, With<InspectorTreeView>>,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) -> bool {
    panel_visible(panels, visibilities, nodes, parents)
}

/// Returns `true` if a resource list is showing.
pub fn resource_list_visible(
    panels: Query<Entity, With<InspectorResourceList>>,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) -> bool {
    panel_visible(panels, visibilities, nodes, parents)
}

/// Returns `true` if a details panel is showing.
pub fn details_panel_visible(
    panels: Query<Entity, With<InspectorDetailsBody>>,
    visibilities: Query<&Visibility>,
    nodes: Query<&Node>,
    parents: Query<&ChildOf>,
) -> bool {
    panel_visible(panels, visibilities, nodes, parents)
}

fn show_selection_in_lists(
    selection: Res<InspectorSelection>,
    index: Res<TreeRowIndex>,
    mut lists: Query<
        (Entity, &mut SelectedTreeItem, Has<InspectorResourceList>),
        Or<(With<InspectorTreeView>, With<InspectorResourceList>)>,
    >,
    resource_rows: Query<(Entity, &InspectorResourceRow, &ChildOf)>,
    header_rows: Query<(), With<InspectorHeaderRow>>,
    parents: Query<&ChildOf>,
) {
    let selection = selection.0.as_ref();
    for (list, mut selected, is_resource_list) in &mut lists {
        let row = if is_resource_list {
            selection
                .filter(|selection| selection.source == InspectorSource::Local)
                .and_then(|selection| {
                    resource_rows
                        .iter()
                        .find(|(_, row, parent)| {
                            parent.parent() == list && row.inspected == selection.entity
                        })
                        .map(|(row, ..)| row)
                })
        } else if selection.is_none() && selected.0.is_some_and(|row| header_rows.contains(row)) {
            continue;
        } else {
            selection
                .and_then(|selection| index.row(selection.entity, selection.source))
                .filter(|row| {
                    parents
                        .iter_ancestors(*row)
                        .any(|ancestor| ancestor == list)
                })
        };
        if selected.0 != row {
            selected.0 = row;
        }
    }
}

/// Plugin which registers the inspector resources and the entity tree synchronization systems.
///
/// This plugin does not add the widget plugins the inspector renders with; the app is expected to
/// add `bevy_feathers::FeathersPlugins` as well. Adding this plugin again has no effect.
///
/// The local tree, resource list and details synchronization systems only run while
/// [`entity_tree_visible`], [`resource_list_visible`] and [`details_panel_visible`] are `true`.
pub struct InspectorPlugin;

impl Plugin for InspectorPlugin {
    fn is_unique(&self) -> bool {
        false
    }

    fn finish(&self, app: &mut App) {
        outline::add_outline(app);
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
            .init_resource::<InspectorMode>()
            .init_resource::<TreeRowIndex>()
            .init_resource::<EntityTreeSync>()
            .init_resource::<TreeReveal>()
            .init_resource::<ResourceListSync>()
            .init_resource::<DetailsIndex>()
            .init_resource::<DetailsCollapsed>()
            .init_resource::<DetailsColumnSplits>()
            .init_resource::<DetailsPanelSync>()
            .add_message::<PointerHits>()
            .add_message::<PointerInput>()
            .add_observer(apply_field_edit)
            .add_systems(
                PreUpdate,
                inspector_shown
                    .pipe(select_clicked_entity)
                    .ambiguous_with_all()
                    .in_set(PickingSystems::Last),
            )
            .add_systems(
                PostUpdate,
                (
                    (
                        entity_tree_visible.pipe(sync_when_shown::<InspectorTreeView>),
                        (sync_entity_tree, reveal_in_tree)
                            .chain()
                            .run_if(entity_tree_visible),
                    )
                        .chain(),
                    (
                        resource_list_visible.pipe(sync_when_shown::<InspectorResourceList>),
                        sync_resource_list.run_if(resource_list_visible),
                    )
                        .chain(),
                    show_selection_in_lists.run_if(
                        resource_changed::<InspectorSelection>
                            .or_eager(any_match_filter::<Added<InspectorResourceRow>>)
                            .or_eager(any_match_filter::<Added<InspectorEntityRow>>),
                    ),
                    (
                        details_panel_visible.pipe(sync_when_shown::<InspectorDetailsBody>),
                        store_column_splits,
                        sync_details_panel.run_if(details_panel_visible),
                    )
                        .chain(),
                )
                    .chain()
                    .ambiguous_with_all()
                    .before(UiSystems::Prepare),
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
