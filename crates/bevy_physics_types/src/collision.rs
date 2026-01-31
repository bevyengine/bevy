//! Collision shape properties for physics objects.
//!
//! The [`CollisionEnabled`] marker identifies an entity as a collision shape
//! (collider) for physics simulation. Collision shapes can be applied to
//! geometric primitives such as spheres, capsules, cubes, cylinders, cones,
//! and meshes.
//!
//! ## Rigid Body Association
//!
//! - **Dynamic colliders**: If any ancestor entity has [`RigidBody`](crate::rigid_body::RigidBody),
//!   the collider becomes part of that rigid body's collision representation.
//!   Multiple colliders can exist under a single rigid body to form compound shapes.
//!
//! - **Static colliders**: If there is no [`RigidBody`](crate::rigid_body::RigidBody)
//!   in the ancestor hierarchy, the collider is treated as static—it doesn't move
//!   but can collide with dynamic bodies. Static colliders are interpreted as having
//!   zero velocity and infinite mass.
//!
//! ## Subtree Behavior
//!
//! Geometric primitives are generally leaf entities. Since [`CollisionEnabled`]
//! can only be applied to geometry, there is no opportunity to inherit collision
//! attributes down the hierarchy. If a mesh is composed of submeshes, all
//! submeshes are considered part of the collider.
//!
//! ## Simulation Owner
//!
//! For static colliders not under a rigid body, the collision's own
//! [`SimulationOwner`](crate::scene::SimulationOwner) determines which physics
//! scene handles them. For colliders associated with a rigid body, the
//! collider's simulation owner is ignored—the body's owner takes precedence.
//!
//! ## Supported Shapes
//!
//! Supported collision primitives include:
//! - `Sphere`, `Capsule`, `Cube`, `Cylinder`, `Cone` (built-in primitives)
//! - `Mesh` (with [`ColliderFromMeshApproximation`](crate::mesh_collision::ColliderFromMeshApproximation))
//!
//! Some implementations may use faceted convex approximations for certain shapes.

make_marker! {
    /// Marks this entity as a collision shape for physics simulation.
    ///
    /// When present, this entity participates in collision detection. If any
    /// ancestor has [`RigidBody`](crate::rigid_body::RigidBody), this collider
    /// is considered part of that body. Otherwise, it is a static collider.
    ///
    /// This marker can also be applied to joints to enable collision between
    /// the connected bodies (by default, jointed bodies do not collide).
    CollisionEnabled;
    apiName = "collisionEnabled"
    displayName = "Collision Enabled"
}
