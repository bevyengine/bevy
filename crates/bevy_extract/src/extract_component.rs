use crate::{
    sync_component::{SyncComponent, SyncComponentPlugin},
    sync_world::SubEntity,
    Extract, ExtractSchedule,
};
use bevy_app::{App, AppLabel, Plugin};
use bevy_camera::visibility::ViewVisibility;
use bevy_ecs::{
    bundle::NoBundleEffect,
    prelude::*,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
};
use core::marker::PhantomData;

pub use bevy_extract_macros::ExtractComponent;

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
pub trait ExtractComponent<L: AppLabel, F = ()>: SyncComponent<L, F> {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;
    /// The output from extraction, i.e. [`ExtractComponent::extract_component`].
    ///
    /// The output components won't be removed automatically from the render world if the implementing component is removed,
    /// unless you set them in the [`SyncComponent::Target`].
    type Out: Bundle<Effect: NoBundleEffect>;
    // TODO: https://github.com/rust-lang/rust/issues/29661
    // type Out: Bundle<Effect: NoBundleEffect> = Self;

    /// Defines how the component is transferred into the "render world".
    ///
    /// Returning `None` based on the queried item will remove the [`SyncComponent::Target`] from the entity in
    /// the render world.
    fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self::Out>;
}

/// This plugin extracts the components into the render world for synced
/// entities. To do so, it sets up the [`ExtractSchedule`] step for the
/// specified [`ExtractComponent`].
///
/// It also registers [`SyncComponentPlugin`](`crate::sync_component::SyncComponentPlugin`) to ensure the extracted components
/// are deleted if the main world components are removed.
///
/// The marker type `F` is only used as a way to bypass the orphan rules. To
/// implement the trait for a foreign type you can use a local type as the
/// marker, e.g. the type of the plugin that calls [`ExtractComponentPlugin`].
pub struct ExtractComponentPlugin<C, L: AppLabel, F = ()> {
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (C, L, F)>,
}

impl<C, L: AppLabel, F> Default for ExtractComponentPlugin<C, L, F> {
    fn default() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }
}

impl<C, L: AppLabel, F> ExtractComponentPlugin<C, L, F> {
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<
        C: ExtractComponent<L, F>,
        L: AppLabel + Default + Clone + Copy + Eq,
        F: 'static + Send + Sync,
    > Plugin for ExtractComponentPlugin<C, L, F>
{
    fn build(&self, app: &mut App) {
        app.add_plugins(SyncComponentPlugin::<C, L, F>::default());

        if let Some(render_app) = app.get_sub_app_mut(L::default()) {
            if self.only_extract_visible {
                render_app.add_systems(ExtractSchedule, extract_visible_components::<C, L, F>);
            } else {
                render_app.add_systems(ExtractSchedule, extract_components::<C, L, F>);
            }
        }
    }
}

/// This system extracts all components of the corresponding [`ExtractComponent`], for entities that are synced via [`crate::sync_world::SyncToRenderWorld`].
fn extract_components<C: ExtractComponent<L, F>, L: AppLabel + Clone + Copy + Eq, F>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(SubEntity<L>, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, query_item) in &query {
        if let Some(component) = C::extract_component(query_item) {
            values.push((entity, component));
        } else {
            commands.entity(entity).remove::<C::Target>();
        }
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}

/// This system extracts all components of the corresponding [`ExtractComponent`], for entities that are visible and synced via [`crate::sync_world::SyncToRenderWorld`].
fn extract_visible_components<C: ExtractComponent<L, F>, L: AppLabel + Clone + Copy + Eq, F>(
    mut commands: Commands,
    mut previous_len: Local<usize>,
    query: Extract<Query<(SubEntity<L>, &ViewVisibility, C::QueryData), C::QueryFilter>>,
) {
    let mut values = Vec::with_capacity(*previous_len);
    for (entity, view_visibility, query_item) in &query {
        if view_visibility.get() {
            if let Some(component) = C::extract_component(query_item) {
                values.push((entity, component));
            } else {
                commands.entity(entity).remove::<C::Target>();
            }
        }
    }
    *previous_len = values.len();
    commands.try_insert_batch(values);
}
