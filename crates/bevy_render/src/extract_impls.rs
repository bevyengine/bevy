//! This module exists because of the orphan rule

use bevy_ecs::query::QueryItem;
use bevy_light::{cluster::ClusteredDecal, ShadowFilteringMethod};

use crate::extract_component::ExtractComponent;

impl ExtractComponent for ClusteredDecal {
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
