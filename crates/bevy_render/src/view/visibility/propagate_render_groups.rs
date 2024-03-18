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

### Propagator Repair

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

### Non-Propagator Hierarchy Repair

- If a non-propagator entity with InheritedRenderGroups is un-parented, then remove InheritedRenderGroups from the
entity and its children (stopping at propagators).
    - SYSTEM: handle_orphaned_nonpropagators
    - Iterate all children even if one without InheritedRenderGroups is encountered. This ensures 'gaps' caused
    by hierarchy changes won't cause problems. For example, an entity without InheritedRenderGroups parented to a
    descendent of an un-parented entity will want to pull inheritance from its parent, but removal of
    InheritedRenderGroups is deferred so the component it would access would be stale.
- If an entity loses a PropagateRenderGroups component, then inherit its parent's propagator entity, and propagate
that to its own children (stopping when a propagator is encountered or if an entity is non-updated and has an
InheritedRenderGroups that matches the propagator). If the parent isn't a propagator and doesn't
have InheritedRenderGroups (or there is no parent), then remove InheritedRenderGroups from the entity and its
children (stopping at propagators and non-updated descendents without InheritedRenderGroups). Skip already-updated
entities that lost PropagateRenderGroups.
    - SYSTEM: handle_lost_propagator
    - The goal of this step is to update the span of entities starting at the entity that lost a
    PropagateRenderGroups component, and ending at entities that aren't potentially-invalid.
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
propagator entity (stopping at propagators and descendents that share the parent's propagator). If the parent doesn't
have InheritedRenderGroups, then remove InheritedRenderGroups from the entity and its children (stopping at propagators
and children that don't have InheritedRenderGroups and aren't updated). Skip already-updated entities that were parented
to a non-propagator.
    - SYSTEMS: handle_new_children_nonpropagator, handle_new_parent_nonpropagator
    - The goal of this step is to update the span of entities starting at the entity that was reparented, and ending
    at entities that aren't potentially-invalid.
    - If the parent is marked updated (but the entity is not marked updated), then this entity's propagator
    stays the same (see "If a propagator gains non-propagator children"). In that case, do not mark the entity updated
    and simply skip it.
    - Force-update children for the same reason as "If an entity loses a PropagateRenderGroups component".
    - The implementation does not iterate non-propagator entities without InheritedRenderGroups that were parented
    to entities without InheritedRenderGroups. Issues that can arise from that case, such as other hierarchy moves
    below or above an entity, will be handled correctly by this and previous steps.
    - Note that the challenging hierarchy reasoning used here is necessary to allow efficient queries. A careless
    solution would require iterating all entities with Parent or Children changes, and traversing the hierarchy many
    times redundantly.

### Non-Propagator Targeted Repair

- If a non-propagator entity with InheritedRenderGroups has an added/removed/changed RenderGroups, then recompute
its InheritedRenderGroups::computed field. Skip already-updated entities.
    - SYSTEM: handle_modified_rendergroups


## Performance

The base-line performance cost of this algorithm comes from iterating in order to detect changes.
- All entities with `Children` and `PropagateRenderGroups` are iterated twice (can potentially be reduced to once).
- `RemovedComponents<RenderGroups>` is iterated twice.
- `RemovedComponents<CameraView>` is iterated once.
- `RemovedComponents<Camera>` is iterated once.
- `RemovedComponents<Parents>` is iterated once.
*/

use crate::view::{CameraView, RenderGroups};

use bevy_app::{App, PostUpdate, Plugin};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_utils::error_once;
use bevy_utils::tracing::warn;
use crate::prelude::Camera;

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
    pub fn get_render_groups<'a>(
        &'a self,
        entity: Entity,
        groups: Option<&'a RenderGroups>,
        view: Option<&CameraView>,
        is_camera: bool,
    ) -> PropagatingRenderGroups<'a> {
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
                let empty_view = CameraView::empty();
                let view = view.unwrap_or(&empty_view);
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
    pub propagator: Entity,
    /// The [`RenderGroups`] computed by merging the [`RenderGroups`] of the `Self::propagator` entity into
    /// the node's [`RenderGroups`] component.
    ///
    /// This is cached for efficient access in the [`check_visibility`] system.
    pub computed: RenderGroups,
}

impl InheritedRenderGroups {
    fn empty() -> Self {
        Self{ propagator: Entity::PLACEHOLDER, computed: RenderGroups::empty() }
    }
}

