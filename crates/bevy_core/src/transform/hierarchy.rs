use bevy_ecs::{Entity, Query};
use bevy_transform::prelude::Children;

pub fn run_on_hierarchy<T, S>(
    children_query: &Query<&Children>,
    state: &mut S,
    entity: Entity,
    parent_result: Option<&mut T>,
    mut previous_result: Option<T>,
    run: &mut dyn FnMut(&mut S, Entity, Option<&mut T>, Option<T>) -> Option<T>,
) -> Option<T>
where
    T: Clone,
{
    // TODO: not a huge fan of this pattern. are there ways to do recursive updates in legion without allocations?
    // TODO: the problem above might be resolvable with world splitting
    let children = match children_query.get::<Children>(entity) {
        Ok(children) => Some(
            children
                .iter()
                .map(|entity| *entity)
                .collect::<Vec<Entity>>(),
        ),
        Err(_) => None,
    };

    let mut parent_result = run(state, entity, parent_result, previous_result);
    previous_result = None;
    if let Some(children) = children {
        for child in children {
            previous_result = run_on_hierarchy(
                children_query,
                state,
                child,
                parent_result.as_mut(),
                previous_result,
                run,
            );
        }
    } else {
        previous_result = parent_result;
    }

    previous_result
}
