//! This module exists because of the orphan rule

use bevy_ecs::query::QueryItem;
use bevy_light::{cluster::ClusteredDecal, AmbientLight, ShadowFilteringMethod};

use crate::{extract_component::ExtractComponent, extract_resource::ExtractResource};

impl ExtractComponent for ClusteredDecal {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
impl ExtractResource for AmbientLight {
    type Source = Self;

    fn extract_resource(source: &Self::Source) -> Self {
        source.clone()
    }
}
impl ExtractComponent for AmbientLight {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(item.clone())
    }
}
impl ExtractComponent for ShadowFilteringMethod {
    type QueryData = &'static Self;
    type QueryFilter = ();
    type Out = Self;

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        Some(*item)
    }
}
