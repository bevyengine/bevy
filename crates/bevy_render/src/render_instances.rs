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
    component::Component,
    prelude::Entity,
    query::{QueryItem, ReadOnlyWorldQuery, WorldQuery},
    system::{lifetimeless::Read, Query, ResMut, Resource},
};
use bevy_utils::EntityHashMap;

use crate::{prelude::ViewVisibility, Extract, ExtractSchedule, RenderApp};

/// Describes how a component gets turned into a render instance for rendering.
///
/// Before rendering, a component will be transferred from the main world to the
/// render world in the [`ExtractSchedule`] step.
///
/// This is essentially the same as
/// [`ExtractComponent`](crate::extract_component::ExtractComponent), but
/// higher-performance because it avoids the ECS overhead.
pub trait ExtractToRenderInstance: Component {
    /// ECS [`WorldQuery`] to fetch the components to extract.
    type Query: WorldQuery + ReadOnlyWorldQuery;
    /// Filters the entities with additional constraints.
    type Filter: WorldQuery + ReadOnlyWorldQuery;

    type Instance: Send + Sync;

    /// Defines how the component is transferred into the "render world".
    fn extract_to_render_instance(item: QueryItem<'_, Self::Query>) -> Option<Self::Instance>;
}

/// This plugin extracts the components into the "render world" as render
/// instances.
///
/// Therefore it sets up the [`ExtractSchedule`](crate::ExtractSchedule) step
/// for the specified [`RenderInstances`].
#[derive(Default)]
pub struct ExtractToRenderInstancePlugin<C>
where
    C: ExtractToRenderInstance,
{
    only_extract_visible: bool,
    marker: PhantomData<fn() -> C>,
}

/// Stores all render instances corresponding to the given component in the render world.
#[derive(Resource, Deref, DerefMut)]
pub struct RenderInstances<C>(EntityHashMap<Entity, C::Instance>)
where
    C: ExtractToRenderInstance;

impl<C> Default for RenderInstances<C>
where
    C: ExtractToRenderInstance,
{
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<C> ExtractToRenderInstancePlugin<C>
where
    C: ExtractToRenderInstance,
{
    /// Creates a new [`ExtractToRenderInstancePlugin`] that unconditionally
    /// extracts the component to the render world, whether visible or not.
    pub fn new() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }
}

impl<C> ExtractToRenderInstancePlugin<C>
where
    C: ExtractToRenderInstance,
{
    /// Creates a new [`ExtractToRenderInstancePlugin`] that extracts the
    /// component to the render world if and only if the entity it's attached to
    /// is visible.
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<C> Plugin for ExtractToRenderInstancePlugin<C>
where
    C: ExtractToRenderInstance,
{
    fn build(&self, app: &mut App) {
        if let Ok(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.init_resource::<RenderInstances<C>>();
            if self.only_extract_visible {
                render_app.add_systems(ExtractSchedule, extract_visible_to_render_instances::<C>);
            } else {
                render_app.add_systems(ExtractSchedule, extract_to_render_instances::<C>);
            }
        }
    }
}

fn extract_to_render_instances<C>(
    mut instances: ResMut<RenderInstances<C>>,
    query: Extract<Query<(Entity, C::Query), C::Filter>>,
) where
    C: ExtractToRenderInstance,
{
    instances.clear();
    for (entity, other) in &query {
        if let Some(render_instance) = C::extract_to_render_instance(other) {
            instances.insert(entity, render_instance);
        }
    }
}

fn extract_visible_to_render_instances<C>(
    mut instances: ResMut<RenderInstances<C>>,
    query: Extract<Query<(Entity, &ViewVisibility, C::Query), C::Filter>>,
) where
    C: ExtractToRenderInstance,
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

impl<A> ExtractToRenderInstance for Handle<A>
where
    A: Asset,
{
    type Query = Read<Handle<A>>;
    type Filter = ();
    type Instance = AssetId<A>;

    fn extract_to_render_instance(item: QueryItem<'_, Self::Query>) -> Option<Self::Instance> {
        Some(item.id())
    }
}
