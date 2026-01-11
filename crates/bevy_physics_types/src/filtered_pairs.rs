//! Fine-grained pairwise collision filtering.
//!
//! The [`FilteredPairs`] component provides fine-grained collision filtering
//! between specific entity pairs, complementing the coarse-grained
//! [`CollisionGroup`](crate::collision_group::CollisionGroup) filtering.
//!
//! ## Use Case
//!
//! Group-based filtering sometimes cannot handle special cases. For example,
//! human character bodies might be set to collide with extremities (arms, legs)
//! assuming they belong to different characters. However, you typically don't
//! want a character's own extremities to collide with its own body during
//! close-proximity movement.
//!
//! Pairwise filtering solves this by explicitly disabling collisions between
//! specific entity pairs.
//!
//! ## Applicable Types
//!
//! [`FilteredPairs`] can be applied to:
//! - Entities with [`RigidBody`](crate::rigid_body::RigidBody)
//! - Entities with [`CollisionEnabled`](crate::collision::CollisionEnabled)
//! - Entities with [`ArticulationRoot`](crate::articulation::ArticulationRoot)
//!
//! ## Precedence
//!
//! Pairwise filtering has **higher precedence** than group-based filtering.
//! If a pair is filtered here, they won't collide even if their collision
//! groups would otherwise allow it.
//!
//! ## Relationship Direction
//!
//! The filtering relationship is **bidirectional by implication**. If entity A
//! has entity B in its [`FilteredPairs`], collisions between A and B are
//! disabled. Entity B does not need a reciprocal relationship to A.
//!
//! This simplifies authoring since you only need to specify the filter in one
//! direction.

use bevy_ecs::entity::EntityHashSet;

make_attribute! {
    /// Set of entities that should not collide with this entity.
    ///
    /// This provides fine-grained pairwise collision filtering. Any entity
    /// referenced here will not collide with this entity, regardless of
    /// collision group settings.
    ///
    /// The relationship is implicitly bidirectional: if A filters B, then
    /// B also won't collide with A, even if B doesn't explicitly filter A.
    ///
    /// This attribute can reference:
    /// - Rigid bodies (filters all their colliders)
    /// - Individual colliders
    /// - Articulation roots (filters all bodies in the articulation)
    FilteredPairs(EntityHashSet) = EntityHashSet::default();
    apiName = "filteredPairs"
    displayName = "Filtered Pairs"
}
