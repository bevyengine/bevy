/*
# Notes on propagation algorithm for PropagateRenderGroups

The propagation algorithm is in control of updating InheritedRenderGroups on the descendents of
entities with PropagateRenderGroups. It must take into account changes in propagated values,
changes in RenderGroups on descendents, the possibility of entities gaining/losing PropagateRenderGroups,
and all potential hierarchy mutations (which can occur simultaneously with the other factors).

At the same time, the algorithm must be efficient. It should not update entities more than once (unless
necessary), and it should not update entities that don't need to be updated.

These goals are achieved with a sequence of update-handling steps that are logically accumulative. As much
as possible, each step covers only update-reasons not covered by previous steps. Note that most steps include
recursion for passing updates down hierarchies, however each step has slightly different propagation rules as
described below.


## Contributing factors

Dimensions:
- InheritedRenderGroups and PropagateRenderGroups should not exist on the same entity
- Add/remove/change RenderGroups
- Add/remove/change CameraView
- Add/remove/change PropagateRenderGroups
- Add/remove Camera
- Gain parent with PropagateRenderGroups
- Gain parent without PropagateRenderGroups
    - Entity with InheritedRenderGroups
    - Entity without InheritedRenderGroups
- Become un-parented

Actions:
- Add/remove/change InheritedRenderGroups
    - When updating an entity, always compute a fresh InheritedRenderGroups::computed value.


## Propagation algorithm

These steps must be done in order, and they are computed 'accumulatively', which means once
an entity has been updated it won't be updated again (unless explicitly specified).

We take note of a couple 'problems' that are challenging edge cases, and how they are handled.

### Propagators

- If an entity has both InheritedRenderGroups and PropagateRenderGroups, then remove its InheritedRenderGroups.
    - SYSTEM: clean_propagators
- If a propagator has an updated or new propagate-value, then propagate it. Stop propagating only if another propagator
is encountered.
    - SYSTEM: propagate_updated_propagators
    - Possible change sources: RenderGroups (add/remove/change), CameraView (add/remove/change), Camera (add/remove),
    PropagateRenderGroups (add/change).
- If a propagator gains non-propagator children, then propagate it to the new children. Stop propagating if the
propagator entity is already known to a descendent (do not mark those descendents as updated), or if another
propagator is encountered.
    - SYSTEM: propagate_to_new_children
    - PROBLEM: If a new entity is inserted between a propagator and another entity, then the propagator will run in
    this step, and the new entity's non-propagator children will run in "If a non-propagator entity is parented to a
    non-propagator". This step will insert an InheritedRenderGroups component to the new entity, but it *won't* be
    added to the pre-existing entity which already records the propagator. When the pre-eisting entity gets updated
    because of being parented to a non-propagator, the parent's InheritedRenderGroups component won't be available
    since its insertion is deferred.
        - SOLUTION: See "If a non-propagator entity is parented to a non-propagator".

### Non-Propagators

- If a non-propagator entity with InheritedRenderGroups is un-parented, then remove InheritedRenderGroups from the
entity and its children (stopping at propagators).
    - SYSTEM: handle_orphaned_nonpropagators
    - Iterate all children even if one without InheritedRenderGroups is encountered. This ensures 'gaps' caused
    by hierarchy changes won't cause problems. For example, an entity without InheritedRenderGroups parented to a
    descendent of an un-parented entity will want to pull inheritance from its parent, but removal of
    InheritedRenderGroups is deferred so the component it would access would be stale.
- If an entity loses a PropagateRenderGroups component, then inherit its parent's propagator entity, and propagate
that to its own children (stopping when a propagator is encountered). If the parent isn't a propagator and doesn't
have InheritedRenderGroups (or there is no parent), then remove InheritedRenderGroups from the entity and its
children (stopping at propagators). Skip already-updated entities that lost PropagateRenderGroups.
    - SYSTEM: handle_lost_propagator
    - If the parent is marked updated (but the entity is not marked updated), then this entity's propagator
    stays the same (see "If a propagator gains non-propagator children") (this can only happen if InheritedRenderGroups
    was manually inserted by a user, since the entity would not have had InheritedRenderGroups in the previous tick
    because we force-remove it if an entity has PropagateRenderGroups). In that case, instead of using the parent's
    InheritedRenderGroups, recompute this entity's InheritedRenderGroups from its existing propagator (we assume it
    is invalid/stale since the entity used to be a propagator).
    - PROBLEM: What if multiple entities in a hierarchy lose PropagateRenderGroups components?
        - SOLUTION: Ignore the updated_entities cache and force-update children even if they were previously updated.
        This will cause some redundant work, but gives the correct result.
            - If a child is marked as updated, then always insert/remove InheritedRenderGroups components to match
            the desired policy (regardless of it the entity has InheritedRenderGroups), in case previous deferred
            updates of this type need to be overwritten.
- If a non-propagator entity is parented to a non-propagator, then propagate the parent's InheritedRenderGroups
propagator entity (stopping at propagators). If the parent doesn't have
InheritedRenderGroups, then remove InheritedRenderGroups from the entity and its children (stopping at propagators).
Skip already-updated entities that were parented to a non-propagator.
    - If the parent is marked updated (but the entity is not marked updated), then this entity's propagator
    stays the same (see "If a propagator gains non-propagator children"). In that case, do not mark the entity updated
    and simply skip it.
    - Force-update children for the same reason as "If an entity loses a PropagateRenderGroups component".
- If a non-propagator entity with InheritedRenderGroups has an added/removed/changed RenderGroups, then recompute
its InheritedRenderGroups::computed field. Skip already-updated entities.


## Performance

The base-line performance cost of this algorithm comes from iterating in order to detect changes.
- All entities with `Children` and `PropagateRenderGroups` are iterated twice (can potentially be reduced to once).
- `RemovedComponents<RenderGroups>` is iterated twice.
- `RemovedComponents<CameraView>` is iterated once.
- `RemovedComponents<Camera>` is iterated once.
- `RemovedComponents<Parents>` is iterated once.
*/

