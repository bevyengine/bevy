/*
# Notes on propagation algorithm for PropagateRenderLayers

The propagation algorithm is in control of updating InheritedRenderLayers on the descendents of
entities with PropagateRenderLayers. It must take into account changes in propagated values,
changes in RenderLayers on descendents, the possibility of entities gaining/losing PropagateRenderLayers,
and all potential hierarchy mutations (which can occur simultaneously with the other factors).

At the same time, the algorithm must be efficient. It should not update entities more than once (unless
necessary), and it should not update entities that don't need to be updated.

These goals are achieved with a sequence of update-handling steps that are logically accumulative. As much
as possible, each step covers only update-reasons not covered by previous steps. Note that most steps include
recursion for passing updates down hierarchies, however each step has slightly different propagation rules as
described below.


## Contributing factors

Dimensions:
- InheritedRenderLayers and PropagateRenderLayers should not exist on the same entity
- RenderLayers can be added/changed/removed
- CameraLayer can be added/changed/removed
- PropagateRenderLayers can be added/changed/removed
- Camera can be added/removed (changes don't matter)
- Entities can gain a parent with PropagateRenderLayers
- Entities can gain a parent without PropagateRenderLayers
    - The entity may or may not have InheritedRenderLayers already
    - The parent may or may not have InheritedRenderLayers already
- Entities can become un-parented

Actions:
- This algorithm can: add/remove/change InheritedRenderLayers
    - When updating an entity, we always compute a fresh InheritedRenderLayers::computed value.


## Propagation algorithm

These steps must be done in order, and they are computed 'accumulatively', which means once
an entity has been updated it won't be updated again (unless explicitly specified).

We take note of a couple 'problems' that are challenging edge cases, and how they are handled.

### Propagator Repair

- If an entity has both InheritedRenderLayers and PropagateRenderLayers, then remove its InheritedRenderLayers.
    - SYSTEM: clean_propagators
- If a propagator has an updated or new propagate-value, then propagate it. Stop propagating only if another propagator
is encountered.
    - SYSTEM: propagate_updated_propagators
    - Possible change sources: RenderLayers (add/remove/change), CameraLayer (add/remove/change), Camera (add/remove),
    PropagateRenderLayers (add/change).
- If a propagator gains non-propagator children, then propagate it to the new children. Stop propagating if the
propagator entity is already known to a descendent (do not mark those descendents as updated), or if another
propagator is encountered.
    - SYSTEM: propagate_to_new_children
    - PROBLEM: If a new entity is inserted between a propagator and another entity, then the propagator will run in
    this step, and the new entity's non-propagator children will run in "If a non-propagator entity is parented to a
    non-propagator". This step will insert an InheritedRenderLayers component to the new entity, but it *won't* be
    added to the pre-existing entity which already records the propagator. When the pre-eisting entity gets updated
    because of being parented to a non-propagator, the parent's InheritedRenderLayers component won't be available
    since its insertion is deferred.
        - SOLUTION: See "If a non-propagator entity is parented to a non-propagator".

### Non-Propagator Hierarchy Repair

- If a non-propagator entity with InheritedRenderLayers is un-parented, then remove InheritedRenderLayers from the
entity and its children (stopping at propagators).
    - SYSTEM: handle_orphaned_nonpropagators
    - Iterate all children even if one without InheritedRenderLayers is encountered. This ensures 'gaps' caused
    by hierarchy changes won't cause problems. For example, an entity without InheritedRenderLayers parented to a
    descendent of an un-parented entity will want to pull inheritance from its parent, but removal of
    InheritedRenderLayers is deferred so the component it would access would be stale.
- If an entity loses a PropagateRenderLayers component, then inherit its parent's propagator entity, and propagate
that to its own children (stopping when a propagator is encountered or if an entity is non-updated and has an
InheritedRenderLayers that matches the propagator). If the parent isn't a propagator and doesn't
have InheritedRenderLayers (or there is no parent), then remove InheritedRenderLayers from the entity and its
children (stopping at propagators and non-updated descendents without InheritedRenderLayers). Skip already-updated
entities that lost PropagateRenderLayers.
    - SYSTEM: handle_lost_propagator
    - The goal of this step is to update the span of entities starting at the entity that lost a
    PropagateRenderLayers component, and ending at entities that aren't potentially-invalid.
    - If the parent is marked updated (but the entity is not marked updated), then this entity's propagator
    stays the same (see "If a propagator gains non-propagator children") (this can only happen if InheritedRenderLayers
    was manually inserted by a user, since the entity would not have had InheritedRenderLayers in the previous tick
    because we force-remove it if an entity has PropagateRenderLayers). In that case, instead of using the parent's
    InheritedRenderLayers, recompute this entity's InheritedRenderLayers from its existing propagator (we assume it
    is invalid/stale since the entity used to be a propagator).
    - PROBLEM: What if multiple entities in a hierarchy lose PropagateRenderLayers components?
        - SOLUTION: Ignore the updated_entities cache and force-update children even if they were previously updated.
        This will cause some redundant work, but gives the correct result.
            - If a child is marked as updated, then always insert/remove InheritedRenderLayers components to match
            the desired policy (regardless of it the entity has InheritedRenderLayers), in case previous deferred
            updates of this type need to be overwritten.
- If a non-propagator entity is parented to a non-propagator, then propagate the parent's InheritedRenderLayers
propagator entity (stopping at propagators and descendents that share the parent's propagator). If the parent doesn't
have InheritedRenderLayers, then remove InheritedRenderLayers from the entity and its children (stopping at propagators
and children that don't have InheritedRenderLayers and aren't updated). Skip already-updated entities that were parented
to a non-propagator.
    - SYSTEMS: handle_new_children_nonpropagator, handle_new_parent_nonpropagator
    - The goal of this step is to update the span of entities starting at the entity that was reparented, and ending
    at entities that aren't potentially-invalid.
    - If the parent is marked updated (but the entity is not marked updated), then this entity's propagator
    stays the same (see "If a propagator gains non-propagator children"). In that case, do not mark the entity updated
    and simply skip it.
    - Force-update children for the same reason as "If an entity loses a PropagateRenderLayers component".
    - The implementation does not iterate non-propagator entities without InheritedRenderLayers that were parented
    to entities without InheritedRenderLayers. Issues that can arise from that case, such as other hierarchy moves
    below or above an entity, will be handled correctly by this and previous steps.
    - Note that the challenging hierarchy reasoning used here is necessary to allow efficient queries. A careless
    solution would require iterating all entities with Parent or Children changes, and traversing the hierarchy many
    times redundantly.

### Non-Propagator Targeted Repair

- If a non-propagator entity with InheritedRenderLayers has an added/removed/changed RenderLayers, then recompute
its InheritedRenderLayers::computed field. Skip already-updated entities.
    - SYSTEM: handle_modified_renderlayers


## Performance

The base-line performance cost of this algorithm comes from detecting changes, which requires iterating queries.
- All entities with `Children` and `PropagateRenderLayers` are iterated twice (can potentially be reduced to once).
- All entities with `Children` and `InheritedRenderLayers` are iterated once.
- All entities with `Parent` and `InheritedRenderLayers` are iterated once.
- All entities with `RenderLayers` and `InheritedRenderLayers` are iterated once.
- `RemovedComponents<RenderLayers>` is iterated twice.
- `RemovedComponents<CameraLayer>` is iterated once.
- `RemovedComponents<Camera>` is iterated once.
- `RemovedComponents<Parent>` is iterated once.
*/

