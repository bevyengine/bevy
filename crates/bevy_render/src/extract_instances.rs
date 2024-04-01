//! Convenience logic for turning components from the main world into extracted
//! instances in the render world.
//!
//! This is essentially the same as the `extract_component` module, but
//! higher-performance because it avoids the ECS overhead.

use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    entity::EntityHashMap,
    prelude::Entity,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
    system::{lifetimeless::Read, Query, ResMut, Resource},
};

use crate::{prelude::ViewVisibility, Extract, ExtractSchedule, RenderApp};

/// Describes how to extract data needed for rendering from a component or
/// components.
///
/// Before rendering, any applicable components will be transferred from the
/// main world to the render world in the [`ExtractSchedule`] step.
///
/// This is essentially the same as
/// [`ExtractComponent`](crate::extract_component::ExtractComponent), but
/// higher-performance because it avoids the ECS overhead.
pub trait ExtractInstance: Send + Sync + Sized + 'static {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;

    /// Defines how the component is transferred into the "render world".
    fn extract(item: QueryItem<'_, Self::QueryData>) -> Option<Self>;
}

/// This plugin extracts one or more components into the "render world" as
/// extracted instances.
///
/// Therefore it sets up the [`ExtractSchedule`] step for the specified
/// [`ExtractedInstances`].
#[derive(Default)]
pub struct ExtractInstancesPlugin<EI>
where
    EI: ExtractInstance,
{
    only_extract_visible: bool,
    marker: PhantomData<fn() -> EI>,
}

/// Stores all extract instances of a type in the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct ExtractedInstances<EI>(EntityHashMap<EI>)
where
    EI: ExtractInstance;

impl<EI> Default for ExtractedInstances<EI>
where
    EI: ExtractInstance,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<EI> ExtractInstancesPlugin<EI>
where
    EI: ExtractInstance,
{
    /// Creates a new [`ExtractInstancesPlugin`] that unconditionally extracts to
    /// the render world, whether the entity is visible or not.
    pub fn new() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }

    /// Creates a new [`ExtractInstancesPlugin`] that extracts to the render world
    /// if and only if the entity it's attached to is visible.
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<EI> Plugin for ExtractInstancesPlugin<EI>
where
    EI: ExtractInstance,
{
    fn build(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<ExtractedInstances<EI>>();
            if self.only_extract_visible {
                render_app.add_systems(ExtractSchedule, extract_visible::<EI>);
            } else {
                render_app.add_systems(ExtractSchedule, extract_all::<EI>);
            }
        }
    }
}

fn extract_all<EI>(
    mut extracted_instances: ResMut<ExtractedInstances<EI>>,
    query: Extract<Query<(Entity, EI::QueryData), EI::QueryFilter>>,
) where
    EI: ExtractInstance,
{
    extracted_instances.clear();
    for (entity, other) in &query {
        if let Some(extract_instance) = EI::extract(other) {
            extracted_instances.insert(entity, extract_instance);
        }
    }
}

fn extract_visible<EI>(
    mut extracted_instances: ResMut<ExtractedInstances<EI>>,
    query: Extract<Query<(Entity, &ViewVisibility, EI::QueryData), EI::QueryFilter>>,
) where
    EI: ExtractInstance,
{
    extracted_instances.clear();
    for (entity, view_visibility, other) in &query {
        if view_visibility.get() {
            if let Some(extract_instance) = EI::extract(other) {
                extracted_instances.insert(entity, extract_instance);
            }
        }
    }
}

impl<A> ExtractInstance for AssetId<A>
where
    A: Asset,
{
    type QueryData = Read<Handle<A>>;
    type QueryFilter = ();

    fn extract(item: QueryItem<'_, Self::QueryData>) -> Option<Self> {
        Some(item.id())
    }
}