/// Returned by [`PropagateRenderGroups::get_render_groups`].
pub enum PropagatingRenderGroups<'a> {
    Ref(&'a RenderGroups),
    Val(RenderGroups),
}

impl<'a> PropagatingRenderGroups<'a> {
    /// Gets a reference to the internal [`RenderGroups`].
    pub fn get(&self) -> &RenderGroups {
        match self {
            Self::Ref(groups) => groups,
            Self::Val(groups) => &groups,
        }
    }
}

/// Component on an entity that causes it to propagate a [`RenderGroups`] value to its children.
///
/// Entities with this component will ignore [`RenderGroups`] propagated by parents.
///
/// See [`RenderGroups`] and [`CameraView`].
#[derive(Component)]
pub enum PropagateRenderGroups
{
    /// If the entity has a [`RenderGroups`] component, that value is propagated, otherwise a default
    /// [`RenderGroups`] is propagated.
    ///
    /// Note that it is allowed to add a [`RenderGroup`] component to a camera.
    Auto,
    /// If the entity has a [`Camera`] component, propagates `RenderGroups::new_with_camera(entity)`.
    ///
    /// Otherwise a warning will be logged and an empty [`RenderGroups`] will be propagated.
    Camera,
    /// If the entity has a [`Camera`] component and a [`CameraView`] component, propagates
    /// `CameraView::get_groups(entity)`.
    ///
    /// Otherwise a warning will be logged and an empty [`RenderGroups`] will be propagated.
    CameraWithView,
    /// Propagates a custom [`RenderGroups`].
    Custom(RenderGroups),
}