#![allow(clippy::manual_map, clippy::collapsible_match)]

use crate::view::*;

use crate::prelude::Camera;
use bevy_app::PostUpdate;
use bevy_derive::Deref;
use bevy_ecs::entity::EntityHashSet;
use bevy_ecs::prelude::*;
use bevy_hierarchy::{Children, Parent};
use bevy_utils::error_once;
use bevy_utils::tracing::warn;

/// System set that applies [`PropagateRenderLayers`] by updating [`InheritedRenderLayers`] components on
/// entities.
///
/// Runs in [`PostUpdate`] in the [`VisibilityPropagate`](VisibilitySystems::VisibilityPropagate) set.
#[derive(SystemSet, Debug, Clone, Hash, Eq, PartialEq)]
pub struct PropagateRenderLayersSet;

pub(crate) struct PropagateRenderLayersPlugin;

impl Plugin for PropagateRenderLayersPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PropagateRenderLayersEntityCache>()
            .add_systems(
                PostUpdate,
                (
                    clean_propagators,
                    propagate_updated_propagators,
                    propagate_to_new_children,
                    handle_orphaned_nonpropagators,
                    handle_lost_propagator,
                    handle_new_children_nonpropagator,
                    handle_new_parent_nonpropagator,
                    apply_deferred,
                    handle_modified_renderlayers, //does not have deferred commands
                )
                    .chain()
                    .in_set(PropagateRenderLayersSet),
            );
    }
}

