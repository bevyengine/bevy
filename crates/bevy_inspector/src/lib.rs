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

use bevy_app::{App, Plugin, PostUpdate};
use bevy_camera::visibility::Visibility;
use bevy_dev_tools::inspection::label_resolution::LabelResolutionPlugin;
use bevy_ecs::{
    component::{ComponentId, ComponentInfo},
    entity::Entity,
    hierarchy::ChildOf,
    query::{Or, With},
    reflect::{AppTypeRegistry, ReflectResource},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{In, IntoSystem, Local, Query, ResMut},
    world::World,
};
use bevy_reflect::{prelude::ReflectDefault, Reflect};
use bevy_ui::UiSystems;

use crate::column_split::ColumnSplitPlugin;
use crate::details_panel::{
    apply_field_edit, store_column_splits, sync_details_panel, DetailsCollapsed,
    DetailsColumnSplits, DetailsIndex, DetailsPanelSync, InspectorDetailsBody,
};
use crate::entity_tree::{sync_entity_tree, EntityTreeSync, InspectorTreeView, TreeRowIndex};

/// Where the inspector reads its data from.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
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
pub(crate) fn world_to_inspect(world: &World) -> &World {
    #[cfg(feature = "remote")]
    if remote::is_remote(world)
        && let Some(remote) = world.get_resource::<remote::RemoteWorld>()
    {
        return remote.world();
    }
    world
}

/// The entity currently being inspected, as an id in the inspected world.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Resource, Debug, Default, Clone, PartialEq)]
pub struct InspectorSelection(pub Option<Entity>);

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

/// Returns `true` if an entity tree or details panel is showing, that is if one exists with no
/// [`Visibility::Hidden`] on itself or its ancestors.
pub fn inspector_visible(
    panels: Query<Entity, Or<(With<InspectorTreeView>, With<InspectorDetailsBody>)>>,
    visibilities: Query<&Visibility>,
    parents: Query<&ChildOf>,
) -> bool {
    panels.iter().any(|panel| {
        core::iter::once(panel)
            .chain(parents.iter_ancestors(panel))
            .all(|entity| visibilities.get(entity) != Ok(&Visibility::Hidden))
    })
}

fn sync_when_shown(
    In(visible): In<bool>,
    mut was_visible: Local<bool>,
    mut entity_tree: ResMut<EntityTreeSync>,
    mut details_panel: ResMut<DetailsPanelSync>,
) {
    if visible && !*was_visible {
        let duration = entity_tree.timer.duration();
        entity_tree.timer.set_elapsed(duration);
        let duration = details_panel.timer.duration();
        details_panel.timer.set_elapsed(duration);
    }
    *was_visible = visible;
}

/// Plugin which registers the inspector resources and the entity tree synchronization systems.
///
/// This plugin does not add the widget plugins the inspector renders with; the app is expected to
/// add `bevy_feathers::FeathersPlugins` as well. Adding this plugin again has no effect.
///
/// The local synchronization systems only run while [`inspector_visible`] is `true`.
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

        app.init_resource::<InspectorSource>()
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
                    inspector_visible.pipe(sync_when_shown),
                    (
                        sync_entity_tree.run_if(inspector_visible),
                        (
                            store_column_splits,
                            sync_details_panel.run_if(inspector_visible),
                        )
                            .chain(),
                    ),
                )
                    .chain()
                    .before(UiSystems::Prepare),
            );

        #[cfg(feature = "remote")]
        app.init_resource::<remote::RemoteConnection>()
            .init_resource::<remote::RemoteSnapshot>()
            .init_resource::<remote::RemoteWorld>()
            .init_resource::<remote::stepping::RemoteStepping>()
            .init_resource::<remote::stepping::SteppingStatus>()
            .add_systems(
                PostUpdate,
                (
                    remote::sync_remote_source,
                    remote::poll_remote_connection,
                    remote::stepping::poll_remote_stepping,
                    remote::sync_remote_world,
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
                    .after(remote::poll_remote_connection)
                    .before(UiSystems::Prepare),
            );
    }
}
