use bevy_ecs::{
    entity::Entity,
    prelude::{Changed, Component, With, Without},
    system::Query,
};

use crate::{Children, Parent};

/// Describe how the global state can be computed and updated from the local state
pub trait ComputedGlobal {
    /// Local component associated to this global component
    type Local: Component;

    /// Subset of `Self` that is needed to propagate information
    type ToPropagate;

    /// How to get the global state from the local state
    ///
    /// This is used on the root entity of the hierarchy, to initialize the global state from
    /// the local state of the root entity
    fn from_local(local: &Self::Local) -> Self;

    /// How to combine the propagated value with the local state of an entity to get its global
    /// state
    fn combine_with_local(propagated: &Self::ToPropagate, local: &Self::Local) -> Self;

    /// How to extract the value to propagate from a global state
    fn value_to_propagate(&self) -> Self::ToPropagate;
}

/// Propagate a component through a hierarchy. This is done by having two components, a local describing the
/// state of a given entity, and a global describing its state combined with the one of its parent. As the
/// local state changes during execution, the global state reflect those changes and is updated automatically.
pub fn propagate_system<Global: Component + ComputedGlobal>(
    mut root_query: Query<
        (
            Option<(&Children, Changed<Children>)>,
            &Global::Local,
            Changed<Global::Local>,
            &mut Global,
            Entity,
        ),
        Without<Parent>,
    >,
    mut local_query: Query<(&Global::Local, Changed<Global::Local>, &mut Global, &Parent)>,
    children_query: Query<(&Children, Changed<Children>), (With<Parent>, With<Global>)>,
) {
    for (children, local, local_changed, mut global, entity) in &mut root_query {
        let mut changed = local_changed;
        if local_changed {
            *global = Global::from_local(local);
        }

        if let Some((children, changed_children)) = children {
            // If our `Children` has changed, we need to recalculate everything below us
            changed |= changed_children;
            for child in children {
                let _ = propagate_recursive(
                    &global.value_to_propagate(),
                    &mut local_query,
                    &children_query,
                    *child,
                    entity,
                    changed,
                );
            }
        }
    }
}

fn propagate_recursive<Global: Component + ComputedGlobal>(
    parent: &Global::ToPropagate,
    local_query: &mut Query<(&Global::Local, Changed<Global::Local>, &mut Global, &Parent)>,
    children_query: &Query<(&Children, Changed<Children>), (With<Parent>, With<Global>)>,
    entity: Entity,
    expected_parent: Entity,
    mut changed: bool,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let global_value = {
        let (local, local_changed, mut global, child_parent) =
            local_query.get_mut(entity).map_err(drop)?;
        assert_eq!(
            child_parent.get(), expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        changed |= local_changed;
        if changed {
            *global = ComputedGlobal::combine_with_local(parent, local);
        }
        global.value_to_propagate()
    };

    let (children, changed_children) = children_query.get(entity).map_err(drop)?;
    // If our `Children` has changed, we need to recalculate everything below us
    changed |= changed_children;
    for child in children {
        let _ = propagate_recursive(
            &global_value,
            local_query,
            children_query,
            *child,
            entity,
            changed,
        );
    }
    Ok(())
}
