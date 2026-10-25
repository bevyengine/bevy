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
pub struct ExtractInstancesPlugin<EI, L>
where
    EI: ExtractInstance<L>,
    L: AppLabel,
{
    only_extract_visible: bool,
    marker: PhantomData<fn() -> (L, EI)>,
}

/// Stores all extract instances of a type in the sub world.
#[derive(Resource, Deref, DerefMut)]
pub struct ExtractedInstances<EI, L>(#[deref] MainEntityHashMap<EI>, PhantomData<L>)
where
    EI: ExtractInstance<L>,
    L: AppLabel;

impl<EI, L> Default for ExtractedInstances<EI, L>
where
    EI: ExtractInstance<L>,
    L: AppLabel,
{
    fn default() -> Self {
        Self(Default::default(), PhantomData)
    }
}

impl<EI, L> ExtractInstancesPlugin<EI, L>
where
    EI: ExtractInstance<L>,
    L: AppLabel,
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

impl<EI, L> Plugin for ExtractInstancesPlugin<EI, L>
where
    EI: ExtractInstance<L>,
    L: AppLabel + Default,
{
    fn build(&self, app: &mut App) {
        if let Some(sub_app) = app.get_sub_app_mut(L::default()) {
            sub_app.init_resource::<ExtractedInstances<EI, L>>();
            if self.only_extract_visible {
                sub_app.add_systems(ExtractSchedule, extract_visible::<EI, L>);
            } else {
                sub_app.add_systems(ExtractSchedule, extract_all::<EI, L>);
            }
        }
    }
}

fn extract_all<EI, L>(
    mut extracted_instances: ResMut<ExtractedInstances<EI, L>>,
    query: Extract<Query<(Entity, EI::QueryData), EI::QueryFilter>>,
) where
    EI: ExtractInstance<L>,
    L: AppLabel,
{
    extracted_instances.clear();
    for (entity, other) in &query {
        if let Some(extract_instance) = EI::extract(other) {
            extracted_instances.insert(entity.into(), extract_instance);
        }
    }
}

fn extract_visible<EI, L>(
    mut extracted_instances: ResMut<ExtractedInstances<EI, L>>,
    query: Extract<Query<(Entity, &ViewVisibility, EI::QueryData), EI::QueryFilter>>,
) where
    EI: ExtractInstance<L>,
    L: AppLabel,
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