/// Component on an entity that causes it to propagate a [`RenderLayers`] value to its children.
///
/// Entities with this component will ignore [`RenderLayers`] propagated by parents.
///
/// See [`RenderLayers`] and [`CameraLayer`].
#[derive(Component)]
pub enum PropagateRenderLayers {
    /// If the entity has a [`RenderLayers`] component, that value is propagated, otherwise a default
    /// [`RenderLayers`] is propagated.
    ///
    /// Note that it is allowed to add a [`RenderLayers`] component to a camera for propagation.
    Auto,
    /// If the entity has a [`Camera`] component and a [`CameraLayer`] component, propagates
    /// [`CameraLayer::get_layers`]. Uses [`CameraLayer::default`] if there is no [`CameraLayer`].
    ///
    /// If there is no [`Camera`] component, a warning will be logged and an empty [`RenderLayers`] will be propagated.
    Camera,
    /// Propagates a custom [`RenderLayers`].
    Custom(RenderLayers),
}

impl PropagateRenderLayers {
    pub fn get_render_layers<'a>(
        &'a self,
        saved: &mut RenderLayers,
        layers: Option<&'a RenderLayers>,
        view: Option<&CameraLayer>,
        is_camera: bool,
    ) -> RenderLayersRef<'a> {
        match self {
            Self::Auto => {
                let Some(layers) = layers else {
                    return RenderLayersRef::Val(RenderLayers::default());
                };
                RenderLayersRef::Ref(layers)
            }
            Self::Camera => {
                if !is_camera {
                    warn!("failed propagating PropagateRenderLayers::Camera, entity doesn't have a camera");
                    return RenderLayersRef::Val(RenderLayers::empty());
                };
                let default_camera_layer = CameraLayer::default();
                let view = view.unwrap_or(&default_camera_layer);

                // Reuse saved allocation.
                saved.clear();
                if let Some(layer) = view.layer() {
                    saved.add(layer);
                }
                let mut temp = RenderLayers::empty();
                std::mem::swap(&mut temp, saved);
                RenderLayersRef::Val(temp)
            }
            Self::Custom(layers) => RenderLayersRef::Ref(layers),
        }
    }
}

/// Component on an entity that stores the result of merging the entity's [`RenderLayers`]
/// component with the [`RenderLayers`] of an entity propagated by the entity's parent.
///
/// See [`PropagateRenderLayers`].
///
/// This is updated in [`PostUpdate`] in the [`PropagateRenderLayersSet`].
/// The component will be automatically added or removed depending on if it is needed.
///
/// ### Merge details
///
/// The merge direction is `entity_renderLayers.merge(propagated_renderLayers)`
/// (see [`RenderLayers::merge`]).
/// This means the entity's affiliated camera will be prioritized over the propagated affiliated camera.
#[derive(Component, Debug, Clone)]
pub struct InheritedRenderLayers {
    /// The entity that propagated a [`RenderLayers`] to this entity.
    ///
    /// This is cached so children of this entity can update themselves without needing to traverse the
    /// entire hierarchy.
    pub propagator: Entity,
    /// The [`RenderLayers`] computed by merging the [`RenderLayers`] of the `Self::propagator` entity into
    /// the node's [`RenderLayers`] component.
    ///
    /// This is cached for efficient access in the [`check_visibility`] system.
    pub computed: RenderLayers,
}

impl InheritedRenderLayers {
    /// Makes an empty `InheritedRenderLayers`.
    pub fn empty() -> Self {
        Self {
            propagator: Entity::PLACEHOLDER,
            computed: RenderLayers::empty(),
        }
    }
}

/// Contains the final [`RenderLayers`] of an entity for extraction to the render world.
#[derive(Component, Debug, Deref)]
pub struct ExtractedRenderLayers(RenderLayers);

/// Evaluates an entity's possible `RenderLayers` and `InheritedRenderLayers` components to get a
/// final [`ExtractedRenderLayers`] for the render world.
///
/// Potentially allocates if [`InheritedRenderLayers`] or [`RenderLayers`] is allocated.
pub fn extract_render_layers(
    inherited: Option<&InheritedRenderLayers>,
    render_layers: Option<&RenderLayers>,
) -> ExtractedRenderLayers {
    ExtractedRenderLayers(
        inherited
            .map(|i| &i.computed)
            .or(render_layers)
            .cloned()
            .unwrap_or(RenderLayers::default()),
    )
}

