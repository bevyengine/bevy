//! This module contains [`GhostNode`] and utilities to flatten the UI hierarchy, traversing past ghost nodes.

use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_render::view::{InheritedVisibility, ViewVisibility, Visibility};
use bevy_transform::prelude::{GlobalTransform, Transform};

/// Marker component for entities that should be ignored by within UI hierarchies.
///
/// The UI systems will traverse past these and consider their first non-ghost descendants as direct children of their first non-ghost ancestor.
///
/// Any components necessary for transform and visibility propagation will be added automatically.
#[derive(Component, Default, Debug, Copy, Clone, Reflect)]
#[reflect(Component, Debug)]
#[require(
    Visibility,
    InheritedVisibility,
    ViewVisibility,
    Transform,
    GlobalTransform
)]
pub struct GhostNode;
