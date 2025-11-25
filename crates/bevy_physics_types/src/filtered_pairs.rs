//! Fine-grained collision pair filtering.
//!
//! The PhysicsFilteredPairsAPI describes fine-grained filtering. If a collision
//! between two objects occurs, this pair might be filtered if the pair is defined
//! through this API. This API can be applied either to a body, collision, or
//! articulation. Note that FilteredPairsAPI filtering has precedence over
//! CollisionGroup filtering.

use bevy_ecs::entity::EntityHashSet;

usd_attribute! {
    /// Set of entities that should be filtered from collision with this entity.
    FilteredPairs(EntityHashSet) = EntityHashSet::default();
    apiName = "filteredPairs"
    displayName = "Filtered Pairs"
}
