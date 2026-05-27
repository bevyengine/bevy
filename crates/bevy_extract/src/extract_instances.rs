//! Convenience logic for turning components from the main world into extracted
//! instances in the sub world.
//!
//! This is essentially the same as the `extract_component` module, but
//! higher-performance because it avoids the ECS overhead.

use core::marker::PhantomData;

use bevy_app::{App, AppLabel, Plugin};
use bevy_camera::visibility::ViewVisibility;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::Entity,
    query::{QueryFilter, QueryItem, ReadOnlyQueryData},
    resource::Resource,
    system::{Query, ResMut},
};

use crate::sync_world::MainEntityHashMap;
use crate::{Extract, ExtractSchedule};

/// Describes how to extract data needed for processing from a component or
/// components.
///
/// Before processing, any applicable components will be transferred from the
/// main world to the sub world in the [`ExtractSchedule`] step.
///
/// This is essentially the same as
/// [`ExtractComponent`](crate::extract_component::ExtractComponent), but
/// higher-performance because it avoids the ECS overhead.
pub trait ExtractInstance<L: AppLabel>: Send + Sync + Sized + 'static {
    /// ECS [`ReadOnlyQueryData`] to fetch the components to extract.
    type QueryData: ReadOnlyQueryData;
    /// Filters the entities with additional constraints.
    type QueryFilter: QueryFilter;

    /// Defines how the component is transferred into the "sub world".
    fn extract(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self>;
}

/// This plugin extracts one or more components into the "sub world" as
/// extracted instances.
///
/// Therefore it sets up the [`ExtractSchedule`] step for the specified
/// [`ExtractedInstances`].
#[derive(Default)]
pub struct ExtractInstancesPlugin<L, EI>
where
    L: AppLabel,
    EI: ExtractInstance<L>,
{
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (L, EI)>,
}

/// Stores all extract instances of a type in the sub world.
#[derive(Resource, Deref, DerefMut)]
pub struct ExtractedInstances<L, EI>(#[deref] MainEntityHashMap<EI>, PhantomData<L>)
where
    L: AppLabel,
    EI: ExtractInstance<L>;

impl<L, EI> Default for ExtractedInstances<L, EI>
where
    L: AppLabel,
    EI: ExtractInstance<L>,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<L, EI> ExtractInstancesPlugin<L, EI>
where
    L: AppLabel,
    EI: ExtractInstance<L>,
{
    /// Creates a new [`ExtractInstancesPlugin`] that unconditionally extracts to
    /// the sub world, whether the entity is visible or not.
    pub fn new() -> Self {
        Self {
            only_extract_visible: false,
            marker: PhantomData,
        }
    }

    /// Creates a new [`ExtractInstancesPlugin`] that extracts to the sub world
    /// if and only if the entity it's attached to is visible.
    pub fn extract_visible() -> Self {
        Self {
            only_extract_visible: true,
            marker: PhantomData,
        }
    }
}

impl<L, EI> Plugin for ExtractInstancesPlugin<L, EI>
where
    L: AppLabel + Default,
    EI: ExtractInstance<L>,
{
    fn build(&self, app: &mut App) {
        if let Some(sub_app) = app.get_sub_app_mut(L::default()) {
            sub_app.init_resource::<ExtractedInstances<L, EI>>();
            if self.only_extract_visible {
                sub_app.add_systems(ExtractSchedule, extract_visible::<L, EI>);
            } else {
                sub_app.add_systems(ExtractSchedule, extract_all::<L, EI>);
            }
        }
    }
}

fn extract_all<L, EI>(
    mut extracted_instances: ResMut<ExtractedInstances<L, EI>>,
    query: Extract<Query<(Entity, EI::QueryData), EI::QueryFilter>>,
) where
    L: AppLabel,
    EI: ExtractInstance<L>,
{
    extracted_instances.clear();
    for (entity, other) in &query {
        if let Some(extract_instance) = EI::extract(other) {
            extracted_instances.insert(entity.into(), extract_instance);
        }
    }
}

fn extract_visible<L, EI>(
    mut extracted_instances: ResMut<ExtractedInstances<L, EI>>,
    query: Extract<Query<(Entity, &ViewVisibility, EI::QueryData), EI::QueryFilter>>,
) where
    L: AppLabel,
    EI: ExtractInstance<L>,
{
    extracted_instances.clear();
    for (entity, view_visibility, other) in &query {
        if view_visibility.get()
            && let Some(extract_instance) = EI::extract(other)
        {
            extracted_instances.insert(entity.into(), extract_instance);
        }
    }
}
