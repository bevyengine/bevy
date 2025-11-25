//! Articulated rigid body hierarchies.
//!
//! PhysicsArticulationRootAPI can be applied to a scene graph node and marks
//! the subtree rooted here for inclusion in one or more reduced coordinate
//! articulations. For floating articulations, this should be on the root body.
//! For fixed articulations (e.g., a robot arm bolted to the floor), this API
//! can be on a direct or indirect parent of the root joint which is connected
//! to the world, or on the joint itself.
usd_marker! {
    /// Marks this subtree as an articulated rigid body hierarchy.
    /// Like the logical item.
    /// For a ragdoll, this would be the the root.
    ArticulationRoot;
    apiName = "articulationRootApi"
    displayName = "Articulation Root"
}
