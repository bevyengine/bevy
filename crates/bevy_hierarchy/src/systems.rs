use crate::components::*;
use bevy_ecs::{
    entity::Entity,
    prelude::Changed,
    query::Without,
    system::{Commands, Query},
};
use bevy_utils::HashMap;
use smallvec::SmallVec;

/// Updates parents when the hierarchy is changed
pub fn parent_update_system(
    mut commands: Commands,
    removed_parent_query: Query<(Entity, &PreviousParent), Without<Parent>>,
    mut parent_query: Query<(Entity, &Parent, Option<&mut PreviousParent>), Changed<Parent>>,
    mut children_query: Query<&mut Children>,
) {
    // Entities with a missing `Parent` (ie. ones that have a `PreviousParent`), remove
    // them from the `Children` of the `PreviousParent`.
    for (entity, previous_parent) in removed_parent_query.iter() {
        if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
            previous_parent_children.0.retain(|e| *e != entity);
            commands.entity(entity).remove::<PreviousParent>();
        }
    }

    // Tracks all newly created `Children` Components this frame.
    let mut children_additions = HashMap::<Entity, SmallVec<[Entity; 8]>>::default();

    // Entities with a changed Parent (that also have a PreviousParent, even if None)
    for (entity, parent, possible_previous_parent) in parent_query.iter_mut() {
        if let Some(mut previous_parent) = possible_previous_parent {
            // New and previous point to the same Entity, carry on, nothing to see here.
            if previous_parent.0 == parent.0 {
                continue;
            }

            // Remove from `PreviousParent.Children`.
            if let Ok(mut previous_parent_children) = children_query.get_mut(previous_parent.0) {
                (*previous_parent_children).0.retain(|e| *e != entity);
            }

            // Set `PreviousParent = Parent`.
            *previous_parent = PreviousParent(parent.0);
        } else {
            commands.entity(entity).insert(PreviousParent(parent.0));
        };

        // Add to the parent's `Children` (either the real component, or
        // `children_additions`).
        if let Ok(mut new_parent_children) = children_query.get_mut(parent.0) {
            // This is the parent
            // PERF: Ideally we shouldn't need to check for duplicates
            if !(*new_parent_children).0.contains(&entity) {
                (*new_parent_children).0.push(entity);
            }
        } else {
            // The parent doesn't have a children entity, lets add it
            children_additions
                .entry(parent.0)
                .or_insert_with(Default::default)
                .push(entity);
        }
    }

    // Flush the `children_additions` to the command buffer. It is stored separate to
    // collect multiple new children that point to the same parent into the same
    // SmallVec, and to prevent redundant add+remove operations.
    children_additions.iter().for_each(|(e, v)| {
        commands.entity(*e).insert(Children::with(v));
    });
}