/// System set that applies [`PropagateRenderGroups`] by updating [`InheritedRenderGroups`] components on
/// entities.
#[derive(SystemSet, Debug, Clone, Hash, Eq, PartialEq)]
pub struct PropagateRenderGroupsSet;

pub(crate) struct PropagateRenderGroupsPlugin;

impl Plugin for PropagateRenderGroupsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PropagateRenderGroupsEntityCache>()
            .add_systems(PostUpdate,
                (
                    clean_propagators,
                    propagate_updated_propagators,
                    propagate_to_new_children,
                    handle_orphaned_nonpropagators,
                    handle_lost_propagator,
                    handle_new_children_nonpropagator,
                    handle_new_parent_nonpropagator,
                    apply_deferred,
                    handle_modified_rendergroups,  //does not have deferred commands
                )
                    .chain()
                    .in_set(PropagateRenderGroupsSet)
            );
    }
}

#[derive(Resource, Default, Deref, DerefMut)]
struct PropagateRenderGroupsEntityCache(EntityHashSet);

/// Removes InheritedRenderGroups from entities with PropagateRenderGroups.
fn clean_propagators(
    mut commands: Commands,
    dirty_propagators: Query<Entity, (With<InheritedRenderGroups>, With<PropagateRenderGroups>)>
){
    for dirty in dirty_propagators.iter() {
        commands.get_entity(dirty).map(|mut e| { e.remove::<InheritedRenderGroups>(); });
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
        Or<(
            Changed<RenderGroups>,
            Changed<CameraView>,
            Added<Camera>,
            Changed<PropagateRenderGroups>,
        )>
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
    let propagators = changed_propagators.iter()
        .chain(
            // IMPORTANT: Removals should be ordered first if propagate_to_new_children is merged
            //            into changed_propagators.
            removed_rendergroups.read()
                .chain(removed_cameraview.read())
                .chain(removed_camera.read())
                .filter_map(|e| all_propagators.get(e).ok())
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
        // TODO: This can allocate spuriously if there are no children that need it.
        let propagated: PropagatingRenderGroups<'_> = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Propagate
        updated_entities.insert(propagator);

        for child in children.iter().copied() {
            apply_full_propagation(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated.get(),
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
    let Ok((maybe_render_groups, maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Update inherited value or insert a new one.
    let empty_render_groups = RenderGroups::empty();
    let initial_groups = maybe_render_groups.unwrap_or(&empty_render_groups);
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
        commands.entity(entity).insert(new);
    }

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter().copied() {
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
        // TODO: This can allocate spuriously if there are no children that need it.
        let propagated = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Propagate
        updated_entities.insert(propagator);

        for child in children.iter().copied() {
            apply_new_children_propagation(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated.get(),
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
    let Ok((maybe_render_groups, maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Leave if the propagator is already known (implying this is a pre-existing child).
    if maybe_inherited_groups.as_ref().map(|i| i.propagator == propagator).unwrap_or(false) {
        return;
    }

    // Update inherited value or insert a new one.
    let empty_render_groups = RenderGroups::empty();
    let initial_groups = maybe_render_groups.unwrap_or(&empty_render_groups);
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
        commands.entity(entity).insert(new);
    }

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter().copied() {
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
    mut removed_parents: RemovedComponents<Parent>,
    orphaned: Query<
        (Entity, Option<&Children>),
        (Without<Parent>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>),
    >,
    // Query for getting non-propagator children.
    nonpropagators: Query<Option<&Children>, Without<PropagateRenderGroups>>,
) {
    for (orphan, maybe_children) in removed_parents.read().filter_map(|r| orphaned.get(r).ok()) {
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
    commands.get_entity(entity).map(|mut e| { e.remove::<InheritedRenderGroups>(); });

    // Mark as updated.
    updated_entities.insert(entity);

    // Update non-propagator children.
    let Some(children) = maybe_children else {
        return;
    };
    for child in children.iter().copied() {
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
    for (entity, maybe_parent) in removed_propagate.read().filter_map(|r| unpropagated.get(r).ok()) {
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
        // - Subcase 3: Recalculate InheritedRenderGroups with self-stored propagator
        //   - Policy: propagate value derived from propagator

        // Case 5: parent is a propagator
        // - Policy: propagate value derived from propagator

        // Case 6: parent is missing
        // - (this is a hierarchy error but we don't check it)
        // - Policy: remove all

        // Check cases where a value should be propagated.
        let propagator = if let Some(parent) = maybe_parent {
            if let Ok((propagator, ..)) = all_propagators.get(**parent) {
                // Case 5
                Some(propagator)
            } else if updated_entities.contains(&**parent){
                // Parent was marked updated (but self was not)
                if let Ok((_, maybe_inherited)) = maybe_inherited.get(entity) {
                    if let Some(inherited) = maybe_inherited {
                        // Case 4-2, 4-3
                        Some(inherited.propagator)
                    } else {
                        // Case 4-1
                        None
                    }
                } else {
                    // We already know entity is a non-propagator and exists
                    unreachable!();
                }
            } else {
                // Parent was not marked updated
                if let Ok((_, maybe_inherited)) = maybe_inherited.get(**parent) {
                    if let Some(inherited) = maybe_inherited {
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
        // - Case 3-2, 4-3
        // - Cases 3-1, 4-2 are filtered out here.
        if let Some((
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera,
            propagate
        )) = propagator.and_then(|p| all_propagators.get(p).ok()) {
            // Propagation value
            // TODO: This can allocate spuriously if there are no children that need it.
            let propagated = propagate.get_render_groups(
                propagator,
                maybe_render_groups,
                maybe_camera_view,
                maybe_camera
            );

            // Pre-update the entity as a hack for case 4-3. If we don't do this then
            // the entity will be caught in "Leave if entity is non-updated and inherits a matching propagator."
            // - Note: Case 4-3 is compensating for users manually inserting InheritedRenderGroups
            // components, so this could be simplified if that's deemed overkill (we don't fully compensate for
            // manual messing with InheritedRenderGroups anyway, so there is no real reliability for doing so).
            updated_entities.insert(entity);

            apply_full_propagation_force_update(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated.get(),
                entity
            );
        // In all other cases, remove all InheritedRenderGroups.
        } else {
            apply_full_propagation_force_remove(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &maybe_inherited,
                entity
            );
        }
    }
}

/// Applies propagation to entities for `handle_lost_propagator` and `handle_new_children_nonpropagator`.
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
    let Ok((maybe_render_groups, maybe_inherited_groups)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Leave if entity is non-updated and inherits a matching propagator.
    if let Some(inherited) = maybe_inherited_groups.as_ref() {
        if (inherited.propagator == propagator) && !updated_entities.contains(&entity) {
            return;
        }
    }

    // Force-update
    let empty_render_groups = RenderGroups::empty();
    let initial_groups = maybe_render_groups.unwrap_or(&empty_render_groups);

    let mut new = InheritedRenderGroups::empty();
    if let Some(mut inherited) = maybe_inherited_groups {
        // Steal existing allocation if possible.
        std::mem::swap(&mut *inherited, &mut new);
    }

    new.propagator = propagator;
    new.computed.set_from(initial_groups);
    new.computed.merge(propagated);
    commands.entity(entity).insert(new);

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter().copied() {
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

/// Applies InheritedRenderGroups removal to entities for `handle_lost_propagator`,
/// `handle_new_children_nonpropagator`, and `handle_new_parent_nonpropagator`.
fn apply_full_propagation_force_remove(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderGroupsEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderGroups.
    let Ok((_, maybe_inherited_inner)) = maybe_inherited.get(entity) else {
        return;
    };

    // Leave if entity is non-updated and doesn't have InheritedRenderGroups.
    if maybe_inherited_inner.is_none() && !updated_entities.contains(&entity) {
        return;
    }

    // Force-remove InheritedRenderGroups
    commands.entity(entity).remove::<InheritedRenderGroups>();

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter().copied() {
        apply_full_propagation_force_remove(commands, updated_entities, children_query, maybe_inherited, child);
    }
}

/// Handles non-propagator entities with InheritedRenderGroups whose children changed.
fn handle_new_children_nonpropagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Entities with InheritedRenderGroups that changed children
    inherited_with_children: Query<
        (Entity, &Children, &InheritedRenderGroups),
        (Changed<Children>, Without<PropagateRenderGroups>)
    >,
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
    for (entity, children, inherited) in inherited_with_children.iter() {
        // Skip entity if already updated, which implies children are already in an accurate state.
        if updated_entities.contains(&entity) {
            continue;
        }

        let Ok((
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera,
            propagate
        )) = all_propagators.get(inherited.propagator) else {
            // Remove InheritedRenderGroups from descendents if the propagator is missing
            // - This is either an error caused by manually modifying InheritedRenderGroups, or is caused by a
            // reparenting + propagator despawn.

            // Iterate children
            for child in children.iter().copied() {
                // Skip children that were already updated.
                // - Note that this can happen e.g. because the child lost the PropagateRenderGroups component.
                if updated_entities.contains(&child) {
                    continue;
                }

                // Propagate
                apply_full_propagation_force_remove(
                    &mut commands,
                    &mut updated_entities,
                    &children_query,
                    &maybe_inherited,
                    child
                );
            }

            continue;
        };

        // Get value to propagate.
        // TODO: This can allocate spuriously if there are no children that need it.
        let propagated = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Iterate children
        for child in children.iter().copied() {
            // Skip children that were already updated. We only skip updated children of this initial high-level
            // loop, not children within the recursion which need to be force-updated. The 'span' of entities
            // we update in this step starts at non-updated children of an entity with InheritedRenderGroups.
            // - Note that this can happen e.g. because the child lost the PropagateRenderGroups component.
            if updated_entities.contains(&child) {
                continue;
            }

            // Propagate
            apply_full_propagation_force_update(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                propagated.get(),
                child
            );
        }
    }
}

/// Handles non-propagator entities with InheritedRenderGroups whose parents changed.
/// - Since handle_new_children_nonpropagator handles all cases where the parent has InheritedRenderGroups, this
/// system just needs to remove InheritedRenderGroups from non-updated entities and their non-updated descendents
/// that have InheritedRenderGroups (stopping at propagators and non-updated descendents without
/// InheritedRenderGroups).
/// - We skip non-updated entities whose parents are updated, because that implies the current InheritedRenderGroups
/// propagator is accurate.
fn handle_new_parent_nonpropagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Entities with InheritedRenderGroups that changed parents
    inherited_with_parent: Query<
        (Entity, Option<&Children>, &Parent),
        (Changed<Parent>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>)
    >,
    // Query for Children
    children_query: Query<&Children>,
    // Query for updating InheritedRenderGroups on non-propagator entities.
    maybe_inherited: Query<
        (Option<&RenderGroups>, Option<&mut InheritedRenderGroups>),
        Without<PropagateRenderGroups>,
    >,
) {
    for (entity, maybe_children, parent) in inherited_with_parent.iter() {
        // Skip entity if already updated
        if updated_entities.contains(&entity) {
            continue;
        }

        // Skip entity if parent was updated
        if updated_entities.contains(&**parent) {
            continue;
        }

        // Remove from self.
        commands.entity(entity).remove::<InheritedRenderGroups>();

        // Mark as updated.
        updated_entities.insert(entity);

        // Iterate children.
        // - We assume the parent of this entity does NOT have InheritedRenderGroups, so neither should its
        // descendents.
        let Some(children) = maybe_children else {
            continue;
        };
        for child in children.iter().copied() {
            apply_full_propagation_force_remove(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &maybe_inherited,
                child
            );
        }
    }
}

/// Handles added/removed/changed RenderGroups for entities with existing InheritedRenderGroups.
fn handle_modified_rendergroups(
    mut updated_entities: ResMut<PropagateRenderGroupsEntityCache>,
    // Entities with InheritedRenderGroups that changed RenderGroups
    inherited_changed: Query<
        Entity,
        (Changed<RenderGroups>, With<InheritedRenderGroups>, Without<PropagateRenderGroups>)
    >,
    // RenderGroups removals.
    mut removed_rendergroups: RemovedComponents<RenderGroups>,
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
    // Query for updating InheritedRenderGroups on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderGroups>, &mut InheritedRenderGroups),
        Without<PropagateRenderGroups>,
    >,
) {
    for entity in inherited_changed.iter().chain(removed_rendergroups.read()) {
        // Skip entity if already updated.
        if updated_entities.contains(&entity) {
            continue;
        }

        // Skip entity if it's a propagator or doesn't exist.
        let Ok((entity_render_groups, mut inherited)) = maybe_inherited.get_mut(entity) else {
            continue;
        };

        // Skip entity if propagator is missing.
        // - This is an error, hierarchy steps should have marked this entity as updated.
        let Ok((
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera,
            propagate
        )) = all_propagators.get(inherited.propagator) else {
            error_once!("hierarchy error: propagator missing for {entity} in `handle_modified_rendergroups`");
            continue;
        };

        // Get propagated value.
        let propagated = propagate.get_render_groups(
            propagator,
            maybe_render_groups,
            maybe_camera_view,
            maybe_camera
        );

        // Update entity value.
        inherited.computed.set_from(entity_render_groups.unwrap_or(&RenderGroups::default()));
        inherited.computed.merge(propagated.get());

        // Mark updated (in case of duplicates due to removals).
        updated_entities.insert(entity);
    }
}