impl PropagateRenderGroups {
    pub fn get_render_groups(
        &self,
        entity: Entity,
        groups: Option<&RenderGroups>,
        view: Option<&CameraView>,
        is_camera: bool,
    ) -> PropagatingRenderGroups<'_> {
        match self {
            Self::Auto =>
            {
                let Some(groups) = groups else {
                    return PropagatingRenderGroups::Val(RenderGroups::default());
                };
                PropagatingRenderGroups::Ref(groups)
            }
            Self::Camera =>
            {
                if !is_camera {
                    warn!("failed propagating PropagateRenderGroups::Camera, {entity} doesn't have a camera");
                    PropagatingRenderGroups::Val(RenderGroups::empty());
                };
                PropagatingRenderGroups::Val(RenderGroups::new_with_camera(entity))
            }
            Self::CameraWithView =>
            {
                if !is_camera {
                    warn!("failed propagating PropagateRenderGroups::CameraWithView, {entity} doesn't have a camera");
                    PropagatingRenderGroups::Val(RenderGroups::empty());
                };
                let view = view.unwrap_or(&CameraView::empty());
                PropagatingRenderGroups::Val(view.get_groups(entity))
            }
            Self::Custom(groups) =>
            {
                PropagatingRenderGroups::Ref(groups)
            }
        }
    }
}

/// Component on an entity that stores the result of merging the entity's [`RenderGroups`]
/// component with the [`RenderGroups`] of an entity propagated by the entity's parent.
///
/// See [`PropagateRenderGroups`].
///
/// This is automatically updated in [`PostUpdate`] in the [`VisibilityPropagate`] set.
/// The component will be automatically added or removed depending on if it is needed.
///
/// ### Merge details
///
/// The merge direction is 'entity_rendergroups.merge(propagated_rendergroups)`
/// (see [`RenderGroups::merge`]).
/// This means the entity's affiliated camera will be prioritized over the propagated affiliated camera.
#[derive(Component, Debug, Clone)]
pub struct InheritedRenderGroups
{
    /// The entity that propagated a [`RenderGroups`] to this entity.
    ///
    /// This is cached so children of this entity can update themselves without needing to traverse the
    /// entire hierarchy.
    pub propagater: Entity,
    /// The [`RenderGroups`] computed by merging the [`RenderGroups`] of the `Self::propagater` entity into
    /// the node's [`RenderGroups`] component.
    ///
    /// This is cached for efficient access in the [`check_visibility`] system.
    pub computed: RenderGroups,
};

//todo: insert resource
#[derive(Resource, Default, Deref, DerefMut)]
struct PropagateRenderGroupsEntityCache(EntityHashMap);

/// Removes InheritedRenderGroups from entities with PropagateRenderGroups.
fn clean_propagators(
    mut commands: Commands,
    dirty_propagators: Query<Entity, (With<InheritedRenderGroups>, With<PropagateRenderGroups>)>
){
    for dirty in dirty_propagators.iter() {
        commands.get_entity(dirty).map(|e| e.remove::<InheritedRenderGroups>())
    }
}

