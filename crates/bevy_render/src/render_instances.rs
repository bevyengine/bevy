//! Convenience logic for turning components from the main world into render
//! instances in the render world.
//!
//! This is essentially the same as the `extract_component` module, but
//! higher-performance because it avoids the ECS overhead.

use std::marker::PhantomData;

use bevy_app::{App, Plugin};
use bevy_asset::{Asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Entity,
    query::{QueryItem, ReadOnlyWorldQuery, WorldQuery},
    system::{lifetimeless::Read, Query, ResMut, Resource},
};
use bevy_utils::EntityHashMap;

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
pub trait RenderInstance: Send + Sync + Sized + 'static {
    /// ECS [`WorldQuery`] to fetch the components to extract.
    type Query: WorldQuery + ReadOnlyWorldQuery;
    /// Filters the entities with additional constraints.
    type Filter: WorldQuery + ReadOnlyWorldQuery;

    /// Defines how the component is transferred into the "render world".
    fn extract_to_render_instance(item: QueryItem<'_, Self::Query>) -> Option<Self>;
}

/// This plugin extracts one or more components into the "render world" as
/// render instances.
///
/// Therefore it sets up the [`ExtractSchedule`](crate::ExtractSchedule) step
/// for the specified [`RenderInstances`].
#[derive(Default)]
pub struct RenderInstancePlugin<RI>
where
    RI: RenderInstance,
{
    only_extract_visible: bool,
    marker: PhantomData<fn() -> RI>,
}

/// Stores all render instances corresponding to the given component in the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderInstances<RI>(EntityHashMap<Entity, RI>)
where
    RI: RenderInstance;

impl<RI> Default for RenderInstances<RI>
where
    RI: RenderInstance,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<RI> RenderInstancePlugin<RI>
where
    RI: RenderInstance,
{
    /// Creates a new [`RenderInstancePlugin`] that unconditionally
    /// extracts the component to the render world, whether visible or not.
    pub fn new() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }
}

impl<RI> RenderInstancePlugin<RI>
where
    RI: RenderInstance,
{
    /// Creates a new [`RenderInstancePlugin`] that extracts the
    /// component to the render world if and only if the entity it's attached to
    /// is visible.
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<RI> Plugin for RenderInstancePlugin<RI>
where
    RI: RenderInstance,
{
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<RenderInstances<RI>>();
            if self.only_extract_visible {
                render_app.add_systems(ExtractSchedule, extract_visible_to_render_instances::<RI>);
            } else {
                render_app.add_systems(ExtractSchedule, extract_to_render_instances::<RI>);
            }
        }
    }
}

fn extract_to_render_instances<RI>(
    mut instances: ResMut<RenderInstances<RI>>,
    query: Extract<Query<(Entity, RI::Query), RI::Filter>>,
) where
    RI: RenderInstance,
{
    instances.clear();
    for (entity, other) in &query {
        if let Some(render_instance) = RI::extract_to_render_instance(other) {
            instances.insert(entity, render_instance);
        }
    }
}

fn extract_visible_to_render_instances<C>(
    mut instances: ResMut<RenderInstances<C>>,
    query: Extract<Query<(Entity, &ViewVisibility, C::Query), C::Filter>>,
) where
    C: RenderInstance,
{
    instances.clear();
    for (entity, view_visibility, other) in &query {
        if view_visibility.get() {
            if let Some(render_instance) = C::extract_to_render_instance(other) {
                instances.insert(entity, render_instance);
            }
        }
    }
}

impl<A> RenderInstance for AssetId<A>
where
    A: Asset,
{
    type Query = Read<Handle<A>>;
    type Filter = ();

    fn extract_to_render_instance(item: QueryItem<'_, Self::Query>) -> Option<Self> {
        Some(item.id())
    }
}
