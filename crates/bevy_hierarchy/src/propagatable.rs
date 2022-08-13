use bevy_ecs::prelude::*;

use crate::{Children, Parent};

/// Marks a component as propagatable thrown hierachy alike `Transform`/`GlobalTransorm`
/// or `Visibility`/`ComputedVisibility`.
pub trait Propagatable: Component {
    /// The computed version of this component.
    type Computed: Component;
    /// The payload passed to children for computation.
    type Payload;

    /// If set to `false`, children are computed only if the hierarchy is changed
    /// or their local component changed.
    /// Otherwise, always compute all components.
    const ALWAYS_PROPAGATE: bool;

    /// Update computed component for root entity from it's local component.
    fn compute_root(computed: &mut Self::Computed, local: &Self);

    /// Update computed component from the parent's payload and the local component.
    fn compute(computed: &mut Self::Computed, payload: &Self::Payload, local: &Self);

    /// Compute the payload to pass to children from the computed component.
    fn payload(computed: &Self::Computed) -> Self::Payload;
}

type LocalQuery<'w, 's, 'a, T> = Query<
    'w,
    's,
    (
        &'a T,
        Changed<T>,
        &'a mut <T as Propagatable>::Computed,
        &'a Parent,
    ),
>;
type ChildrenQuery<'w, 's, 'a, T> = Query<
    'w,
    's,
    (&'a Children, Changed<Children>),
    (With<Parent>, With<<T as Propagatable>::Computed>),
>;

/// Update `T::Computed` component of entities based on entity hierarchy and
/// `T` component.
pub fn propagate_system<T: Propagatable>(
    mut root_query: Query<
        (
            Option<(&Children, Changed<Children>)>,
            &T,
            Changed<T>,
            &mut T::Computed,
            Entity,
        ),
        Without<Parent>,
    >,
    mut local_query: LocalQuery<T>,
    children_query: ChildrenQuery<T>,
) {
    for (children, local, local_changed, mut computed, entity) in root_query.iter_mut() {
        let mut changed = local_changed;
        if T::ALWAYS_PROPAGATE | changed {
            T::compute_root(computed.as_mut(), local);
        }

        if let Some((children, changed_children)) = children {
            // If our `Children` has changed, we need to recalculate everything below us
            changed |= changed_children;
            let payload = T::payload(computed.as_ref());
            for child in children {
                let _ = propagate_recursive(
                    &payload,
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

fn propagate_recursive<T: Propagatable>(
    payload: &T::Payload,
    local_query: &mut LocalQuery<T>,
    children_query: &ChildrenQuery<T>,
    entity: Entity,
    expected_parent: Entity,
    mut changed: bool,
    // BLOCKED: https://github.com/rust-lang/rust/issues/31436
    // We use a result here to use the `?` operator. Ideally we'd use a try block instead
) -> Result<(), ()> {
    let payload = {
        let (local, local_changed, mut computed, child_parent) =
            local_query.get_mut(entity).map_err(drop)?;
        // Note that for parallelising, this check cannot occur here, since there is an `&mut GlobalTransform` (in global_transform)
        assert_eq!(
            child_parent.get(), expected_parent,
            "Malformed hierarchy. This probably means that your hierarchy has been improperly maintained, or contains a cycle"
        );
        changed |= local_changed;
        if T::ALWAYS_PROPAGATE | changed {
            T::compute(computed.as_mut(), payload, local);
        }
        T::payload(computed.as_ref())
    };

    let (children, changed_children) = children_query.get(entity).map_err(drop)?;
    // If our `Children` has changed, we need to recalculate everything below us
    changed |= changed_children;
    for child in children {
        let _ = propagate_recursive(
            &payload,
            local_query,
            children_query,
            *child,
            entity,
            changed,
        );
    }
    Ok(())
}