/// Propagates propagation values that have changed.
//todo: Detect if the propagated value has actually changed? Hard to expect this would matter in practice.
fn propagate_updated_propagators(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Detect added/changed: RenderGroups, CameraView, Camera (added only), PropagateRenderGroups.
    changed_propagators: Query<
        (
            Entity,
            &Children,
            Option<&RenderGroups>,
            Option<&CameraView>,
            Has<Camera>,
            &PropagateRenderGroups,
        ),
        (Or<(
            Changed<RenderGroups>,
            Changed<CameraView>,
            Added<Camera>,
            Changed<PropagateRenderGroups>,
        )>)
    >,
    // Detect removed: RenderGroups, CameraView, Camera.
    mut removed_rendergroups: RemovedComponents<RenderGroups>,
    mut removed_cameraview: RemovedComponents<CameraView>,
    mut removed_camera: RemovedComponents<Camera>,
    all_propagators: Query<
        (
            Entity,
            &Children,
            Option<&RenderGroups>,
            Option<&CameraView>,
            Has<Camera>,
            &PropagateRenderGroups,
        )
    >,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderGroups on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
) {
    // Track updated entities to prevent redundant updates, as `Commands` changes are deferred.
    //
    // Using this ensures that once mutations to entities with PropagateRenderGroups have been
    // propagated, all entities affected by those changes won't be mutated again. This makes it
    // safe to read parent InheritedRenderGroups (in the other cases that need to be handled) in
    // order to 'pull in' inherited changes when needed. See algorithm description for more details.
    updated_entities.clear();

    // Collect aggregate iterator for all propagators that need to propagate.
    let mut propagators = changed_propagators.iter()
        .chain(
            // IMPORTANT: Removals should be ordered first if propagate_to_new_children is merged
            //            into changed_propagators.
            removed_rendergroups.read()
                .chain(removed_cameraview.read())
                .chain(removed_camera.read())
                .filter_map(|e| all_propagators.get(*e).ok())
        );

    // Propagate each propagator.
    for (
        propagator,
        children,
        maybe_render_groups,
        maybe_camera_view,
        maybe_camera,
        propagate,
    ) in propagators {
        // There can be duplicates due to component removals.
        if updated_entities.contains(&propagator) {
            continue;
        }

        // Get value to propagate.
        // Note: This can allocate spuriously if there are no children that need it.
        let propagated = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Propagate
        updated_entities.insert(propagator);

        for child in children.iter() {
            apply_full_propagation(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated,
                child
            );
        }
    }
}

/// Applies propagation to entities for `apply_full_propagation`.
// Note: This does not require checking updated_entities because all children will be fresh.
fn apply_full_propagation(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
    propagator: Entity,
    propagated: &RenderGroups,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderGroups.
    let Ok((maybe_render_groups, mut maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    }

    // Update inherited value or insert a new one.
    let initial_groups = maybe_render_groups.unwrap_or(&RenderGroups::empty());
    let apply_changes = |groups: &mut InheritedRenderGroups| {
        groups.propagator = propagator;
        groups.computed.set_from(initial_groups);
        groups.computed.merge(propagated);
    };

    if let Some(mut inherited) = maybe_inherited_groups {
        apply_changes(&mut inherited);
    } else {
        let mut new = InheritedRenderGroups::empty();
        apply_changes(&mut new);
        commands.get(entity).insert(new);
    }

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        apply_full_propagation(
            commands,
            updated_entities,
            children_query,
            maybe_inherited,
            propagator,
            propagated,
            child
        );
    }
}

/// Propagates propagation values to new children of a propagator.
//todo: Can be merged with apply_full_propagation at the cost of code density. Must make sure iterating
//      this comes *after* iterating removals, because this step must come *after* all propagation value changes
//      are handled.
fn propagate_to_new_children(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Changed children.
    changed_children: Query<
        (
            Entity,
            &Children,
            Option<&RenderGroups>,
            Option<&CameraView>,
            Has<Camera>,
            &PropagateRenderGroups,
        ),
        Changed<Children>,
    >,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderGroups on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
) {
    // Propagate each propagator that has new children.
    for (
        propagator,
        children,
        maybe_render_groups,
        maybe_camera_view,
        maybe_camera,
        propagate,
    ) in changed_children.iter() {
        // The propagator could have been updated in a previous step.
        if updated_entities.contains(&propagator) {
            continue;
        }

        // Get value to propagate.
        // Note: This can allocate spuriously if there are no children that need it.
        let propagated = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Propagate
        updated_entities.insert(propagator);

        for child in children.iter() {
            apply_new_children_propagation(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated,
                child
            );
        }
    }
}

