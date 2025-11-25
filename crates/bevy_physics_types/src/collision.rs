//! Collision properties for physics objects.

usd_marker! {
    /// If a simulation is running, this entity will collide with other entities
    /// that have CollisionEnabled applied. If any entity in the parent hierarchy has
    /// the RigidBody, the collider is considered part of the closest ancestor body.
    /// If there is no body in the parent hierarchy, this collider is considered static.
    /// Can also be applied to joints. Doing so allows collsion between the linked joints.
    CollisionEnabled;
    apiName = "collisionEnabled"
    displayName = "Collision Enabled"
}