/// Evaluates a camera's possible `CameraLayer` component to get a
/// final [`ExtractedRenderLayers`] for the render world.
///
/// Potentially allocates if [`InheritedRenderLayers`] or [`CameraLayer`] is allocated.
pub fn extract_camera_layer(camera_layer: Option<&CameraLayer>) -> ExtractedRenderLayers {
    ExtractedRenderLayers(
        camera_layer
            .map(|i| i.get_layers())
            .unwrap_or(RenderLayers::default()),
    )
}

/// Derives a [`RenderLayersRef`] from an optional [`InheritedRenderLayers`] and [`RenderLayers`].
///
/// Returns in order of priority:
/// - [`InheritedRenderLayers::computed`]
/// - [`RenderLayers`]
/// - [`RenderLayers::default`]
pub fn derive_render_layers<'a>(
    inherited: Option<&'a InheritedRenderLayers>,
    render_layers: Option<&'a RenderLayers>,
) -> RenderLayersRef<'a> {
    if let Some(inherited) = inherited {
        RenderLayersRef::Ref(&inherited.computed)
    } else if let Some(render_layers) = render_layers {
        RenderLayersRef::Ref(render_layers)
    } else {
        RenderLayersRef::Val(RenderLayers::default())
    }
}

/// Derives a [`RenderLayersPtr`] from an optional [`InheritedRenderLayers`] and [`RenderLayers`].
///
/// See [`derive_render_layers`].
pub fn derive_render_layers_ptr(
    inherited: Option<&InheritedRenderLayers>,
    render_layers: Option<&RenderLayers>,
) -> RenderLayersPtr {
    if let Some(inherited) = inherited {
        RenderLayersPtr::Ptr(&inherited.computed)
    } else if let Some(render_layers) = render_layers {
        RenderLayersPtr::Ptr(render_layers)
    } else {
        RenderLayersPtr::Val(RenderLayers::default())
    }
}

#[derive(Resource, Default)]
struct PropagateRenderLayersEntityCache {
    /// Buffered to absorb spurious allocations of propagated values during traversals.
    saved: RenderLayers,
    /// Updated entities.
    ///
    /// This is cleared at the start of [`PropagateRenderLayersSet`].
    entities: EntityHashSet,
}

impl PropagateRenderLayersEntityCache {
    fn insert(&mut self, entity: Entity) {
        self.entities.insert(entity);
    }

    fn contains(&self, entity: Entity) -> bool {
        self.entities.contains(&entity)
    }

    fn clear(&mut self) {
        self.entities.clear();
    }

    fn saved(&mut self) -> &mut RenderLayers {
        &mut self.saved
    }
}

/// Removes `InheritedRenderLayers` from entities with `PropagateRenderLayers`.
fn clean_propagators(
    mut commands: Commands,
    dirty_propagators: Query<Entity, (With<InheritedRenderLayers>, With<PropagateRenderLayers>)>,
) {
    for dirty in dirty_propagators.iter() {
        if let Some(mut entity) = commands.get_entity(dirty) {
            entity.remove::<InheritedRenderLayers>();
        }
    }
}

/// Propagates propagation values that have changed.
//todo: Detect if the propagated value has actually changed? Hard to expect this would matter in practice.
#[allow(clippy::too_many_arguments)]
fn propagate_updated_propagators(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Detect added/changed: RenderLayers, CameraLayer, Camera (added only), PropagateRenderLayers.
    changed_propagators: Query<
        (
            Entity,
            &Children,
            Option<&RenderLayers>,
            Option<&CameraLayer>,
            Has<Camera>,
            &PropagateRenderLayers,
        ),
        Or<(
            Changed<RenderLayers>,
            Changed<CameraLayer>,
            Added<Camera>,
            Changed<PropagateRenderLayers>,
        )>,
    >,
    // Detect removed: RenderLayers, CameraLayer, Camera.
    mut removed_renderlayers: RemovedComponents<RenderLayers>,
    mut removed_cameralayer: RemovedComponents<CameraLayer>,
    mut removed_camera: RemovedComponents<Camera>,
    all_propagators: Query<(
        Entity,
        &Children,
        Option<&RenderLayers>,
        Option<&CameraLayer>,
        Has<Camera>,
        &PropagateRenderLayers,
    )>,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
) {
    // Track updated entities to prevent redundant updates, as `Commands` changes are deferred.
    //
    // Using this ensures that once mutations to entities with PropagateRenderLayers have been
    // propagated, all entities affected by those changes won't be mutated again. This makes it
    // safe to read parent InheritedRenderLayers (in the other cases that need to be handled) in
    // order to 'pull in' inherited changes when needed. See algorithm description for more details.
    updated_entities.clear();

    // Collect aggregate iterator for all propagators that need to propagate.
    let propagators = changed_propagators.iter().chain(
        // IMPORTANT: Removals should be ordered first if propagate_to_new_children is merged
        //            into changed_propagators.
        removed_renderlayers
            .read()
            .chain(removed_cameralayer.read())
            .chain(removed_camera.read())
            .filter_map(|e| all_propagators.get(e).ok()),
    );

    // Propagate each propagator.
    for (propagator, children, maybe_render_layers, maybe_camera_layer, maybe_camera, propagate) in
        propagators
    {
        // There can be duplicates due to component removals.
        if updated_entities.contains(propagator) {
            continue;
        }

        // Get value to propagate.
        let mut propagated = propagate.get_render_layers(
            updated_entities.saved(),
            maybe_render_layers,
            maybe_camera_layer,
            maybe_camera,
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
                &propagated,
                child,
            );
        }

        // Reclaim memory
        propagated.reclaim(updated_entities.saved());
    }
}