/// Applies propagation to entities for `propagate_to_new_children`.
// Note: This does not require checking updated_entities because all children will be fresh.
fn apply_new_children_propagation(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
    propagator: Entity,
    propagated: &RenderGroups,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderGroups.
    let Ok((maybe_render_groups, mut maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    }

    // Leave if the propagator is already known (implying this is a pre-existing child).
    if maybe_inherited_groups.map(|i| i.propagator == propagator).unwrap_or(false) {
        return;
    }

    // Update inherited value or insert a new one.
    let initial_groups = maybe_render_groups.unwrap_or(&RenderGroups::empty());
    let apply_changes = |groups: &mut InheritedRenderGroups| {
        groups.propagator = propagator;
        groups.computed.set_from(initial_groups);
        groups.computed.merge(propagated);
    };

    if let Some(mut inherited) = maybe_inherited_groups {
        apply_changes(&mut inherited);
    } else {
        let mut new = InheritedRenderGroups::empty();
        apply_changes(&mut new);
        commands.get(entity).insert(new);
    }

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        apply_new_children_propagation(
            commands,
            updated_entities,
            children_query,
            maybe_inherited,
            propagator,
            propagated,
            child
        );
    }
}

/// Removes InheritedRenderGroups from orphaned branches of the hierarchy.
fn handle_orphaned_nonpropagators(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Orphaned non-propagator entities that previously had InheritedRenderGroups.
    mut removed_parents: RemovedComponents<Parents>,
    orphaned: Query<
        (Entity, Option<&Children>),
        (Without<Parent>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>),
    >,
    // Query for getting non-propagator children.
    nonpropagators: Query<Option<&Children>, Without<PropagateRenderGroups>>,
) {
    for (orphan, maybe_children) in removed_parents.read().filter_map(|r| orphaned.get(r)) {
        apply_orphan_cleanup(&mut commands, &mut updated_entities, &nonpropagators, orphan, maybe_children);
    }
}

/// Applies propagation for `handle_orphaned_nonpropagators`.
fn apply_orphan_cleanup(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    nonpropagators: &Query<Option<&Children>, Without<PropagateRenderGroups>>,
    entity: Entity,
    maybe_children: Option<&Children>,
){
    // Remove InheritedRenderGroups.
    commands.get_entity(entity).map(|e| e.remove::<InheritedRenderGroups>());

    // Mark as updated.
    updated_entities.insert(entity);

    // Update non-propagator children.
    let Some(children) = maybe_children else {
        continue;
    };
    for child in children.iter() {
        // Ignore children that have PropagateRenderGroups.
        let Ok(maybe_children) = nonpropagators.get(child) else {
            continue;
        };

        apply_orphan_cleanup(commands, updated_entities, nonpropagators, child, maybe_children);
    }
}

