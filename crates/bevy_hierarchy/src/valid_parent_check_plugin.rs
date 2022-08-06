use std::marker::PhantomData;

use bevy_app::{App, CoreStage, Plugin};
use bevy_core::Name;
use bevy_ecs::prelude::*;
use bevy_log::warn;
use bevy_utils::{get_short_name, HashSet};

use crate::Parent;

/// System to print a warning for each `Entity` with a `T` component
/// which parent hasn't a `T` component.
///
/// Hierarchy propagations are top-down, and limited only to entities
/// with a specific component (such as `ComputedVisibility` and `GlobalTransform`).
/// This means that entities with one of those component
/// and a parent without the same component is probably a programming error.
/// (See B0004 explanation linked in warning message)
pub fn check_hierarchy_component_has_valid_parent<T: Component>(
    parent_query: Query<
        (Entity, &Parent, Option<&Name>),
        (With<T>, Or<(Changed<Parent>, Added<T>)>),
    >,
    component_query: Query<(), With<T>>,
    mut already_diagnosed: Local<HashSet<Entity>>,
) {
    for (entity, parent, name) in &parent_query {
        let parent = parent.get();
        if !component_query.contains(parent) && !already_diagnosed.contains(&entity) {
            already_diagnosed.insert(entity);
            warn!(
                "warning[B0004]: {name} with the {ty_name} component has a parent without {ty_name}.\n\
                This will cause inconsistent behaviors! See https://bevyengine.org/learn/errors/#B0004",
                ty_name = get_short_name(std::any::type_name::<T>()),
                name = name.map_or("An entity".to_owned(), |s| format!("The {s} entity")),
            );
        }
    }
}

/// Print a warning for each `Entity` with a `T` component
/// which parent hasn't a `T` component.
///
/// See [`check_hierarchy_component_has_valid_parent`] for details.
pub struct ValidParentCheckPlugin<T>(PhantomData<fn() -> T>);
impl<T: Component> Default for ValidParentCheckPlugin<T> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<T: Component> Plugin for ValidParentCheckPlugin<T> {
    fn build(&self, app: &mut App) {
        app.add_system_to_stage(
            CoreStage::Last,
            check_hierarchy_component_has_valid_parent::<T>,
        );
    }
}