/// Applies propagation to entities for `apply_full_propagation`.
// Note: This does not require checking updated_entities because all children will be fresh.
fn apply_full_propagation(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderLayersEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
    propagator: Entity,
    propagated: &RenderLayers,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderLayers.
    let Ok((maybe_render_layers, maybe_inherited_layers)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Update inherited value or insert a new one.
    let empty_render_layers = RenderLayers::empty();
    let initial_layers = maybe_render_layers.unwrap_or(&empty_render_layers);
    let apply_changes = |layers: &mut InheritedRenderLayers| {
        layers.propagator = propagator;
        layers.computed.set_from(initial_layers);
        layers.computed.merge(propagated);
    };

    if let Some(mut inherited) = maybe_inherited_layers {
        apply_changes(&mut inherited);
    } else {
        let mut new = InheritedRenderLayers::empty();
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
            child,
        );
    }
}

/// Propagates propagation values to new children of a propagator.
//todo: Can be merged with apply_full_propagation at the cost of code density. Must make sure iterating
//      this comes *after* iterating removals, because this step must come *after* all propagation value changes
//      are handled.
fn propagate_to_new_children(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Changed children.
    changed_children: Query<
        (
            Entity,
            &Children,
            Option<&RenderLayers>,
            Option<&CameraLayer>,
            Has<Camera>,
            &PropagateRenderLayers,
        ),
        Changed<Children>,
    >,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
) {
    // Propagate each propagator that has new children.
    for (propagator, children, maybe_render_layers, maybe_camera_layer, maybe_camera, propagate) in
        changed_children.iter()
    {
        // The propagator could have been updated in a previous step.
        if updated_entities.contains(propagator) {
            continue;
        }

        // Get value to propagate.
        let mut propagated = propagate.get_render_layers(
            updated_entities.saved(),
            maybe_render_layers,
            maybe_camera_layer,
            maybe_camera,
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
                &propagated,
                child,
            );
        }

        // Reclaim memory
        propagated.reclaim(updated_entities.saved());
    }
}

/// Applies propagation to entities for `propagate_to_new_children`.
// Note: This does not require checking updated_entities because all children will be fresh.
fn apply_new_children_propagation(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderLayersEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
    propagator: Entity,
    propagated: &RenderLayers,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderLayers.
    let Ok((maybe_render_layers, maybe_inherited_layers)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Leave if the propagator is already known (implying this is a pre-existing child).
    if maybe_inherited_layers
        .as_ref()
        .map(|i| i.propagator == propagator)
        .unwrap_or(false)
    {
        return;
    }

    // Update inherited value or insert a new one.
    let empty_render_layers = RenderLayers::empty();
    let initial_layers = maybe_render_layers.unwrap_or(&empty_render_layers);
    let apply_changes = |layers: &mut InheritedRenderLayers| {
        layers.propagator = propagator;
        layers.computed.set_from(initial_layers);
        layers.computed.merge(propagated);
    };

    if let Some(mut inherited) = maybe_inherited_layers {
        apply_changes(&mut inherited);
    } else {
        let mut new = InheritedRenderLayers::empty();
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
            child,
        );
    }
}