/// Handles entities that lost the PropagateRenderGroups component.
fn handle_lost_propagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Entities that lost PropagateRenderGroups
    mut removed_propagate: RemovedComponents<PropagateRenderGroups>,
    unpropagated: Query<(Entity, Option<&Parent>), Without<PropagateRenderGroups>>,
    // Query for accessing propagators
    all_propagators: Query<
        (
            Entity,
            Option<&RenderGroups>,
            Option<&CameraView>,
            Has<Camera>,
            &PropagateRenderGroups,
        )
    >,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderGroups on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
) {
    for (entity, maybe_parent) in removed_propagate.read().filter_map(|r| unpropagated.get(r)) {
        // Skip already-updated entities.
        if updated_entities.contains(&entity) {
            continue;
        }

        // Apply propagation.

        // Case 1: no parent
        // - Policy: remove all

        // Case 2: parent is a non-propagator without InheritedRenderGroups (not marked updated)
        // - Policy: remove all

        // Case 3: parent is a non-propagator with InheritedRenderGroups (not marked updated)
        // - Subcase 1: Parent's propagator doesn't exit
        //   - Policy: remove all (note: this is not an error, because the propagation step to hendle it may not
        //     have executed yet)
        // - Subcase 2: Parent's propagator exists
        //   - Policy: Compute propagation value from parent's propagator

        // Case 4: parent is a non-propagator marked updated
        // - Subcase 1: Self doesn't have InheritedRenderGroups
        //   - Policy: remove all
        // - Subcase 2: Propagator stored in self's InheritedRenderGroups doesn't exist
        //   - Policy: remove all
        // - Subcase 3: Recalculate InheritedRenderGroups with stored propagator
        //   - Policy: propagate value derived from propagator

        // Case 5: parent is a propagator
        // - Policy: propagate value derived from propagator

        // Case 6: parent is missing
        // - (this is a hierarchy error but we don't check it)
        // - Policy: remove all

        // Check cases where a value should be propagated.
        let propagator = if let Some(parent) = maybe_parent {
            if let Ok((propagator, ..)) = all_propagators.get(parent) {
                // Case 5
                Some(propagator)
            } else if updated_entities.contains(parent){
                // Parent was marked updated (but self was not)
                let Ok((_, maybe_inherited)) = maybe_inherited.get(entity) {
                    let Some(inherited) = maybe_inherited {
                        // Case 4-1, 4-2
                        Some(inherited.propagator)
                    } else {
                        // Case 4-3
                        None
                    }
                } else {
                    // We already know entity is a non-propagator and exists
                    unreachable!();
                }
            } else {
                // Parent was not marked updated
                let Ok((_, maybe_inherited)) = maybe_inherited.get(parent) {
                    let Some(inherited) = maybe_inherited {
                        // Case 3-1, 3-2
                        Some(inherited.propagator)
                    } else {
                        // Case 2
                        None
                    }
                } else {
                    // Case 6
                    None
                }
            }
        } else {
            // Case 1
            None
        };

        // Propagate if possible
        // - Case 3-2, 4-2
        // - Cases 3-1, 4-1 are filtered out here.
        let Some((
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera,
            propagate
        )) = propagator.filter_map(|p| all_propagators.get(p)) {
            // Propagation value
            let propagated = propagate.get_render_groups(
                propagator,
                maybe_render_groups,
                maybe_camera_view,
                maybe_camera
            );

            apply_full_propagation_force_update(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated,
                entity
            );
        // In all other cases, remove all InheritedRenderGroups.
        } else {
            apply_full_propagation_force_remove(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                entity
            );
        }
    }
}

/// Applies propagation to entities for `handle_lost_propagator`.
// Note: This does not require checking updated_entities because we force-update children regardless of
// previous updates.
fn apply_full_propagation_force_update(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
    propagator: Entity,
    propagated: &RenderGroups,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderGroups.
    let Ok((maybe_render_groups, mut maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    }

    // Force-update
    let initial_groups = maybe_render_groups.unwrap_or(&RenderGroups::empty());

    let mut new = InheritedRenderGroups::empty();
    if let Some(mut inherited) = maybe_inherited_groups {
        // Steal existing allocation if possible.
        std::mem::swap(&mut inherited, &mut new);
    }

    new.propagator = propagator;
    new.computed.set_from(initial_groups);
    new.computed.merge(propagated);
    commands.get(entity).insert(new);

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        apply_full_propagation_force_update(
            commands,
            updated_entities,
            children_query,
            maybe_inherited,
            propagator,
            propagated,
            child
        );
    }
}

/// Applies InheritedRenderGroups-removal to entities for `handle_lost_propagator`.
// Note: This does not require checking updated_entities because we force-update children regardless of
// previous updates.
fn apply_full_propagation_force_remove(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderGroups.
    let Ok(_) = maybe_inherited.get_mut(entity) else {
        return;
    }

    // Force-remove InheritedRenderGroups
    commands.get(entity).remove::<InheritedRenderGroups>();

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter() {
        apply_full_propagation_force_remove(commands, updated_entities, children_query, maybe_inherited, child);
    }
}

