//! Collision group filtering for coarse-grained collision management.
//!
//! [`CollisionGroup`] defines collision groups for efficient broad-phase filtering.
//! When a collision occurs between two objects with groups assigned, they collide
//! unless the group pair is explicitly filtered.
//!
//! ## Basic Filtering
//!
//! Colliders not in any group collide with all other colliders in the scene
//! (except those with disabled collision by default). Groups define which
//! object categories should not interact.
//!
//! ## Inverted Filtering
//!
//! The [`InvertFilteredGroups`] marker inverts the filter behavior: instead of
//! disabling collisions with listed groups, it disables collisions against
//! **all other colliders except** those in the filtered groups.
//!
//! This is useful for configuring objects that should only interact with
//! specific categories (e.g., a trigger volume that only detects players).
//!
//! ## Group Merging
//!
//! When composing a stage from multiple USD files, collision groups may need to
//! be merged. The [`MergeGroupName`] attribute allows groups with matching names
//! to be combined:
//!
//! - Members of merged groups become members of the unified group
//! - Filter relationships are unioned
//!
//! This is useful when multiple character instances each have ragdoll groups
//! that should filter against each other's controllers.
//!
//! ## Merge Semantics with Inverted Groups
//!
//! **Caution**: Merging groups with different [`InvertFilteredGroups`] settings
//! can have unexpected results. Merging should only ever cause collision pairs
//! to become disabled—a filter cannot re-enable a pair disabled by another group.
//!
//! Example: An inverted group referencing only GroupX (collides only with GroupX)
//! merged with a non-inverting group referencing GroupX (doesn't collide with GroupX)
//! results in a group that collides with nothing.
//!
//! ## Precedence
//!
//! Group filtering is overridden by [`FilteredPairs`](crate::filtered_pairs::FilteredPairs)
//! for fine-grained pairwise exceptions.

use bevy_ecs::entity::EntityHashSet;

usd_collection! {
    /// Membership in a collision group.
    ///
    /// Entities with this component belong to the referenced [`CollisionGroup`].
    /// A collider can belong to multiple groups.
    /// Colliders without any group membership collide with everything (unless filtered by other means).
    CollisionGroupMember -> CollisionGroup(EntityHashSet);
    apiName = "PhysicsCollisionGroup"
}

usd_attribute! {
    /// Set of collision groups with which collisions should be ignored.
    ///
    /// By default, colliders in this group will not collide with colliders
    /// in any of the referenced groups. This behavior can be inverted with
    /// [`InvertFilteredGroups`].
    #[require(CollisionGroup)]
    FilteredGroups(EntityHashSet) = Default::default();
    apiName = "filteredGroups"
    displayName = "Filtered Groups"
}

usd_attribute! {
    /// Name used to merge collision groups across composed stages.
    ///
    /// All groups with a matching merge group name should be considered part
    /// of a single unified group. The members and filter relationships of all
    /// matched groups are combined.
    ///
    /// This enables patterns like having each character instance define its
    /// own ragdoll-vs-controller filter that automatically applies to all
    /// instances when the stage is composed.
    #[require(CollisionGroup)]
    MergeGroupName(String);
    apiName = "mergeGroupName"
    displayName = "Merge With Groups"
}

usd_marker! {
    /// Inverts the collision filter behavior.
    ///
    /// When enabled, this group disables collisions against **all colliders
    /// except** those in the [`FilteredGroups`]. Without this marker, the
    /// group disables collisions only against colliders in the filtered groups.
    ///
    /// Use this for objects that should only interact with specific categories
    /// rather than defining what they should ignore.
    #[require(CollisionGroup)]
    InvertFilteredGroups;
    apiName = "invertFilteredGroups"
    displayName = "Invert Filtered Groups"
}