/// Removes `InheritedRenderLayers` from orphaned branches of the hierarchy.
fn handle_orphaned_nonpropagators(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Orphaned non-propagator entities that previously had InheritedRenderLayers.
    mut removed_parents: RemovedComponents<Parent>,
    orphaned: Query<
        (Entity, Option<&Children>),
        (
            Without<Parent>,
            With<InheritedRenderLayers>,
            Without<PropagateRenderLayers>,
        ),
    >,
    // Query for getting non-propagator children.
    nonpropagators: Query<Option<&Children>, Without<PropagateRenderLayers>>,
) {
    for (orphan, maybe_children) in removed_parents.read().filter_map(|r| orphaned.get(r).ok()) {
        apply_orphan_cleanup(
            &mut commands,
            &mut updated_entities,
            &nonpropagators,
            orphan,
            maybe_children,
        );
    }
}

/// Applies propagation for `handle_orphaned_nonpropagators`.
fn apply_orphan_cleanup(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderLayersEntityCache,
    nonpropagators: &Query<Option<&Children>, Without<PropagateRenderLayers>>,
    entity: Entity,
    maybe_children: Option<&Children>,
) {
    // Remove InheritedRenderLayers.
    if let Some(mut entity) = commands.get_entity(entity) {
        entity.remove::<InheritedRenderLayers>();
    }

    // Mark as updated.
    updated_entities.insert(entity);

    // Update non-propagator children.
    let Some(children) = maybe_children else {
        return;
    };
    for child in children.iter().copied() {
        // Ignore children that have PropagateRenderLayers.
        let Ok(maybe_children) = nonpropagators.get(child) else {
            continue;
        };

        apply_orphan_cleanup(
            commands,
            updated_entities,
            nonpropagators,
            child,
            maybe_children,
        );
    }
}

/// Handles entities that lost the `PropagateRenderLayers` component.
fn handle_lost_propagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Entities that lost PropagateRenderLayers
    mut removed_propagate: RemovedComponents<PropagateRenderLayers>,
    unpropagated: Query<(Entity, Option<&Parent>), Without<PropagateRenderLayers>>,
    // Query for accessing propagators
    all_propagators: Query<(
        Entity,
        Option<&RenderLayers>,
        Option<&CameraLayer>,
        Has<Camera>,
        &PropagateRenderLayers,
    )>,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
) {
    for (entity, maybe_parent) in removed_propagate
        .read()
        .filter_map(|r| unpropagated.get(r).ok())
    {
        // Skip already-updated entities.
        if updated_entities.contains(entity) {
            continue;
        }

        // Apply propagation.

        // Case 1: no parent
        // - Policy: remove all

        // Case 2: parent is a non-propagator without InheritedRenderLayers (not marked updated)
        // - Policy: remove all

        // Case 3: parent is a non-propagator with InheritedRenderLayers (not marked updated)
        // - Subcase 1: Parent's propagator doesn't exit
        //   - Policy: remove all (note: this is not an error, because the propagation step to hendle it may not
        //     have executed yet)
        // - Subcase 2: Parent's propagator exists
        //   - Policy: Compute propagation value from parent's propagator

        // Case 4: parent is a non-propagator marked updated
        // - Subcase 1: Self doesn't have InheritedRenderLayers
        //   - Policy: remove all
        // - Subcase 2: Propagator stored in self's InheritedRenderLayers doesn't exist
        //   - Policy: remove all
        // - Subcase 3: Recalculate InheritedRenderLayers with self-stored propagator
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
            } else if updated_entities.contains(**parent) {
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
            maybe_render_layers,
            maybe_camera_layer,
            maybe_camera,
            propagate,
        )) = propagator.and_then(|p| all_propagators.get(p).ok())
        {
            // Propagation value
            let mut propagated = propagate.get_render_layers(
                updated_entities.saved(),
                maybe_render_layers,
                maybe_camera_layer,
                maybe_camera,
            );

            // Pre-update the entity as a hack for case 4-3. If we don't do this then
            // the entity will be caught in "Leave if entity is non-updated and inherits a matching propagator."
            // - Note: Case 4-3 is compensating for users manually inserting InheritedRenderLayers
            // components, so this could be simplified if that's deemed overkill (we don't fully compensate for
            // manual messing with InheritedRenderLayers anyway, so there is no real reliability for doing so).
            updated_entities.insert(entity);

            apply_full_propagation_force_update(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                &propagated,
                entity,
            );

            // Reclaim memory
            propagated.reclaim(updated_entities.saved());
        // In all other cases, remove all InheritedRenderLayers.
        } else {
            apply_full_propagation_force_remove(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &maybe_inherited,
                entity,
            );
        }
    }
}