/*

fn propagate_render_groups_updated_propagators(
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    mut commands: Commands,
    // Query for entities with PropagateRenderGroups that InheritedRenderGroups needs to be removed from.
    dirty_propagators: Query<Entity, (With<InheritedRenderGroups>, With<PropagateRenderGroups>)>,
    // Query for all propagators with changed PropagateRenderGroups.
    // This does a 'full propagation' to push changes all the way down the tree.
    changed_propagate_cameras_query: Query<
        (Entity, Option<&CameraView>, Option<&Camera>, Option<&RenderGroups>, &PropagateRenderGroups),
        Changed<PropagateRenderGroups>
    >,
    // Query for camera propagator entities with changed CameraView.
    // This does a 'full propagation' (depending on propagation mode) to push changes all the way down the tree.
    changed_view_cameras_query: Query<
        (Entity, &CameraView, Option<&RenderGroups>, &PropagateRenderGroups),
        (With<Camera>, Changed<CameraView>),
    >,
    // Tracker to identify entities that lost CameraView.
    mut removed_cameraview: RemovedComponents<CameraView>,
    // Query for all propagators with removed CameraView.
    // This does a 'full propagation' (depending on propagation mode) to push changes all the way down the tree.
    removed_cameraview_propagator_query: Query<
        (Entity, Option<&RenderGroups>, Option<&Camera>, &PropagateRenderGroups),
        Without<CameraView>
    >,
    // Query for all propagator entities with updated RenderGroups.
    // This does a 'full propagation' (depending on propagation mode) to push changes all the way down the tree.
    changed_rendergroups_propagator_query: Query<
        (Entity, &RenderGroups, &PropagateRenderGroups),
        (Changed<RenderGroups>)
    >,
    // Tracker to identify entities that lost RenderGroups.
    mut removed_rendergroups: RemovedComponents<RenderGroups>,
    // Query for propagator entities with removed RenderGroups.
    // This does a 'full propagation' (depending on propagation mode) to push changes all the way down the tree.
    removed_rendergroups_propagator_query: Query<
        (Entity, Option<&CameraView>, Option<&Camera>, &PropagateRenderGroups),
        Without<RenderGroups>
    >,

    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderGroups on entities.
    mut maybe_inherited_query: Query<(Entity, Option<&RenderGroups>, Option<&mut InheritedRenderGroups>)>,
) {
    // Track updated entities to prevent redundant updates, as `Commands` changes are deferred.
    //
    // Using this ensures that once mutations to entities with PropagateRenderGroups have been
    // propagated, all entities affected by those changes won't be mutated again. This makes it
    // safe to read parent InheritedRenderGroups (in the other cases that need to be handled) in
    // order to 'pull in' inherited changes when needed.
    updated_entities.clear();
}

//todo: chain after propagate_render_groups_full
fn propagate_render_groups_targeted(
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    mut commands: Commands,
    // Query for all propagators with changed children.
    // This does a 'propagator enity propagation' to push changes to only descendents with different propagator
    // entities.
    changed_children_propagator_query: Query<
        (Entity, Option<&CameraView>, Option<&Camera>, Option<&RenderGroups>, &PropagateRenderGroups),
        Changed<Children>,
    >,
    // Tracker to identify entities that lost PropagateRenderGroups.
    // - Takes into account where the entity has a parent.
    // - Takes into account whether the parent is a propagator or non-propagator.
    // These entities and their children will be 'repropagated' from the lost entities' parents.
    mut removed_propagate: RemovedComponents<PropagateRenderGroups>,
    removed_propagate_entities: Query<&Children, Without<PropagateRenderGroups>>,
    // Query for entities with InheritedRenderGroups who gained a new parent.
    // - Ignores entities whose parents have PropagateRenderGroups, since that case is handled by
    //   changed_children_propagator_query.
    // - If the parent doesn't have InheritedRenderGroups, then the entity and its children will be 'unpropagated'.
    // - If the parent does have InheritedRenderGroups, then propagation will be applied to the entity and
    //   its children.
    // This does a 'propagator enity propagation' to push changes to only descendents with different propagator
    // entities.
    changed_parents_query: Query<
        (Entity, Option<&Children>),
        (Changed<Parent>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>)
    >,
    // Query for entities with InheritedRenderGroups whose children changed.
    // - Ignores children with InheritedRenderGroups, those are handled by changed_parents_query.
    // This does a 'propagator enity propagation' to push changes to only descendents with different propagator
    // entities.
    changed_children_query: Query<
        (Entity, &Children),
        (Changed<Children>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>)
    >,
    // Query for non-propagator entities with updated RenderGroups.
    // This updates the entity's InheritedRenderGroups.
    changed_rendergroups_query: Query<
        (Entity, &RenderGroups),
        (Changed<RenderGroups>, Without<PropagateRenderGroups>)
    >,
    // Tracker to identify entities that lost RenderGroups.
    mut removed_rendergroups: RemovedComponents<RenderGroups>,
    // Query for non-propagator entities with InheritedRenderGroups and removed RenderGroups.
    // This updates the entity's InheritedRenderGroups.
    removed_rendergroups_query: Query<
        Entity,
        (With<InheritedRenderGroups>, Without<PropagateRenderGroups>)
    >,

    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for entities that propagate.
    // Used when pulling inheritance information from the parent.
    propagators: Query<
        (Entity, Option<&CameraView>, Option<&Camera>, Option<&RenderGroups>, &PropagateRenderGroups)
    >,
    // Query for entities that inherited and don't propagate.
    // Used when pulling inheritance information from the parent.
    nonpropagators: Query<(), (With<InheritedRenderGroups>, Without<PropagateRenderGroups>)>,
    // Query for updating InheritedRenderGroups on entities.
    mut maybe_inherited_query: Query<(Entity, Option<&RenderGroups>, Option<&mut InheritedRenderGroups>)>,
) {

    let camera_view = maybe_camera_view.unwrap_or(&CameraView::default());

    let mut propagated = if let Some(propagate) = maybe_propagate_groups {
        Some(propagate.get_from_camera(entity, camera_view))
    } else {
        None
    };

    // Assuming that TargetCamera is manually set on the root node only,
    // update root nodes first, since it implies the biggest change
    for (root_node, target_camera) in &changed_root_nodes_query {
        update_children_render_groups(
            root_node,
            target_camera,
            &node_query,
            &children_query,
            &mut commands,
            &mut updated_entities,
        );
    }

    // If the root node TargetCamera was changed, then every child is updated
    // by this point, and iteration will be skipped.
    // Otherwise, update changed children
    for (parent, target_camera) in &changed_children_query {
        update_children_render_groups(
            parent,
            target_camera,
            &node_query,
            &children_query,
            &mut commands,
            &mut updated_entities,
        );
    }
}

fn update_children_render_groups(
    updated_entities: &mut HashSet<Entity>,
    entity: Entity,
    camera_to_set: Option<&TargetCamera>,
    node_query: &Query<Option<&TargetCamera>, With<Node>>,
    children_query: &Query<&Children, With<Node>>,
    commands: &mut Commands,
) {
    let Ok(children) = children_query.get(entity) else {
        return;
    };

    for &child in children {
        // Skip if the child has already been updated or update is not needed
        if updated_entities.contains(&child)
            || camera_to_set == node_query.get(child).ok().flatten()
        {
            continue;
        }

        match camera_to_set {
            Some(camera) => {
                commands.entity(child).insert(camera.clone());
            }
            None => {
                commands.entity(child).remove::<TargetCamera>();
            }
        }
        updated_entities.insert(child);

        update_children_render_groups(
            child,
            camera_to_set,
            node_query,
            children_query,
            commands,
            updated_entities,
        );
    }
}
*/
