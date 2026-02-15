use crate::{
    sync_component::{SyncComponent, SyncComponentPlugin},
    sync_world::RenderEntity,
    Extract, ExtractSchedule,
};
use bevy_app::{App, AppLabel, InternedAppLabel, Plugin};
use bevy_camera::visibility::ViewVisibility;
use bevy_ecs::{
    prelude::*,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
};
use core::marker::PhantomData;

pub use bevy_extract_macros::ExtractBaseComponent;

/// Describes how a component gets extracted for rendering.
///
/// Therefore the component is transferred from the "app world" into the "render
/// world" in the [`ExtractSchedule`] step. This functionality is enabled by
/// adding [`ExtractComponentPlugin`] with the component type.
///
/// The Out type is defined in [`SyncComponent`].
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractComponentPlugin`].
pub trait ExtractBaseComponent<L, F = ()>: SyncComponent<F> {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;

    /// Defines how the component is transferred into the "render world".
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;
}

/// This plugin extracts the components into the render world for synced
/// entities. To do so, it sets up the [`ExtractSchedule`] step for the
/// specified [`ExtractBaseComponent`].
///
/// It also registers [`SyncComponentPlugin`] to ensure the extracted components
/// are deleted if the main world components are removed.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractComponentPlugin`].
pub struct ExtractComponentPlugin<C, F = ()> {
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (C, F)>,

    /// The [`AppLabel`](bevy_app::AppLabel) of the [`SubApp`] to set up with extraction.
    pub app_label: InternedAppLabel,
}

impl <C: ExtractBaseComponent<F>, F> ExtractComponentPlugin<C, F> {
    pub fn new<L: AppLabel>(app: L) -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
            app_label: app.intern(),
        }
    }

    pub fn extract_visible<L: AppLabel>(app: L) -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
            app_label: app.intern(),
        }
    }
}

impl<C: ExtractBaseComponent<F>, F: 'static + Send + Sync> Plugin for ExtractComponentPlugin<C, F> {
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<C, F>::default());

        if let Some(render_app) = app.get_sub_app_mut(self.app_label) {
            if self.only_extract_visible {
                render_app.add_systems(ExtractSchedule, extract_visible_components::<C, F>);
            } else {
                render_app.add_systems(ExtractSchedule, extract_components::<C, F>);
            }
        }
    }
}

/// This system extracts all components of the corresponding [`ExtractBaseComponent`], for entities that are synced via [`crate::sync_world::SyncToRenderWorld`].
fn extract_components<C: ExtractBaseComponent<F>, F>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(RenderEntity, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, query_item) in &query {
        if let Some(component) = C::extract_component(query_item) {
            values.push((entity, component));
        } else {
            commands.entity(entity).remove::<C::Out>();
        }
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

/// This system extracts all components of the corresponding [`ExtractBaseComponent`], for entities that are visible and synced via [`crate::sync_world::SyncToRenderWorld`].
fn extract_visible_components<C: ExtractBaseComponent<F>, F>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(RenderEntity, &ViewVisibility, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, view_visibility, query_item) in &query {
        if view_visibility.get() {
            if let Some(component) = C::extract_component(query_item) {
                values.push((entity, component));
            } else {
                commands.entity(entity).remove::<C::Out>();
            }
        }
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}
