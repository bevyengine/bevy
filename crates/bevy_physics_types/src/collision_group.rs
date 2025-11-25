//! Collision group filtering for coarse-grained collision management.
//!
//! PhysicsCollisionGroup defines a collision group for coarse filtering.
//! When a collision occurs between two objects with a PhysicsCollisionGroup assigned,
//! they collide unless the group pair is filtered.
//! ECS mantain a list of PhysicsCollisionAPI relationships defining group members.

use bevy_ecs::entity::EntityHashSet;

usd_collection! {
    /// CollisionGroup defines a collision group for coarse filtering. When a
    /// collision occurs between two objects with a PhysicsCollisionGroup assigned,
    /// they collide unless the group pair is filtered. A CollectionAPI:colliders
    /// maintains a list of PhysicsCollisionAPI relationships defining group members.
    CollisionGroupMember -> CollisionGroup(EntityHashSet);
    apiName = "PhysicsCollisionGroup"

}

usd_attribute! {
    /// Collection of PhysicsCollisionGroups with which collisions should be ignored.
    #[require(CollisionGroup)]
    FilteredGroups(EntityHashSet) = Default::default();
    apiName = "filteredGroups"
    displayName = "Filtered Groups"
}

usd_attribute! {
    /// any collision groups with a matching MergeGroupName should be merged
    #[require(CollisionGroup)]
    MergeGroupName(String);
    apiName = "mergeGroupName"
    displayName = "Merge With Groups"
}

usd_marker! {
    /// When enabled, the filter will disable collisions against all colliders
    /// except for those in the selected filter groups.
    #[require(CollisionGroup)]
    InvertFilteredGroups;
    apiName = "invertFilteredGroups"
    displayName = "Invert Filtered Groups"
}