/// Applies propagation to entities for `handle_lost_propagator` and `handle_new_children_nonpropagator`.
fn apply_full_propagation_force_update(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderLayersEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &mut Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
    propagator: Entity,
    propagated: &RenderLayers,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderLayers.
    let Ok((maybe_render_layers, maybe_inherited_layers)) = maybe_inherited.get_mut(entity) else {
        return;
    };

    // Leave if entity is non-updated and inherits a matching propagator.
    if let Some(inherited) = maybe_inherited_layers.as_ref() {
        if (inherited.propagator == propagator) && !updated_entities.contains(entity) {
            return;
        }
    }

    // Force-update
    let empty_render_layers = RenderLayers::empty();
    let initial_layers = maybe_render_layers.unwrap_or(&empty_render_layers);

    let mut new = InheritedRenderLayers::empty();
    if let Some(mut inherited) = maybe_inherited_layers {
        // Steal existing allocation if useful.
        if inherited.computed.is_allocated() {
            std::mem::swap(&mut inherited.computed, &mut new.computed);
        }
    }

    new.propagator = propagator;
    new.computed.set_from(initial_layers);
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
            child,
        );
    }
}

/// Applies `InheritedRenderLayers` removal to entities for `handle_lost_propagator`,
/// `handle_new_children_nonpropagator`, and `handle_new_parent_nonpropagator`.
fn apply_full_propagation_force_remove(
    commands: &mut Commands,
    updated_entities: &mut PropagateRenderLayersEntityCache,
    children_query: &Query<&Children>,
    maybe_inherited: &Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
    entity: Entity,
) {
    // Leave if entity doesn't exist or has PropagateRenderLayers.
    let Ok((_, maybe_inherited_inner)) = maybe_inherited.get(entity) else {
        return;
    };

    // Leave if entity is non-updated and doesn't have InheritedRenderLayers.
    if maybe_inherited_inner.is_none() && !updated_entities.contains(entity) {
        return;
    }

    // Force-remove InheritedRenderLayers
    commands.entity(entity).remove::<InheritedRenderLayers>();

    // Mark as updated.
    updated_entities.insert(entity);

    // Continue propagation to children.
    let Ok(children) = children_query.get(entity) else {
        return;
    };
    for child in children.iter().copied() {
        apply_full_propagation_force_remove(
            commands,
            updated_entities,
            children_query,
            maybe_inherited,
            child,
        );
    }
}

/// Handles non-propagator entities with `InheritedRenderLayers` whose children changed.
fn handle_new_children_nonpropagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Entities with InheritedRenderLayers that changed children
    inherited_with_children: Query<
        (Entity, &Children),
        (
            Changed<Children>,
            With<InheritedRenderLayers>,
            Without<PropagateRenderLayers>,
        ),
    >,
    // Query for accessing propagators
    all_propagators: Query<(
        Entity,
        Option<&RenderLayers>,
        Option<&CameraLayer>,
        Has<Camera>,
        &PropagateRenderLayers,
    )>,
    // Query for getting Children.
    children_query: Query<&Children>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    mut maybe_inherited: Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
) {
    for (entity, children) in inherited_with_children.iter() {
        // Skip entity if already updated, which implies children are already in an accurate state.
        if updated_entities.contains(entity) {
            continue;
        }

        // Get the inherited component. We need to do a lookup due to query conflict (Error B0001).
        let inherited_propagator = maybe_inherited.get(entity).unwrap().1.unwrap().propagator;

        let Ok((propagator, maybe_render_layers, maybe_camera_layer, maybe_camera, propagate)) =
            all_propagators.get(inherited_propagator)
        else {
            // Remove InheritedRenderLayers from descendents if the propagator is missing
            // - This is either an error caused by manually modifying InheritedRenderLayers, or is caused by a
            // reparenting + propagator despawn.

            // Iterate children
            for child in children.iter().copied() {
                // Skip children that were already updated.
                // - Note that this can happen e.g. because the child lost the PropagateRenderLayers component.
                if updated_entities.contains(child) {
                    continue;
                }

                // Propagate
                apply_full_propagation_force_remove(
                    &mut commands,
                    &mut updated_entities,
                    &children_query,
                    &maybe_inherited,
                    child,
                );
            }

            continue;
        };

        // Get value to propagate.
        let mut propagated = propagate.get_render_layers(
            updated_entities.saved(),
            maybe_render_layers,
            maybe_camera_layer,
            maybe_camera,
        );

        // Iterate children
        for child in children.iter().copied() {
            // Skip children that were already updated. We only skip updated children of this initial high-level
            // loop, not children within the recursion which need to be force-updated. The 'span' of entities
            // we update in this step starts at non-updated children of an entity with InheritedRenderLayers.
            // - Note that this can happen e.g. because the child lost the PropagateRenderLayers component.
            if updated_entities.contains(child) {
                continue;
            }

            // Propagate
            apply_full_propagation_force_update(
                &mut commands,
                &mut updated_entities,
                &children_query,
                &mut maybe_inherited,
                propagator,
                &propagated,
                child,
            );
        }

        // Reclaim memory
        propagated.reclaim(updated_entities.saved());
    }
}

/// Handles non-propagator entities with `InheritedRenderLayers` whose parents changed.
/// - Since `handle_new_children_nonpropagator` handles all cases where the parent has `InheritedRenderLayers`, this
/// system just needs to remove `InheritedRenderLayers` from non-updated entities and their non-updated descendents
/// that have `InheritedRenderLayers` (stopping at propagators and non-updated descendents without
/// `InheritedRenderLayers`).
/// - We skip non-updated entities whose parents are updated, because that implies the current `InheritedRenderLayers`
/// propagator is accurate.
fn handle_new_parent_nonpropagator(
    mut commands: Commands,
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Entities with InheritedRenderLayers that changed parents
    inherited_with_parent: Query<
        (Entity, Option<&Children>, &Parent),
        (
            Changed<Parent>,
            With<InheritedRenderLayers>,
            Without<PropagateRenderLayers>,
        ),
    >,
    // Query for Children
    children_query: Query<&Children>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    maybe_inherited: Query<
        (Option<&RenderLayers>, Option<&mut InheritedRenderLayers>),
        Without<PropagateRenderLayers>,
    >,
) {
    for (entity, maybe_children, parent) in inherited_with_parent.iter() {
        // Skip entity if already updated
        if updated_entities.contains(entity) {
            continue;
        }

        // Skip entity if parent was updated
        if updated_entities.contains(**parent) {
            continue;
        }

        // Remove from self.
        commands.entity(entity).remove::<InheritedRenderLayers>();

        // Mark as updated.
        updated_entities.insert(entity);

        // Iterate children.
        // - We assume the parent of this entity does NOT have InheritedRenderLayers, so neither should its
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
                child,
            );
        }
    }
}

/// Handles added/removed/changed `RenderLayers` for entities with existing `InheritedRenderLayers`.
fn handle_modified_renderlayers(
    mut updated_entities: ResMut<PropagateRenderLayersEntityCache>,
    // Entities with InheritedRenderLayers that changed RenderLayers
    inherited_changed: Query<
        Entity,
        (
            Changed<RenderLayers>,
            With<InheritedRenderLayers>,
            Without<PropagateRenderLayers>,
        ),
    >,
    // RenderLayers removals.
    mut removed_renderlayers: RemovedComponents<RenderLayers>,
    // Query for accessing propagators
    all_propagators: Query<(
        Entity,
        Option<&RenderLayers>,
        Option<&CameraLayer>,
        Has<Camera>,
        &PropagateRenderLayers,
    )>,
    // Query for updating InheritedRenderLayers on non-propagator entities.
    mut inherited: Query<
        (Option<&RenderLayers>, &mut InheritedRenderLayers),
        Without<PropagateRenderLayers>,
    >,
) {
    for entity in inherited_changed.iter().chain(removed_renderlayers.read()) {
        // Skip entity if already updated.
        if updated_entities.contains(entity) {
            continue;
        }

        // Skip entity if it's a propagator or doesn't exist.
        let Ok((entity_render_layers, mut inherited)) = inherited.get_mut(entity) else {
            continue;
        };

        // Skip entity if propagator is missing.
        // - This is an error, hierarchy steps should have marked this entity as updated.
        let Ok((_propagator, maybe_render_layers, maybe_camera_layer, maybe_camera, propagate)) =
            all_propagators.get(inherited.propagator)
        else {
            error_once!("hierarchy error: propagator missing for {entity} in `handle_modified_renderlayers`");
            continue;
        };

        // Get propagated value.
        let mut propagated = propagate.get_render_layers(
            updated_entities.saved(),
            maybe_render_layers,
            maybe_camera_layer,
            maybe_camera,
        );

        // Update entity value.
        inherited
            .computed
            .set_from(entity_render_layers.unwrap_or(&RenderLayers::empty()));
        inherited.computed.merge(&propagated);

        // Mark updated (in case of duplicates due to removals).
        updated_entities.insert(entity);

        // Reclaim memory
        propagated.reclaim(updated_entities.saved());
    }
}
